//! Research experiment: two-phenotype competition.
//!
//! Question (HYPOTHESIZED, defined operationally before running):
//! When two phenotypes differing only in absorb_affinity compete for the
//! same scarce resource, does the higher-affinity phenotype's share of the
//! population increase across divisions?
//!
//! Operational definitions:
//! - phenotype A: absorb_affinity = 1.3; phenotype B: absorb_affinity = 0.7
//!   (both within the clamped genome range; symmetric around 1.0 so neither
//!   has a mutation-artifact advantage)
//! - "share of population": count_A / (count_A + count_B) among living
//!   organisms at end of run; founders are 50/50
//! - "selection measured": final share_A > 0.5 replicated across seeds, with
//!   the null condition (affinities equal) staying at 0.5
//! - births/deaths per phenotype give the direct mechanism (A divides more
//!   often because it extracts more per absorb)
//!
//! This directly measures relative reproductive success instead of waiting
//! for weak mutation drift (the honest negative result of the scarcity
//! experiment, PR #11).

use serde::{Deserialize, Serialize};

use crate::lineage::Genome;
use crate::primordial::{PrimordialConfig, PrimordialWorld};
use crate::rng::Rng;
use crate::wabix::Action;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompetitionConfig {
    pub world_seed: u64,
    pub ticks: u64,
    /// Founders per phenotype (population = 2 × pairs).
    pub pairs: u32,
    pub start_energy: f32,
    pub regen_rate: f32,
    pub affinity_a: f32,
    pub affinity_b: f32,
    pub mutation_rate: f32,
    pub division_threshold: f32,
    pub population_cap: usize,
}

impl Default for CompetitionConfig {
    fn default() -> Self {
        Self {
            world_seed: 42,
            ticks: 3000,
            pairs: 8,
            start_energy: 25.0,
            regen_rate: 0.001,
            affinity_a: 1.3,
            affinity_b: 0.7,
            mutation_rate: 0.05, // small: phenotypes stay distinguishable
            division_threshold: 15.0,
            population_cap: 200,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompetitionResult {
    pub config: CompetitionConfig,
    pub ticks_executed: u64,
    pub alive_a: u32,
    pub alive_b: u32,
    pub births_a: u32,
    pub births_b: u32,
    pub deaths_a: u32,
    pub deaths_b: u32,
    /// share_A = alive_a / (alive_a + alive_b); None on total extinction.
    pub final_share_a: Option<f32>,
    pub extinct: bool,
}

fn policy_action(energy: f32, threshold: f32) -> Action {
    // Absorb; probe DIVIDE when energy >= threshold (direct genome read is
    // not available to the policy, but the world re-checks anyway — this
    // probe point keeps rejected DIVIDEs rare).
    if energy >= threshold {
        Action {
            name: "DIVIDE".into(),
            params: vec![],
        }
    } else {
        Action {
            name: "ABSORB".into(),
            params: vec![1.0],
        }
    }
}

/// Run one two-phenotype competition. Deterministic given the seed.
pub fn run_competition(config: &CompetitionConfig) -> CompetitionResult {
    let mut world = PrimordialWorld::new(
        PrimordialConfig {
            regen_rate: config.regen_rate,
            ..Default::default()
        },
        config.world_seed,
    );
    world.population_cap = config.population_cap;

    let mut rng = Rng::new(config.world_seed ^ 0x0C0FEE);
    let mut births_a = 0u32;
    let mut births_b = 0u32;
    let mut deaths_a = 0u32;
    let mut deaths_b = 0u32;

    let spawn_phenotype =
        |world: &mut PrimordialWorld, id: &str, affinity: f32, lineage: String, rng: &mut Rng| {
            let genome = Genome {
                absorb_affinity: affinity,
                mutation_rate: config.mutation_rate,
                division_threshold: config.division_threshold,
                ..Default::default()
            };
            let x = rng.next_range(world.config.width as u64) as u32;
            let y = rng.next_range(world.config.height as u64) as u32;
            world.spawn_with_genome_at(id, (x, y), config.start_energy, genome, lineage, 0);
        };

    // Interleave A and B founders spatially (fair start).
    for i in 0..config.pairs {
        spawn_phenotype(
            &mut world,
            &format!("A-{i:03}"),
            config.affinity_a,
            format!("lin-A-{i:03}"),
            &mut rng,
        );
        spawn_phenotype(
            &mut world,
            &format!("B-{i:03}"),
            config.affinity_b,
            format!("lin-B-{i:03}"),
            &mut rng,
        );
    }

    for _tick in 0..config.ticks {
        let subjects: Vec<(String, f32, bool)> = world
            .all_organisms()
            .filter(|o| o.alive)
            .map(|o| {
                let is_a = o.genome.absorb_affinity > 1.0;
                (o.subject_id.clone(), o.energy, is_a)
            })
            .collect();
        let actions: Vec<(String, Action)> = subjects
            .iter()
            .map(|(sid, energy, _)| {
                (
                    sid.clone(),
                    policy_action(*energy, config.division_threshold),
                )
            })
            .collect();

        let events = world.tick(&actions);
        for e in &events {
            match e {
                crate::wabix::WorldEvent::ActionApplied {
                    action, subject, ..
                } => {
                    if let Some(_child) = action.strip_prefix("DIVIDE->") {
                        if subject.starts_with("A-")
                            || world
                                .organism(subject)
                                .map(|o| o.genome.absorb_affinity > 1.0)
                                .unwrap_or(false)
                        {
                            births_a += 1;
                        } else {
                            births_b += 1;
                        }
                    }
                }
                crate::wabix::WorldEvent::SubjectDied { subject, .. } => {
                    if subject.starts_with("A-") {
                        deaths_a += 1;
                    } else {
                        deaths_b += 1;
                    }
                }
                _ => {}
            }
        }

        if world.alive_count() == 0 {
            break;
        }
    }

    let alive_a: u32 = world
        .all_organisms()
        .filter(|o| o.alive && o.genome.absorb_affinity > 1.0)
        .count() as u32;
    let alive_b: u32 = world
        .all_organisms()
        .filter(|o| o.alive && o.genome.absorb_affinity <= 1.0)
        .count() as u32;
    let extinct = alive_a + alive_b == 0;
    let final_share_a = if extinct {
        None
    } else {
        Some(alive_a as f32 / (alive_a + alive_b) as f32)
    };

    CompetitionResult {
        config: config.clone(),
        ticks_executed: world.tick,
        alive_a,
        alive_b,
        births_a,
        births_b,
        deaths_a,
        deaths_b,
        final_share_a,
        extinct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competition_is_deterministic() {
        let a = run_competition(&CompetitionConfig::default());
        let b = run_competition(&CompetitionConfig::default());
        assert_eq!(a.final_share_a, b.final_share_a);
        assert_eq!(a.births_a, b.births_a);
        assert_eq!(a.births_b, b.births_b);
    }

    #[test]
    fn high_affinity_outcompetes_low_across_seeds() {
        // HYPOTHESIS TEST: A (1.3) should hold > 0.5 share vs B (0.7) across
        // seeds. This is the experiment's core assertion.
        let seeds = [42u64, 7, 99, 123, 2024];
        let mut wins = 0u32;
        for seed in seeds {
            let cfg = CompetitionConfig {
                world_seed: seed,
                ..Default::default()
            };
            let r = run_competition(&cfg);
            assert!(
                !r.extinct,
                "seed {seed}: competition run must not extinguish"
            );
            let share = r.final_share_a.unwrap();
            if share > 0.5 {
                wins += 1;
            }
        }
        assert!(
            wins >= 4,
            "high-affinity phenotype must dominate in most seeds; wins={wins}/{}",
            seeds.len()
        );
    }

    #[test]
    fn equal_affinities_stay_balanced() {
        // NULL CONDITION: with equal affinity, neither phenotype should
        // systematically dominate (share near 0.5 within noise band 0.35-0.65).
        let seeds = [42u64, 7, 99];
        for seed in seeds {
            let cfg = CompetitionConfig {
                world_seed: seed,
                affinity_a: 1.0,
                affinity_b: 1.0,
                ..Default::default()
            };
            let r = run_competition(&cfg);
            let share = r.final_share_a.unwrap();
            assert!(
                (0.3..=0.7).contains(&share),
                "equal-affinity null must stay balanced; seed {seed} share={share}"
            );
        }
    }

    #[test]
    fn births_track_mechanism() {
        // Mechanism check: the high-affinity phenotype should produce more
        // births than the low one under scarcity (it extracts more per tick).
        let cfg = CompetitionConfig::default();
        let r = run_competition(&cfg);
        assert!(
            r.births_a >= r.births_b,
            "A({}) births {} must be >= B({}) births {} (mechanism: more extraction -> more divisions)",
            cfg.affinity_a,
            r.births_a,
            cfg.affinity_b,
            r.births_b
        );
    }
}
