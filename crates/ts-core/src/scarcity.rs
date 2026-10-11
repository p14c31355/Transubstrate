//! Research experiment: resource scarcity and genome selection pressure.
//!
//! Question (HYPOTHESIZED, defined operationally before running):
//! When field regeneration is too low to sustain the population, does
//! genome trait distribution shift across generations — i.e. do organisms
//! with higher absorb_affinity leave more descendants?
//!
//! Operational definitions:
//! - "selection pressure": regen_rate × FIELD_NORMALIZATION × cells <
//!   population × metabolism + absorption ceiling
//! - "trait drift": change in mean absorb_affinity between generation-0
//!   founders and generation-N descendants, beyond mutation noise
//!   (mutation is symmetric uniform, so unbiased drift has E[Δ]=0)
//! - OBSERVED adaptation claim requires: mean trait of survivors differs
//!   from founder mean in the direction of higher absorption, replicated
//!   across seeds, with the neutral-drift control (equal-absorb world)
//!   showing no such shift
//!
//! This module provides the experiment runner and honest metrics. It makes
//! no emergence claims; results are classified per-run.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::lineage::Genome;
use crate::primordial::{PrimordialConfig, PrimordialWorld};
use crate::rng::Rng;
use crate::wabix::Action;

/// Configuration for a scarcity-selection experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScarcityConfig {
    pub world_seed: u64,
    pub ticks: u64,
    pub founders: u32,
    pub start_energy: f32,
    /// Field regeneration rate (per cell per tick). Low values create
    /// scarcity.
    pub regen_rate: f32,
    /// Founder absorb_affinity (all founders identical — drift must come
    /// from mutation + selection, not initial variance).
    pub founder_absorb_affinity: f32,
    pub founder_mutation_rate: f32,
    /// Population cap for the run.
    pub population_cap: usize,
    /// Cognition interval: fixed policy (no backend variety) — pure physics
    /// + genome experiment.
    pub cognition_interval: u64,
}

impl Default for ScarcityConfig {
    fn default() -> Self {
        Self {
            world_seed: 42,
            ticks: 2000,
            founders: 12,
            start_energy: 25.0,
            // Scarcity: 0.001 × 10 × 256 cells = 2.56 units/tick regeneration
            // vs 12 organisms × 0.05 metabolism = 0.6/tick plus absorption —
            // the binding constraint is absorption competition.
            regen_rate: 0.001,
            founder_absorb_affinity: 1.0,
            founder_mutation_rate: 0.15,
            population_cap: 200,
            cognition_interval: 10,
        }
    }
}

/// Trait statistics for one generation cohort.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitStats {
    pub generation: u32,
    pub count: u32,
    pub mean_absorb_affinity: f32,
    pub mean_division_threshold: f32,
    pub mean_mutation_rate: f32,
}

/// Result of one scarcity run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScarcityResult {
    pub config: ScarcityConfig,
    pub ticks_executed: u64,
    pub total_births: u32,
    pub total_deaths: u32,
    pub final_alive: u32,
    pub max_generation: u32,
    /// Per-generation trait stats (cohorts by generation tag).
    pub generation_stats: Vec<TraitStats>,
    /// Founder mean absorb_affinity (all founders identical by design).
    pub founder_affinity: f32,
    /// Mean absorb_affinity of ALL living organisms at the end.
    pub final_mean_affinity: Option<f32>,
    /// Distinct lineage ids at the end.
    pub final_lineages: usize,
}

/// Fixed policy: ABSORB normally, propose DIVIDE when the normalized energy
/// observation is high (the genome's division threshold + world energy gate
/// the actual division). Selection pressure comes from the world, not from
/// controller variety.
fn policy_action(local_energy_obs: f32) -> Action {
    if local_energy_obs >= 0.5 {
        // ~5.0 raw energy: probe division; the genome threshold (30) and
        // world energy check decide whether it actually happens.
        Action {
            name: "DIVIDE".into(),
            params: vec![],
        }
    } else {
        absorb_action()
    }
}

/// Observation lookup helper: the local_energy channel value for a subject.
fn local_energy_observation(world: &PrimordialWorld, subject_id: &str) -> f32 {
    world
        .organism(subject_id)
        .map(|o| (o.energy / 10.0).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

/// Simple fixed policy: always ABSORB (maximum extraction — the selection
/// pressure comes from the world, not from controller variety).
fn absorb_action() -> Action {
    Action {
        name: "ABSORB".into(),
        params: vec![1.0],
    }
}

/// Run one scarcity experiment. Deterministic given the seed.
///
/// NOTE: the world's absorb path must consult each organism's genome
/// absorb_affinity for selection to be possible; this requires the genome
/// modifier to be applied in PrimordialWorld::apply_action (see PR notes).
pub fn run_scarcity(config: &ScarcityConfig, _log_path: Option<&Path>) -> ScarcityResult {
    let mut world = PrimordialWorld::new(
        PrimordialConfig {
            regen_rate: config.regen_rate,
            ..Default::default()
        },
        config.world_seed,
    );
    world.population_cap = config.population_cap;

    // Spawn founders with identical genomes except per-founder lineage ids.
    let mut rng = Rng::new(config.world_seed ^ 0x5eed);
    for i in 0..config.founders {
        let genome = Genome {
            absorb_affinity: config.founder_absorb_affinity,
            mutation_rate: config.founder_mutation_rate,
            // Division threshold BELOW start energy so division is reachable
            // immediately; scarcity (not the threshold) is the binding
            // constraint. World still gates on energy >= threshold + cost.
            division_threshold: 15.0,
            ..Default::default()
        };
        let x = rng.next_range(world.config.width as u64) as u32;
        let y = rng.next_range(world.config.height as u64) as u32;
        world.spawn_with_genome_at(
            &format!("founder-{i:03}"),
            (x, y),
            config.start_energy,
            genome,
            format!("lineage-{i:03}"),
            0,
        );
    }

    let mut births = 0u32;
    let mut deaths = 0u32;
    let mut last_events_alive: u32 = config.founders;

    for _tick in 0..config.ticks {
        // Fixed policy with genome-gated division probing.
        let subjects: Vec<String> = world
            .all_organisms()
            .filter(|o| o.alive)
            .map(|o| o.subject_id.clone())
            .collect();
        let actions: Vec<(String, Action)> = subjects
            .iter()
            .map(|s| {
                let obs = local_energy_observation(&world, s);
                (s.clone(), policy_action(obs))
            })
            .collect();

        let events = world.tick(&actions);
        for e in &events {
            match e {
                crate::wabix::WorldEvent::ActionApplied { action, .. }
                    if action.starts_with("DIVIDE->") =>
                {
                    births += 1;
                }
                crate::wabix::WorldEvent::SubjectDied { .. } => deaths += 1,
                _ => {}
            }
        }

        if world.alive_count() == 0 {
            break;
        }
        last_events_alive = world.alive_count() as u32;
    }
    let _ = last_events_alive;

    // Collect generation cohorts.
    let mut cohorts: std::collections::BTreeMap<u32, Vec<&crate::primordial::Organism>> =
        std::collections::BTreeMap::new();
    for org in world.all_organisms() {
        cohorts.entry(org.generation).or_default().push(org);
    }
    let generation_stats: Vec<TraitStats> = cohorts
        .iter()
        .map(|(generation_tag, orgs)| TraitStats {
            generation: *generation_tag,
            count: orgs.len() as u32,
            mean_absorb_affinity: orgs.iter().map(|o| o.genome.absorb_affinity).sum::<f32>()
                / orgs.len() as f32,
            mean_division_threshold: orgs
                .iter()
                .map(|o| o.genome.division_threshold)
                .sum::<f32>()
                / orgs.len() as f32,
            mean_mutation_rate: orgs.iter().map(|o| o.genome.mutation_rate).sum::<f32>()
                / orgs.len() as f32,
        })
        .collect();

    let alive_orgs: Vec<_> = world.all_organisms().filter(|o| o.alive).collect();
    let final_mean_affinity = if alive_orgs.is_empty() {
        None
    } else {
        Some(
            alive_orgs
                .iter()
                .map(|o| o.genome.absorb_affinity)
                .sum::<f32>()
                / alive_orgs.len() as f32,
        )
    };
    let final_lineages = alive_orgs
        .iter()
        .map(|o| o.lineage_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();

    ScarcityResult {
        config: config.clone(),
        ticks_executed: world.tick,
        total_births: births,
        total_deaths: deaths,
        final_alive: world.alive_count() as u32,
        max_generation: world.max_generation(),
        generation_stats,
        founder_affinity: config.founder_absorb_affinity,
        final_mean_affinity,
        final_lineages,
    }
}

impl PrimordialWorld {
    fn max_generation(&self) -> u32 {
        self.all_organisms()
            .map(|o| o.generation)
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scarcity_run_is_deterministic() {
        let a = run_scarcity(&ScarcityConfig::default(), None);
        let b = run_scarcity(&ScarcityConfig::default(), None);
        assert_eq!(a.total_births, b.total_births);
        assert_eq!(a.final_alive, b.final_alive);
        assert_eq!(a.generation_stats, b.generation_stats);
    }

    #[test]
    fn scarcity_produces_generations() {
        let r = run_scarcity(&ScarcityConfig::default(), None);
        assert!(
            r.max_generation >= 1,
            "scarcity run must produce offspring; got max gen {} with {} births",
            r.max_generation,
            r.total_births
        );
    }

    #[test]
    fn extinction_under_extreme_scarcity() {
        let cfg = ScarcityConfig {
            regen_rate: 0.0, // literally no regeneration
            ticks: 3000,
            ..Default::default()
        };
        let r = run_scarcity(&cfg, None);
        // With zero regen the field drains to 0 and everything dies.
        assert_eq!(r.final_alive, 0, "zero regen must eventually extinguish");
        assert!(r.total_deaths > 0);
    }

    #[test]
    fn stats_are_coherent() {
        let r = run_scarcity(&ScarcityConfig::default(), None);
        for gs in &r.generation_stats {
            assert!(gs.count > 0);
            // absorb_affinity clamp range is [0.5, 1.5]
            assert!((0.5..=1.5).contains(&gs.mean_absorb_affinity));
        }
    }
}
