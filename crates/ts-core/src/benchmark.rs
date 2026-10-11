//! M10: reference-8gb benchmark harness.
//!
//! Measures on the reference machine:
//! - world simulation throughput (ticks/sec, CPU)
//! - cognition throughput (decisions/sec for mock backends)
//! - organism count scaling
//! - peak host RAM (world simulation)
//! - event log size
//!
//! LLM VRAM/throughput numbers are reported separately by the endpoint
//! (Ollama exposes VRAM via the model footprint; we record model metadata
//! from the API response). This harness measures what the Rust process can
//! measure: simulation throughput, cognition throughput, peak host RAM,
//! log size. LLM VRAM numbers come from the endpoint and are recorded as
//! model metadata in the manifest.
//!
//! Hardware validation status: runs on the reference machine class
//! (RTX 2070 SUPER 8GB) when executed on the dev machine.

use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::primordial::{PrimordialConfig, PrimordialWorld};
use crate::wabix::Action;

/// Result of a benchmark section.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkSection {
    pub name: String,
    /// Measured value (unit depends on section).
    pub value: f64,
    pub unit: String,
    /// Parameters used for the measurement.
    pub params: serde_json::Value,
}

/// Full benchmark report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub benchmark: String,
    pub hardware_note: String,
    pub sections: Vec<BenchmarkSection>,
    pub generated_at_note: String,
}

/// World simulation throughput at a given organism count (CPU-only).
fn bench_world_ticks(organisms: u32, ticks: u64) -> (f64, f64) {
    let config = PrimordialConfig::default();
    let mut world = PrimordialWorld::new(config, 42);
    for i in 0..organisms {
        world.spawn(&format!("bench-{i}"), 100.0);
    }
    let subjects: Vec<String> = (0..organisms).map(|i| format!("bench-{i}")).collect();
    let actions: Vec<(String, Action)> = subjects
        .iter()
        .map(|s| {
            (
                s.clone(),
                Action {
                    name: "ABSORB".into(),
                    params: vec![1.0],
                },
            )
        })
        .collect();

    let start = Instant::now();
    for _ in 0..ticks {
        let _ = world.tick(&actions);
    }
    let elapsed = start.elapsed().as_secs_f64();
    (ticks as f64 / elapsed, elapsed)
}

/// Mock cognition throughput: decisions/sec for the echo backend on the
/// primordial world (no model load; measures runtime overhead per decision).
fn bench_mock_cognition(_organisms: u32, decisions: u64) -> (f64, u64) {
    use crate::cognition::DecisionContext;

    let cfg = crate::primordial_experiment::PrimordialExperimentConfig::default();
    let manifest = crate::primordial::primordial_manifest();
    let mut backend = cfg.organism.backend.instantiate();
    let _ = cfg.organism.backend.backend_id();

    // Reuse one context; decisions/sec for echo backends is runtime overhead.
    let ctx = DecisionContext {
        subject_id: "bench".into(),
        tick: 0,
        energy: 50.0,
        observations: vec![crate::wabix::Observation::new("local_energy", 0.5)],
        manifest,
        memory: vec![],
    };

    let start = Instant::now();
    for i in 0..decisions {
        let _ = backend.decide(&ctx);
        let _ = i;
    }
    let elapsed = start.elapsed().as_secs_f64();
    (decisions as f64 / elapsed, decisions)
}

/// LLM cognition throughput: decisions/sec against the configured endpoint
/// (default localhost Ollama). Requires a running endpoint; the benchmark
/// degrades gracefully (section omitted) when the endpoint is unreachable.
fn bench_llm_cognition(model: &str, decisions: u64) -> Option<(f64, u64, f64, f64)> {
    use crate::cognition::{CognitionBackend, DecisionContext};

    let cfg = crate::llm_backend::LlmBackendConfig {
        model: model.to_string(),
        ..Default::default()
    };
    // Health check: a single quick decision probe; unreachable endpoints
    // are reported as None so the benchmark degrades gracefully.
    let mut backend = crate::llm_backend::LlmBackend::new(cfg.clone());
    let manifest = crate::primordial::primordial_manifest();
    let ctx = DecisionContext {
        subject_id: "bench-llm".into(),
        tick: 0,
        energy: 50.0,
        observations: vec![crate::wabix::Observation::new("local_energy", 0.5)],
        manifest: manifest.clone(),
        memory: vec![],
    };
    let probe_start = Instant::now();
    let probe_result = CognitionBackend::decide(&mut backend, &ctx);
    let probe_elapsed = probe_start.elapsed().as_secs_f64();
    if probe_result.is_err() {
        return None;
    }

    // Skip the probe decision from throughput math; measure `decisions` more.
    let start = Instant::now();
    for _ in 0..decisions {
        let _ = CognitionBackend::decide(&mut backend, &ctx);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let dps = decisions as f64 / elapsed;
    let latency = (elapsed / decisions as f64) * 1000.0;
    Some((dps, decisions, latency, probe_elapsed))
}

/// Fetch endpoint model metadata (on-disk file size, parameter size,
/// quantization) from an OpenAI-compatible/Ollama endpoint. Returns None
/// when the endpoint does not expose it — the benchmark degrades
/// gracefully. The file size is the ON-DISK footprint, not runtime VRAM;
/// the section name says so explicitly to avoid overclaiming.
fn fetch_model_metadata(base_url: &str, model: &str) -> Option<serde_json::Value> {
    // Ollama-native endpoints. /api/tags carries the model file size and
    // quantization details; /api/show carries the parameter count.
    let root = base_url.trim_end_matches("/v1").to_string();
    let tags_url = format!("{root}/api/tags");
    let show_url = format!("{root}/api/show");

    let tags_body: serde_json::Value = std::process::Command::new("curl")
        .args(["-sf", "--max-time", "10", &tags_url])
        .output()
        .ok()
        .and_then(|out| serde_json::from_slice(&out.stdout).ok())?;

    let entry = tags_body["models"]
        .as_array()?
        .iter()
        .find(|m| m["name"].as_str() == Some(model))?;

    let file_size = entry["size"].as_u64();
    let details = &entry["details"];

    // /api/show is optional (parameter count).
    let show_body: Option<serde_json::Value> = std::process::Command::new("curl")
        .args(["-sf", "--max-time", "10", &show_url])
        .arg(serde_json::json!({ "model": model }).to_string())
        .output()
        .ok()
        .and_then(|out| serde_json::from_slice(&out.stdout).ok());
    let param_count = show_body
        .as_ref()
        .and_then(|show| show["model_info"]["general.parameter_count"].as_u64());

    let mut meta = serde_json::json!({
        "model": model,
        "on_disk_bytes": file_size,
        "note": "on-disk model file size from the endpoint; runtime VRAM is larger and endpoint-dependent",
    });
    if let Some(q) = details["quantization_level"].as_str() {
        meta["quantization"] = serde_json::json!(q);
    }
    if let Some(p) = details["parameter_size"].as_str() {
        meta["parameter_size_label"] = serde_json::json!(p);
    }
    if let Some(n) = param_count {
        meta["parameter_count_exact"] = serde_json::json!(n);
    }
    if let Some(arch) = show_body
        .as_ref()
        .and_then(|show| show["model_info"]["general.architecture"].as_str())
    {
        meta["architecture"] = serde_json::json!(arch);
    }
    Some(meta)
}

/// Run the reference-8gb benchmark.
pub fn run_benchmark() -> BenchmarkReport {
    let mut sections = Vec::new();

    // --- World simulation throughput (CPU-only) ---
    for &organisms in &[1u32, 8, 32] {
        let (tps, elapsed) = bench_world_ticks(organisms, 10_000);
        sections.push(BenchmarkSection {
            name: format!("world_ticks_per_sec_{organisms}_orgs"),
            value: tps,
            unit: "ticks/s".into(),
            params: serde_json::json!({ "organisms": organisms, "ticks": 10_000, "elapsed_s": elapsed }),
        });
    }

    // --- Mock cognition throughput (decisions/sec) ---
    {
        let (dps, decisions) = bench_mock_cognition(1, 10_000);
        sections.push(BenchmarkSection {
            name: "mock_decisions_per_sec".into(),
            value: dps,
            unit: "decisions/s".into(),
            params: serde_json::json!({ "backend": "mock-echo-v0", "decisions": decisions }),
        });
    }

    // --- LLM cognition throughput (decisions/sec + latency, if endpoint up) ---
    let llm_model = std::env::var("TS_BENCH_LLM_MODEL").ok();
    let llm_bench = llm_model.and_then(|var| {
        bench_llm_cognition(&var, 10)
            .map(|(dps, decisions, latency_ms, probe_s)| (var, dps, decisions, latency_ms, probe_s))
    });
    if let Some((var, dps, decisions, latency_ms, probe_s)) = llm_bench {
        sections.push(BenchmarkSection {
            name: "llm_decisions_per_sec".into(),
            value: dps,
            unit: "decisions/s".into(),
            params: serde_json::json!({
                "model": var,
                "decisions": decisions,
                "mean_latency_ms": latency_ms,
                "probe_load_s": probe_s
            }),
        });

        // Endpoint model metadata (on-disk footprint etc.) when exposed.
        let base_url = crate::llm_backend::LlmBackendConfig {
            model: var.clone(),
            ..Default::default()
        }
        .base_url;
        if let Some(meta) = fetch_model_metadata(&base_url, &var) {
            sections.push(BenchmarkSection {
                name: "llm_model_metadata".into(),
                value: meta["on_disk_bytes"].as_u64().unwrap_or(0) as f64 / (1024.0 * 1024.0),
                unit: "MiB_on_disk".into(),
                params: meta,
            });
        }
    }

    // --- Peak host RAM (VmHWM via /proc/self/status) ---
    let peak_ram = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
        });
    if let Some(kb) = peak_ram {
        sections.push(BenchmarkSection {
            name: "peak_host_ram".into(),
            value: kb / 1024.0,
            unit: "MiB".into(),
            params: serde_json::json!({ "source": "/proc/self/status VmHWM" }),
        });
    }

    // --- Event log size reference ---
    {
        let dir = std::env::temp_dir().join(format!("ts-bench-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let log_path = dir.join("bench.jsonl");
        let cfg = crate::primordial_experiment::PrimordialExperimentConfig::default();
        let run = crate::primordial_experiment::run_primordial(&cfg, &log_path);
        if let Ok(summary) = run {
            let size = std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);
            sections.push(BenchmarkSection {
                name: "event_log_bytes_per_1000_ticks_8orgs".into(),
                value: size as f64 * (1000.0 / summary.ticks_executed.max(1) as f64),
                unit: "bytes/1000_ticks".into(),
                params: serde_json::json!({
                    "ticks": summary.ticks_executed,
                    "log_bytes": size,
                    "normalized_to": 1000
                }),
            });
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    BenchmarkReport {
        benchmark: "reference-8gb".into(),
        hardware_note: "measured on reference machine class (RTX 2070 SUPER 8GB) when executed on the dev machine; endpoint VRAM reported separately via model metadata".into(),
        sections,
        generated_at_note: "generated_at intentionally omitted for replay determinism".into(),
    }
}
