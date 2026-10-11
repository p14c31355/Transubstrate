//! Transubstrate CLI: run experiments, inspect summaries.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use ts_core::event_log::EventPayload;
use ts_core::experiment::{BackendKind, ExperimentConfig, run};
use ts_core::ids::ExperimentId;

#[derive(Parser)]
#[command(
    name = "transubstrate",
    version,
    about = "Persistent artificial subjects: experiments CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a vertical-slice experiment in the deterministic test world.
    RunTest {
        /// Number of world ticks to execute.
        #[arg(long, default_value_t = 100)]
        ticks: u64,
        /// Cognition fires every N ticks.
        #[arg(long, default_value_t = 4)]
        cognition_interval: u64,
        /// Cognition backend: echo | random | energy-rule | malformed
        #[arg(long, default_value = "echo")]
        backend: String,
        /// Seed for seeded backends and the world.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Maintenance energy cost per tick.
        #[arg(long, default_value_t = 0.01)]
        maintenance: f32,
        /// Starting energy.
        #[arg(long, default_value_t = 1000.0)]
        energy: f32,
        /// Output directory for the event log (default: runs/).
        #[arg(long, default_value = "runs")]
        out: PathBuf,
    },
    /// Run the primordial ALife experiment (M3).
    RunPrimordial {
        #[arg(long, default_value_t = 1000)]
        ticks: u64,
        #[arg(long, default_value_t = 10)]
        cognition_interval: u64,
        #[arg(long, default_value_t = 8)]
        population: u32,
        /// Cognition backend: echo | random | energy-rule
        #[arg(long, default_value = "energy-rule")]
        backend: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Starting energy per organism.
        #[arg(long, default_value_t = 10.0)]
        energy: f32,
        #[arg(long, default_value = "runs")]
        out: PathBuf,
    },
    /// Run the controller-migration continuity experiment (M5).
    RunMigration {
        #[arg(long, default_value_t = 400)]
        ticks: u64,
        #[arg(long, default_value_t = 5)]
        cognition_interval: u64,
        /// Backend before migration: echo | random | energy-rule
        #[arg(long, default_value = "echo")]
        backend_a: String,
        /// Backend after migration: echo | random | energy-rule
        #[arg(long, default_value = "energy-rule")]
        backend_b: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 30.0)]
        energy: f32,
        #[arg(long, default_value = "runs")]
        out: PathBuf,
    },
    /// Run the scarcity/selection experiment (research).
    RunScarcity {
        #[arg(long, default_value_t = 2000)]
        ticks: u64,
        #[arg(long, default_value_t = 12)]
        founders: u32,
        #[arg(long, default_value_t = 0.001)]
        regen: f32,
        /// Comma-separated seeds for multi-seed statistics.
        #[arg(long, default_value = "42,7,99,123,2024")]
        seeds: String,
        #[arg(long, default_value = "runs")]
        out: PathBuf,
    },
    /// Run the two-phenotype competition experiment (research).
    RunCompetition {
        #[arg(long, default_value_t = 3000)]
        ticks: u64,
        #[arg(long, default_value_t = 8)]
        pairs: u32,
        #[arg(long, default_value = "42,7,99,123,2024")]
        seeds: String,
    },
    /// Run the reference-8gb benchmark harness (M10).
    Benchmark {
        /// Output path for the JSON report (default: stdout only).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Run the world-migration experiment: primordial → network (M9).
    RunWorldMigration {
        #[arg(long, default_value_t = 400)]
        ticks: u64,
        #[arg(long, default_value_t = 5)]
        cognition_interval: u64,
        /// Cognition backend: echo | random | energy-rule
        #[arg(long, default_value = "energy-rule")]
        backend: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 20.0)]
        energy: f32,
        #[arg(long, default_value = "runs")]
        out: PathBuf,
    },
    /// Regenerate metrics from an event log without re-running cognition (M6).
    Replay {
        /// Path to the events.jsonl file.
        path: PathBuf,
    },
    /// Verify event-log structural integrity (seq numbers, version).
    Verify {
        /// Path to the events.jsonl file.
        path: PathBuf,
    },
    /// Print the event log of a finished experiment as a human-readable digest.
    Inspect {
        /// Path to the events.jsonl file.
        path: PathBuf,
        #[arg(long, default_value_t = 20)]
        tail: usize,
    },
}

fn parse_backend(name: &str, seed: u64) -> Result<BackendKind, String> {
    match name {
        "echo" => Ok(BackendKind::Echo),
        "random" => Ok(BackendKind::Random { seed }),
        "energy-rule" => Ok(BackendKind::EnergyRule {
            absorb_threshold: 0.5,
        }),
        "malformed" => Ok(BackendKind::Malformed),
        other => Err(format!(
            "unknown backend '{other}' (expected echo|random|energy-rule|malformed)"
        )),
    }
}

fn parse_primordial_backend(
    name: &str,
    seed: u64,
) -> Result<ts_core::primordial_experiment::BackendKind, String> {
    use ts_core::primordial_experiment::BackendKind as PB;
    match name {
        "echo" => Ok(PB::Echo),
        "random" => Ok(PB::Random { seed }),
        "energy-rule" => Ok(PB::EnergyRule {
            absorb_threshold: 0.5,
        }),
        "reproducing-rule" => Ok(PB::ReproducingRule {
            absorb_threshold: 0.3,
            divide_probe: 0.4,
        }),
        "signal-seeker" => Ok(PB::SignalSeeker {
            signal_threshold: 0.1,
        }),
        other => Err(format!(
            "unknown backend '{other}' (expected echo|random|energy-rule|reproducing-rule|signal-seeker)"
        )),
    }
}

/// Parse a migration/primal backend spec, supporting `llm:<model>` syntax
/// with an optional reasoning-effort suffix: `llm:<model>[:<effort>]`.
/// Thinking-style models (qwen3.5 etc.) need `:none` to stop hidden
/// reasoning from consuming the completion budget.
fn parse_any_backend(
    name: &str,
    seed: u64,
) -> Result<ts_core::primordial_experiment::BackendKind, String> {
    use ts_core::primordial_experiment::BackendKind as PB;
    if let Some(rest) = name.strip_prefix("llm:") {
        // Model tags contain ':' (e.g. "qwen3.5:2b-q4_K_M"), so the effort
        // suffix is only recognized when it is a known effort value.
        const KNOWN_EFFORTS: [&str; 4] = ["none", "low", "medium", "high"];
        let (model, reasoning_effort) = match rest.rsplit_once(':') {
            Some((m, effort)) if KNOWN_EFFORTS.contains(&effort) => (m, Some(effort.to_string())),
            _ => (rest, None),
        };
        let cfg = ts_core::llm_backend::LlmBackendConfig {
            model: model.to_string(),
            reasoning_effort,
            ..Default::default()
        };
        // LLM backends are wired per-run by the experiment runner (shared
        // instance); the enum variant carries the config.
        return Ok(PB::Llm { config: cfg });
    }
    parse_primordial_backend(name, seed)
}

fn parse_migration_backend(
    name: &str,
    seed: u64,
) -> Result<ts_core::primordial_experiment::BackendKind, String> {
    parse_any_backend(name, seed)
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::RunTest {
            ticks,
            cognition_interval,
            backend,
            seed,
            maintenance,
            energy,
            out,
        } => {
            let backend_kind = match parse_backend(&backend, seed) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };
            let config = ExperimentConfig {
                experiment_id: ExperimentId::generate(),
                world_seed: seed,
                ticks,
                cognition_interval,
                backend: backend_kind,
                maintenance_cost: maintenance,
                start_energy: energy,
            };
            let id = config.experiment_id.to_string();
            let dir = out.join(&id);
            let log_path = dir.join("events.jsonl");
            match run(&config, &log_path) {
                Ok(summary) => {
                    println!("experiment: {}", summary.experiment_id);
                    println!("backend:    {}", summary.backend_id);
                    println!("ticks:      {}", summary.ticks_executed);
                    println!(
                        "cognition events: {} (with action: {}, no action: {})",
                        summary.cognition_events,
                        summary.decisions_with_action,
                        summary.decisions_no_action
                    );
                    println!("backend errors: {}", summary.backend_errors);
                    println!("final energy:   {:.2}", summary.final_energy);
                    println!("final counter:  {:.2}", summary.final_counter);
                    println!("event log: {}", log_path.display());
                }
                Err(e) => {
                    eprintln!("experiment failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::RunPrimordial {
            ticks,
            cognition_interval,
            population,
            backend,
            seed,
            energy,
            out,
        } => {
            let backend_kind = match parse_any_backend(&backend, seed) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };
            let config = ts_core::primordial_experiment::PrimordialExperimentConfig {
                experiment_id: ExperimentId::generate(),
                world: Default::default(),
                world_seed: seed,
                ticks,
                cognition_interval,
                population,
                organism: ts_core::primordial_experiment::OrganismSpec {
                    backend: backend_kind,
                    start_energy: energy,
                },
            };
            let id = config.experiment_id.to_string();
            let dir = out.join(&id);
            let log_path = dir.join("events.jsonl");
            match ts_core::primordial_experiment::run_primordial(&config, &log_path) {
                Ok(summary) => {
                    println!("experiment: {}", summary.experiment_id);
                    println!("backend:    {backend}");
                    println!("ticks:      {}", summary.ticks_executed);
                    println!(
                        "population: {} initial, {} survivors, {} deaths, {} births",
                        summary.initial_population,
                        summary.survivors,
                        summary.deaths,
                        summary.births
                    );
                    if let Some(ml) = summary.mean_lifetime_of_dead {
                        println!("mean lifetime of dead: {ml:.1} ticks");
                    }
                    println!(
                        "lineage: {} distinct, max generation {}",
                        summary.distinct_lineages, summary.max_generation
                    );
                    println!(
                        "actions: absorb={} signal={} move={} divide={} rejected={} backend_errors={}",
                        summary.absorb_events,
                        summary.signal_events,
                        summary.move_events,
                        summary.divide_events,
                        summary.rejected_events,
                        summary.backend_errors
                    );
                    println!("cognition events total: {}", summary.cognition_events_total);
                    if let (Some(first), Some(last)) =
                        (summary.samples.first(), summary.samples.last())
                    {
                        println!(
                            "field energy: {:.0} -> {:.0}; mean org energy: {:.2} -> {:.2}",
                            first.field_energy,
                            last.field_energy,
                            first.mean_energy,
                            last.mean_energy
                        );
                    }
                    println!("event log: {}", log_path.display());
                }
                Err(e) => {
                    eprintln!("experiment failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::RunMigration {
            ticks,
            cognition_interval,
            backend_a,
            backend_b,
            seed,
            energy,
            out,
        } => {
            let bk_a = match parse_migration_backend(&backend_a, seed) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };
            let bk_b = match parse_migration_backend(&backend_b, seed) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };
            let config = ts_core::migration_experiment::MigrationExperimentConfig {
                experiment_id: ExperimentId::generate(),
                world: Default::default(),
                world_seed: seed,
                ticks,
                cognition_interval,
                start_energy: energy,
                backend_a: bk_a,
                backend_b: bk_b,
            };
            let id = config.experiment_id.to_string();
            let log_path = out.join(&id).join("events.jsonl");
            match ts_core::migration_experiment::run_migration_experiment(&config, &log_path) {
                Ok(result) => {
                    let mig = result.migration.as_ref().unwrap();
                    println!("experiment: {}", result.experiment_id);
                    println!(
                        "migration:  {} {} -> {} at tick {}",
                        mig.dimension, mig.from, mig.to, mig.tick
                    );
                    println!("migration id: {}", mig.migration_id);
                    for phase in &result.migration_run.phases {
                        println!(
                            "  phase {}: backend={} actions={} survived={}",
                            phase.label, phase.backend_id, phase.actions_taken, phase.survived
                        );
                    }
                    for dim in &result.continuity.dimensions {
                        println!("continuity ({}): {:.4}", dim.name, dim.value);
                        println!("  computation: {}", dim.computation);
                    }
                    println!(
                        "control continuity (no migration): {:.4}",
                        result.control_continuity.value
                    );
                    println!(
                        "migration effect (migration - control): {:+.4}",
                        result.migration_effect
                    );
                    println!("event log: {}", log_path.display());
                }
                Err(e) => {
                    eprintln!("experiment failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::RunScarcity {
            ticks,
            founders,
            regen,
            seeds,
            out,
        } => {
            let seed_list: Vec<u64> = seeds
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if seed_list.is_empty() {
                eprintln!("no valid seeds in '{seeds}'");
                std::process::exit(2);
            }
            let mut results = Vec::new();
            for seed in &seed_list {
                let cfg = ts_core::scarcity::ScarcityConfig {
                    world_seed: *seed,
                    ticks,
                    founders,
                    regen_rate: regen,
                    ..Default::default()
                };
                results.push((*seed, ts_core::scarcity::run_scarcity(&cfg, None)));
            }
            println!(
                "{:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8} {:>10}",
                "seed", "alive", "births", "deaths", "maxgen", "affinity", "founder", "lineages"
            );
            let mut affinity_deltas = Vec::new();
            for (seed, r) in &results {
                let aff = r.final_mean_affinity.unwrap_or(0.0);
                affinity_deltas.push(aff - r.founder_affinity);
                println!(
                    "{:>6} {:>6} {:>6} {:>6} {:>6} {:>8.4} {:>8.2} {:>10}",
                    seed,
                    r.final_alive,
                    r.total_births,
                    r.total_deaths,
                    r.max_generation,
                    aff,
                    r.founder_affinity,
                    r.final_lineages
                );
            }
            // Multi-seed summary of the selection signal. Extinction (all
            // deltas -1.0) is recorded as extinction, not as a selection
            // signal — dead populations have no traits to compare.
            let n = affinity_deltas.len() as f32;
            let mean: f32 = affinity_deltas.iter().sum::<f32>() / n;
            let var: f32 = affinity_deltas
                .iter()
                .map(|d| (d - mean) * (d - mean))
                .sum::<f32>()
                / n;
            let extinct = results.iter().all(|(_, r)| r.final_alive == 0);
            if extinct {
                println!("OUTCOME: extinction in all seeds — no trait comparison possible");
            } else {
                println!(
                    "mean affinity delta: {mean:+.4} (sd {:+.4}) over {} seeds",
                    var.sqrt(),
                    seed_list.len()
                );
                if mean > 0.02 {
                    println!(
                        "SELECTION SIGNAL: mean affinity increased (drift direction positive)"
                    );
                } else if mean < -0.02 {
                    println!("SELECTION SIGNAL: mean affinity decreased");
                } else {
                    println!("NO CLEAR SELECTION SIGNAL within |0.02| band");
                }
            }
            let _ = out;
        }
        Commands::RunCompetition {
            ticks,
            pairs,
            seeds,
        } => {
            let seed_list: Vec<u64> = seeds
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            println!(
                "{:>6} {:>6} {:>6} {:>8} {:>7} {:>7} {:>7} {:>7}",
                "seed", "aliveA", "aliveB", "shareA", "birthA", "birthB", "deathA", "deathB"
            );
            let mut shares = Vec::new();
            for seed in &seed_list {
                let cfg = ts_core::competition::CompetitionConfig {
                    world_seed: *seed,
                    ticks,
                    pairs,
                    ..Default::default()
                };
                let r = ts_core::competition::run_competition(&cfg);
                if let Some(share) = r.final_share_a {
                    shares.push(share);
                }
                println!(
                    "{:>6} {:>6} {:>6} {:>8.3} {:>7} {:>7} {:>7} {:>7}",
                    seed,
                    r.alive_a,
                    r.alive_b,
                    r.final_share_a.unwrap_or(f32::NAN),
                    r.births_a,
                    r.births_b,
                    r.deaths_a,
                    r.deaths_b
                );
            }
            if !shares.is_empty() {
                let mean: f32 = shares.iter().sum::<f32>() / shares.len() as f32;
                println!(
                    "mean shareA: {mean:.3} over {} seeds (0.5 = no advantage)",
                    shares.len()
                );
                if mean > 0.65 {
                    println!("SELECTION: high-affinity phenotype dominates");
                } else if mean > 0.55 {
                    println!("WEAK SELECTION: high-affinity phenotype favored");
                } else if (0.45..=0.55).contains(&mean) {
                    println!("NO ADVANTAGE (within 0.45-0.55 band)");
                } else {
                    println!("UNEXPECTED: low-affinity phenotype favored — investigate");
                }
            } else {
                println!("extinction in all seeds — no comparison possible");
            }
        }
        Commands::Benchmark { out } => {
            let report = ts_core::benchmark::run_benchmark();
            let json = serde_json::to_string_pretty(&report).unwrap_or_default();
            println!("{json}");
            if let Some(path) = out {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, format!("{json}\n"));
                println!("report written: {}", path.display());
            }
        }
        Commands::RunWorldMigration {
            ticks,
            cognition_interval,
            backend,
            seed,
            energy,
            out,
        } => {
            let backend_kind = match parse_any_backend(&backend, seed) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };
            let config = ts_core::world_migration::WorldMigrationConfig {
                experiment_id: ExperimentId::generate(),
                world: Default::default(),
                world_seed: seed,
                ticks,
                cognition_interval,
                start_energy: energy,
                backend: backend_kind,
                network: Default::default(),
            };
            let id = config.experiment_id.to_string();
            let log_path = out.join(&id).join("events.jsonl");
            match ts_core::world_migration::run_world_migration_experiment(&config, &log_path) {
                Ok(result) => {
                    println!("experiment: {}", config.experiment_id);
                    println!("backend:    {backend}");
                    println!("ticks:      {ticks}");
                    println!(
                        "migration:  {} {} -> {} at tick {}",
                        result.migration.migration_id,
                        result.migration.from,
                        result.migration.to,
                        result.migration.tick
                    );
                    println!("migration run:");
                    for d in &result.continuity.dimensions {
                        println!("  continuity {}: {:.4}", d.name, d.value);
                        println!("    computation: {}", d.computation);
                    }
                    println!("control run (no swap):");
                    for d in &result.control_continuity.dimensions {
                        println!("  continuity {}: {:.4}", d.name, d.value);
                        println!("    computation: {}", d.computation);
                    }
                    println!("migration effect (migration - control):");
                    for d in &result.effect.dimensions {
                        println!("  {}: {:+.4}", d.name, d.value);
                        println!("    computation: {}", d.computation);
                    }
                    println!("event log: {}", log_path.display());
                }
                Err(e) => {
                    eprintln!("experiment failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Replay { path } => match ts_core::replay::regenerate_metrics(&path) {
            Ok(m) => {
                println!("total records:        {}", m.total_records);
                println!("subjects created:     {}", m.subjects_created);
                println!(
                    "decisions:            {} (with action: {}, no action: {})",
                    m.decisions, m.decisions_with_action, m.decisions_no_action
                );
                println!("action distribution:");
                for (name, count) in &m.action_distribution {
                    println!("  {name}: {count}");
                }
                println!("deaths:               {}", m.deaths);
                println!("births (divisions):   {}", m.births);
                println!("migrations:           {}", m.migrations);
                println!(
                    "finish:               {} ({})",
                    m.experiments_finished,
                    m.finish_reason.as_deref().unwrap_or("<none>")
                );
                println!(
                    "tick range:           {:?}..={:?}",
                    m.first_tick, m.last_tick
                );
                println!("population samples:   {}", m.population_samples.len());
            }
            Err(e) => {
                eprintln!("replay failed: {e}");
                std::process::exit(1);
            }
        },
        Commands::Verify { path } => match ts_core::replay::verify_log_integrity(&path) {
            Ok(r) => {
                println!("records:      {}", r.records);
                println!("seq ok:       {}", r.seq_ok);
                println!("version ok:   {} (v{})", r.version_ok, r.log_version_seen);
                println!("first bad seq: {:?}", r.first_bad_seq);
                if !r.seq_ok || !r.version_ok {
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("verify failed: {e}");
                std::process::exit(1);
            }
        },
        Commands::Inspect { path, tail } => {
            let records = match ts_core::event_log::read_log(&path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("cannot read {}: {e}", path.display());
                    std::process::exit(1);
                }
            };
            println!("total records: {}", records.len());
            let start = records.len().saturating_sub(tail);
            for r in &records[start..] {
                let line = match &r.payload {
                    EventPayload::ExperimentStarted {
                        experiment_id,
                        seed,
                        ..
                    } => format!("seq={} START exp={experiment_id} seed={seed}", r.seq),
                    EventPayload::SubjectCreated { subject_id, .. } => {
                        format!("seq={} SUBJECT_CREATED {}", r.seq, subject_id)
                    }
                    EventPayload::DecisionMade {
                        subject_id,
                        tick,
                        decision,
                    } => format!(
                        "seq={} DECISION t={tick} subj={subject_id} action={}",
                        r.seq,
                        decision
                            .action
                            .as_ref()
                            .map(|a| a.name.as_str())
                            .unwrap_or("<none>")
                    ),
                    EventPayload::WorldEvents { tick, events } => format!(
                        "seq={} WORLD t={tick} {}",
                        r.seq,
                        events
                            .iter()
                            .map(|e| match e {
                                ts_core::wabix::WorldEvent::ActionApplied { action, .. } =>
                                    format!("{action}!"),
                                ts_core::wabix::WorldEvent::ActionRejected { action, .. } =>
                                    format!("{action}✗"),
                                ts_core::wabix::WorldEvent::SubjectDied { cause, .. } =>
                                    format!("DIED({cause})"),
                            })
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    EventPayload::SubjectState {
                        subject_id,
                        tick,
                        energy,
                        ..
                    } => format!(
                        "seq={} STATE t={tick} subj={subject_id} energy={energy:.2}",
                        r.seq
                    ),
                    EventPayload::ResourceConsumed(e) => format!(
                        "seq={} RESOURCE t={} {}={} by {}",
                        r.seq,
                        e.tick,
                        e.kind.as_str(),
                        e.amount,
                        e.consumer
                    ),
                    EventPayload::Migration {
                        dimension,
                        from,
                        to,
                        tick,
                        ..
                    } => format!(
                        "seq={} MIGRATION t={tick} {dimension}: {from} -> {to}",
                        r.seq
                    ),
                    EventPayload::ExperimentFinished { reason } => {
                        format!("seq={} FINISHED: {reason}", r.seq)
                    }
                };
                println!("{line}");
            }
        }
    }
}
