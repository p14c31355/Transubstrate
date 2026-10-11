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
        other => Err(format!(
            "unknown backend '{other}' (expected echo|random|energy-rule|reproducing-rule)"
        )),
    }
}

/// Parse a migration/primal backend spec, supporting `llm:<model>` syntax.
fn parse_any_backend(
    name: &str,
    seed: u64,
) -> Result<ts_core::primordial_experiment::BackendKind, String> {
    use ts_core::primordial_experiment::BackendKind as PB;
    if let Some(model) = name.strip_prefix("llm:") {
        let cfg = ts_core::llm_backend::LlmBackendConfig {
            model: model.to_string(),
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
    use ts_core::primordial_experiment::BackendKind as PB;
    match name {
        "echo" => Ok(PB::Echo),
        "random" => Ok(PB::Random { seed }),
        "energy-rule" => Ok(PB::EnergyRule {
            absorb_threshold: 0.3,
        }),
        other => Err(format!(
            "unknown backend '{other}' (expected echo|random|energy-rule)"
        )),
    }
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
                    let dim = &result.continuity.dimensions[0];
                    println!("continuity (behavioral): {:.4}", dim.value);
                    println!("  computation: {}", dim.computation);
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
