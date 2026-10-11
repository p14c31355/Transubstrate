//! Research experiment: multi-subject signal-follower detection in the
//! network world.
//!
//! Question (HYPOTHESIZED, operationally defined before running):
//! With signal-driven energy pull active (signal_pull_rate > 0), do OTHER
//! subjects flock toward or harvest at another subject's signal — i.e. does
//! a signal broadcast by one subject become a resource cue for others?
//!
//! Operational definitions (fixed before running):
//! - "signal subject": one designated emitter that SIGNALs (strength 1.0)
//!   on its node every cognition tick; it does not ABSORB.
//! - "follower subject": N−1 independent subjects with a fixed
//!   energy-seeking policy that HOPs toward the strongest nearby signal
//!   (ring direction with the higher neighbor signal observation) and
//!   ABSORBs otherwise. The policy is written against channel names by
//!   design — it is a rule-based controller, the same class as
//!   signal-seeker, so any flocking is a world-mechanics result, not a
//!   learned convention. What the experiment measures is whether the
//!   PULL MECHANISM creates the follower payoff to detect at all.
//! - "attraction measured": across seeds, followers spend a larger
//!   fraction of ticks on or adjacent to the emitter's node than a null
//!   run with signal_pull_rate = 0 (same seeds, same policies).
//! - "harvest measured": follower energy intake (absorbed units) per tick
//!   is higher in the signal run than in the null run.
//!
//! This measures whether the mechanism creates a candidate signaling
//! convention substrate (emitter → cue → followers). It does NOT claim an
//! emergent convention: the followers' policy is engineered.

use serde::{Deserialize, Serialize};

use crate::network_world::{NetworkConfig, NetworkWorld};
use crate::wabix::{Action, World};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignalingConfig {
    pub world_seed: u64,
    pub ticks: u64,
    pub followers: u32,
    pub start_energy: f32,
    /// Signal strength the emitter uses every cognition tick.
    pub emitter_strength: f32,
    /// The emitter signals every N ticks.
    pub emitter_interval: u64,
    /// 0.0 disables the pull (null run).
    pub signal_pull_rate: f32,
}

impl Default for SignalingConfig {
    fn default() -> Self {
        Self {
            world_seed: 42,
            ticks: 600,
            followers: 6,
            start_energy: 12.0,
            emitter_strength: 1.0,
            emitter_interval: 3,
            signal_pull_rate: 0.5,
        }
    }
}

impl SignalingConfig {
    /// Survival-tuned variant: enough start energy for a follower to sweep
    /// the whole ring (metabolism 0.05/tick over 600 ticks = 30 units, plus
    /// HOP costs) before it must live off the emitter's pulled-in energy.
    /// The default energy (12) covers only ~240 ticks — the disclosed
    /// negative result in docs/experiments/signaling-followers.md.
    pub fn survival_tuned(world_seed: u64, ticks: u64, followers: u32) -> Self {
        // Energy needed to survive the sweep: ticks * metabolism + hop headroom.
        let metabolism = NetworkConfig::default().metabolism;
        let sweep_budget = ticks as f32 * metabolism + 10.0;
        Self {
            world_seed,
            ticks,
            followers,
            start_energy: sweep_budget,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignalingResult {
    pub config: SignalingConfig,
    pub ticks_executed: u64,
    /// Mean fraction of followers on or adjacent to the emitter's node.
    pub mean_proximity: f32,
    /// Mean follower energy intake (absorbed units) per tick.
    pub mean_intake_per_tick: f32,
    /// Followers alive at the end.
    pub followers_alive: u32,
    /// Emitter alive at the end.
    pub emitter_alive: bool,
}

/// Follower policy: ABSORB when the current node is signaling (pull
/// concentrates energy there); otherwise HOP forward with spare energy to
/// sweep the ring for a signal; WAIT otherwise. Uses only the World
/// trait's observe() output.
fn follower_action(world: &mut NetworkWorld, subject_id: &str, energy: f32) -> Action {
    let Some(subject) = world.subject(subject_id) else {
        return Action {
            name: "WAIT".into(),
            params: vec![],
        };
    };
    let sref = crate::wabix::SubjectRef {
        subject_id: subject_id.to_string(),
        pos: (subject.node, 0),
        energy,
    };
    let observations = match world.observe(&sref) {
        Ok(o) => o,
        Err(_) => {
            return Action {
                name: "WAIT".into(),
                params: vec![],
            };
        }
    };
    let here = observations
        .iter()
        .find(|o| o.channel == "nearby_signal_0")
        .map(|o| o.value)
        .unwrap_or(0.0);
    if here > 0.02 {
        return Action {
            name: "ABSORB".into(),
            params: vec![1.0],
        };
    }
    if energy > 0.5 {
        return Action {
            name: "HOP".into(),
            params: vec![0.9],
        };
    }
    Action {
        name: "WAIT".into(),
        params: vec![],
    }
}

fn proximity(world: &NetworkWorld, emitter_node: u32, follower_nodes: &[u32]) -> f32 {
    let n = world.config.nodes;
    if follower_nodes.is_empty() {
        return 0.0;
    }
    let near = follower_nodes
        .iter()
        .filter(|&&f| {
            let d = (f as i64 - emitter_node as i64).rem_euclid(n as i64);
            let d = d.min(n as i64 - d);
            d <= 1
        })
        .count();
    near as f32 / follower_nodes.len() as f32
}

/// Run one signaling experiment (signal_pull_rate from the config; the null
/// run passes 0.0). Deterministic given the seed.
pub fn run_signaling(config: &SignalingConfig) -> SignalingResult {
    let net = NetworkConfig {
        signal_pull_rate: config.signal_pull_rate,
        ..Default::default()
    };
    let mut world = NetworkWorld::new(net, config.world_seed);
    let emitter_node = world.spawn("emitter", config.start_energy * 4.0);
    let mut follower_nodes: Vec<u32> = Vec::new();
    for i in 0..config.followers {
        let node = world.spawn(&format!("follower-{i:02}"), config.start_energy);
        follower_nodes.push(node);
    }

    let mut intake_ticks = 0.0f32;
    let mut proximity_sum = 0.0f32;
    let mut proximity_samples = 0.0f32;
    let mut absorbed_total = 0.0f32;

    for tick in 0..config.ticks {
        let mut actions: Vec<(String, Action)> = Vec::new();
        if tick.is_multiple_of(config.emitter_interval) {
            actions.push((
                "emitter".to_string(),
                Action {
                    name: "SIGNAL".into(),
                    params: vec![config.emitter_strength],
                },
            ));
        }
        let current_nodes: Vec<u32> = (0..config.followers)
            .filter_map(|i| world.subject(&format!("follower-{i:02}")).map(|s| s.node))
            .collect();
        proximity_sum += proximity(&world, emitter_node, &current_nodes);
        proximity_samples += 1.0;

        for i in 0..config.followers {
            let sid = format!("follower-{i:02}");
            let energy = world.subject(&sid).map(|s| s.energy);
            if let Some(energy) = energy {
                actions.push((sid.clone(), follower_action(&mut world, &sid, energy)));
            }
        }
        let energy_before: f32 = (0..config.followers)
            .filter_map(|i| world.subject(&format!("follower-{i:02}")).map(|s| s.energy))
            .sum();
        let _events = world.tick(&actions);
        let energy_after: f32 = (0..config.followers)
            .filter_map(|i| world.subject(&format!("follower-{i:02}")).map(|s| s.energy))
            .sum();
        // Absorbed units = net energy change + metabolism + hop costs paid.
        let hops = actions
            .iter()
            .filter(|(sid, a)| sid.starts_with("follower-") && a.name == "HOP")
            .count() as f32;
        absorbed_total += (energy_after - energy_before)
            + NetworkConfig::default().metabolism * world.alive_count().max(1) as f32
            + hops * 0.1;
        intake_ticks += 1.0;
    }

    let followers_alive = (0..config.followers)
        .filter(|i| world.subject(&format!("follower-{i:02}")).is_some())
        .count() as u32;
    let emitter_alive = world.subject("emitter").is_some();

    SignalingResult {
        config: config.clone(),
        ticks_executed: world.tick,
        mean_proximity: if proximity_samples > 0.0 {
            proximity_sum / proximity_samples
        } else {
            0.0
        },
        mean_intake_per_tick: if intake_ticks > 0.0 {
            absorbed_total / intake_ticks
        } else {
            0.0
        },
        followers_alive,
        emitter_alive,
    }
}

/// Convenience: the attraction contrast (signal run vs null run) for one
/// seed. Returns (signal_proximity, null_proximity, signal_intake,
/// null_intake).
pub fn run_contrast(config: &SignalingConfig) -> (f32, f32, f32, f32) {
    let signal = run_signaling(config);
    let null = run_signaling(&SignalingConfig {
        signal_pull_rate: 0.0,
        ..config.clone()
    });
    (
        signal.mean_proximity,
        null.mean_proximity,
        signal.mean_intake_per_tick,
        null.mean_intake_per_tick,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signaling_is_deterministic() {
        let a = run_signaling(&SignalingConfig::default());
        let b = run_signaling(&SignalingConfig::default());
        assert_eq!(a, b);
    }

    #[test]
    fn followers_attracted_to_signal_across_seeds() {
        // HYPOTHESIS TEST: follower proximity to the emitter is higher with
        // signal pull than in the null run, across seeds.
        let seeds = [42u64, 7, 99, 123, 2024];
        let mut wins = 0u32;
        let mut total_signal = 0.0f32;
        let mut total_null = 0.0f32;
        for seed in seeds {
            let cfg = SignalingConfig {
                world_seed: seed,
                ..Default::default()
            };
            let (sp, np, _, _) = run_contrast(&cfg);
            total_signal += sp;
            total_null += np;
            if sp > np {
                wins += 1;
            }
        }
        assert!(
            total_signal > total_null,
            "aggregate proximity must exceed the null run: {total_signal} vs {total_null}"
        );
        assert!(wins >= 3, "attraction must win most seeds: {wins}/5");
    }

    #[test]
    fn followers_harvest_more_with_signal_pull() {
        let seeds = [42u64, 7, 99, 123, 2024];
        let mut wins = 0u32;
        for seed in seeds {
            let cfg = SignalingConfig {
                world_seed: seed,
                ..Default::default()
            };
            let (_, _, si, ni) = run_contrast(&cfg);
            if si > ni {
                wins += 1;
            }
        }
        assert!(wins >= 3, "intake advantage must win most seeds: {wins}/5");
    }

    #[test]
    fn survival_tuned_followers_outlive_the_sweep() {
        // The default-energy run kills every follower within 600 ticks
        // (disclosed negative result). With a sweep-sized energy budget the
        // followers must survive to reach the emitter and live off its
        // pulled-in energy — the attraction payoff becomes livable.
        let seeds = [42u64, 7, 99, 123, 2024];
        for seed in seeds {
            let cfg = SignalingConfig::survival_tuned(seed, 600, 6);
            let r = run_signaling(&cfg);
            assert!(
                r.followers_alive > 0,
                "seed {seed}: survival-tuned followers must outlive the run (alive={})",
                r.followers_alive
            );
        }
    }

    #[test]
    fn survival_tuned_attraction_persists() {
        // Survival tuning must not erase the attraction effect: proximity
        // in the signal run should still beat the null run on aggregate.
        let seeds = [42u64, 7, 99, 123, 2024];
        let mut total_signal = 0.0f32;
        let mut total_null = 0.0f32;
        for seed in seeds {
            let cfg = SignalingConfig::survival_tuned(seed, 600, 6);
            let (sp, np, _, _) = run_contrast(&cfg);
            total_signal += sp;
            total_null += np;
        }
        assert!(
            total_signal > total_null,
            "survival-tuned attraction must exceed null: {total_signal} vs {total_null}"
        );
    }
}
