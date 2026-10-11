//! M7: Local LLM cognition backend via an OpenAI-compatible API
//! (Ollama / llama.cpp server).
//!
//! 8 GiB VRAM contract:
//! - ONE shared backend instance serves ALL organisms (the model is loaded
//!   once; per-organism state stays in the subject runtime)
//! - target model class: ≤3B quantized (Q4_K_M) ≈ 2-3 GB VRAM
//! - bounded context: prompt is compiled from a bounded DecisionContext
//! - typed decisions: the model must answer with a JSON action object;
//!   anything else is a MalformedDecision (fails safely)
//! - token costs are recorded into the resource ledger
//! - no paid cloud: localhost Ollama or llama.cpp server
//!
//! Determinism note: LLM sampling is NOT deterministic by default; the
//! backend reports its nondeterminism policy via `is_deterministic() = false`
//! (temperature 0 helps but is not a guarantee across versions). Experiments
//! requiring bit-reproducibility must use the mock backends; LLM runs record
//! their outputs in the event log for post-hoc analysis.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::cognition::{
    CognitionBackend, CognitionCost, CognitionError, Decision, DecisionContext,
};

/// Connection configuration for the OpenAI-compatible endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlmBackendConfig {
    /// Base URL, e.g. "http://localhost:11434/v1" (Ollama) or
    /// "http://localhost:8080/v1" (llama.cpp server).
    pub base_url: String,
    /// Model identifier at the endpoint, e.g. "granite4.1:3b-q4_K_M".
    pub model: String,
    /// Sampling temperature (0 recommended for decision stability).
    pub temperature: f32,
    /// Hard cap on completion tokens per decision.
    pub max_tokens: u32,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Optional API key (sent as Bearer); empty = no auth header. Local
    /// servers don't need one; this exists for optional remote endpoints.
    #[serde(default)]
    pub api_key: String,
}

impl Default for LlmBackendConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434/v1".into(),
            model: "granite4.1:3b-q4_K_M".into(),
            temperature: 0.0,
            max_tokens: 200,
            timeout_secs: 120,
            api_key: String::new(),
        }
    }
}

/// Parsed model decision (strict JSON schema).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlmDecision {
    /// Action name; null = no action this event.
    pub action: Option<String>,
    /// Action parameters (floats), matching the world's action schema.
    #[serde(default)]
    pub params: Vec<f32>,
    /// One-line rationale (bounded by max_tokens).
    #[serde(default)]
    pub why: String,
}

/// The shared LLM backend. Clone-friendly: the config is cheap; HTTP clients
/// are per-call (Ollama handles connection reuse server-side).
#[derive(Clone, Debug)]
pub struct LlmBackend {
    config: LlmBackendConfig,
    /// Cumulative token usage across all decisions (for the ledger).
    pub total_prompt_tokens: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub total_completion_tokens: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl LlmBackend {
    pub fn new(config: LlmBackendConfig) -> Self {
        Self {
            config,
            total_prompt_tokens: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_completion_tokens: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Build the bounded decision prompt. No semantic leakage: the prompt
    /// describes channels and actions numerically; it never says "you are
    /// alive", "survive", "find food", or assigns meaning to channels.
    pub fn build_prompt(ctx: &DecisionContext) -> String {
        use std::fmt::Write;
        let mut obs = String::new();
        for o in &ctx.observations {
            let _ = writeln!(obs, "{} = {:.4}", o.channel, o.value);
        }
        let mut acts = String::new();
        for a in &ctx.manifest.actions {
            let _ = writeln!(
                acts,
                "- {} (arity {}, ranges {:?})",
                a.name, a.arity, a.param_ranges
            );
        }
        let mut mem = String::new();
        for m in ctx.memory.iter().rev().take(5) {
            let _ = writeln!(
                mem,
                "tick {}: {}{:?}",
                m.tick, m.action.name, m.action.params
            );
        }
        format!(
            "You are the decision module of an agent in a simulated environment.\n\
             Tick: {tick}\nEnergy level (normalized 0-1): {energy:.4}\n\
             \nObservations (channel = value; channels have no inherent meaning):\n{obs}\n\
             Available actions:\n{acts}\n\
             \nRecent past decisions:\n{mem}\n\
             Choose ONE action. Respond with ONLY a JSON object:\n\
             {{\"action\": \"<name or null>\", \"params\": [<numbers>], \"why\": \"<10 words max>\"}}\n",
            tick = ctx.tick,
            energy = ctx.energy,
            obs = obs,
            acts = acts,
            mem = if mem.is_empty() { "(none)" } else { &mem }
        )
    }

    /// Call the endpoint and parse the decision. Public for testing.
    fn call_api(&self, prompt: &str) -> Result<(String, u64, u64), CognitionError> {
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build();
        let mut req = client.post(&url).set("Content-Type", "application/json");
        if !self.config.api_key.is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.config.api_key));
        }

        let resp = req
            .send_json(body)
            .map_err(|e| CognitionError::Unavailable(format!("llm endpoint {url}: {e}")))?;

        let json: serde_json::Value = resp
            .into_json()
            .map_err(|e| CognitionError::MalformedDecision(format!("non-JSON response: {e}")))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CognitionError::MalformedDecision("missing choices[0].message.content".into())
            })?
            .to_string();

        let prompt_tokens = json
            .pointer("/usage/prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let completion_tokens = json
            .pointer("/usage/completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok((content, prompt_tokens, completion_tokens))
    }

    /// Extract the JSON decision from model output. Models sometimes wrap
    /// JSON in markdown fences or prose; we scan for the outermost object.
    pub fn parse_decision(content: &str) -> Result<LlmDecision, CognitionError> {
        let trimmed = content.trim();
        // Fast path: direct JSON.
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed).map_err(|e| {
                CognitionError::MalformedDecision(format!("unparseable decision JSON: {e}"))
            });
        }
        // Slow path: find the first {...} block (manual Option algebra to
        // avoid let-chains, which this crate's lints disallow here).
        let json_block = match (trimmed.find('{'), trimmed.rfind('}')) {
            (Some(s), Some(e)) if s < e => Some(&trimmed[s..=e]),
            _ => None,
        };
        if let Some(slice) = json_block {
            return serde_json::from_str(slice).map_err(|e| {
                CognitionError::MalformedDecision(format!("unparseable decision JSON: {e}"))
            });
        }
        Err(CognitionError::MalformedDecision(format!(
            "no JSON object in model output: {:.80}",
            trimmed.escape_debug()
        )))
    }
}

impl CognitionBackend for LlmBackend {
    fn implementation(&self) -> &'static str {
        "llm-openai-compat-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let prompt = Self::build_prompt(ctx);
        let (content, ptok, ctok) = self.call_api(&prompt)?;
        self.total_prompt_tokens
            .fetch_add(ptok, std::sync::atomic::Ordering::Relaxed);
        self.total_completion_tokens
            .fetch_add(ctok, std::sync::atomic::Ordering::Relaxed);

        let parsed = Self::parse_decision(&content)?;

        // Convert to the runtime Decision; WABIX schema validation happens
        // in the runner (validate_decision) — schema violations fail safely.
        let action = parsed.action.map(|name| crate::wabix::Action {
            name,
            params: parsed.params,
        });

        Ok(Decision {
            action,
            rationale: format!("llm[{}]: {}", self.config.model, parsed.why),
        })
    }
}

/// Token cost reporting: pull cumulative usage from the backend and record
/// it into a ledger. Call after runs (the backend accumulates atomically).
pub fn charge_llm_tokens(
    backend: &LlmBackend,
    ledger: &mut crate::resource::ResourceLedger,
    tick: u64,
    consumer: &str,
) {
    use std::sync::atomic::Ordering;
    let ptok = backend.total_prompt_tokens.load(Ordering::Relaxed);
    let ctok = backend.total_completion_tokens.load(Ordering::Relaxed);
    if ptok > 0 {
        ledger.record(crate::resource::ResourceEntry {
            tick,
            kind: crate::resource::ResourceKind::Tokens,
            amount: ptok,
            consumer: format!("{consumer}:prompt"),
        });
    }
    if ctok > 0 {
        ledger.record(crate::resource::ResourceEntry {
            tick,
            kind: crate::resource::ResourceKind::Tokens,
            amount: ctok,
            consumer: format!("{consumer}:completion"),
        });
    }
}

/// Cost profile for one LLM decision (post-hoc accounting basis).
pub fn llm_decision_cost(prompt_tokens: u64, completion_tokens: u64) -> CognitionCost {
    CognitionCost {
        cognition_events: 1,
        tokens: prompt_tokens + completion_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::validate_decision;
    use crate::wabix::ManifestBuilder;

    #[test]
    fn parse_direct_json() {
        let d = LlmBackend::parse_decision("{\"action\":\"WAIT\",\"params\":[],\"why\":\"rest\"}")
            .unwrap();
        assert_eq!(d.action.as_deref(), Some("WAIT"));
        assert!(d.params.is_empty());
    }

    #[test]
    fn parse_fenced_json() {
        let d = LlmBackend::parse_decision(
            "```json\n{\"action\":\"ABSORB\",\"params\":[0.5],\"why\":\"take\"}\n```",
        )
        .unwrap();
        assert_eq!(d.action.as_deref(), Some("ABSORB"));
        assert_eq!(d.params, vec![0.5]);
    }

    #[test]
    fn parse_prose_wrapped_json() {
        let d = LlmBackend::parse_decision(
            "Sure! Here is my decision: {\"action\":null,\"params\":[],\"why\":\"no-op\"} hope that helps",
        )
        .unwrap();
        assert!(d.action.is_none());
    }

    #[test]
    fn parse_garbage_fails_safely() {
        let err =
            LlmBackend::parse_decision("I think I should move north because reasons").unwrap_err();
        assert!(matches!(err, CognitionError::MalformedDecision(_)));
    }

    #[test]
    fn prompt_has_no_semantic_leakage() {
        let manifest = ManifestBuilder::new("w")
            .channel("local_energy", "scalar", (0.0, 1.0))
            .action("WAIT", 0, vec![])
            .action("ABSORB", 1, vec![(0.0, 1.0)])
            .build();
        let ctx = DecisionContext {
            subject_id: "s".into(),
            tick: 5,
            energy: 0.4,
            observations: vec![crate::wabix::Observation::new("local_energy", 0.42)],
            manifest,
            memory: vec![],
        };
        let prompt = LlmBackend::build_prompt(&ctx);
        let lower = prompt.to_lowercase();
        for forbidden in ["alive", "survive", "food", "eat", "hungry", "die", "kill"] {
            assert!(
                !lower.contains(forbidden),
                "prompt must not contain semantic-leak word '{forbidden}'"
            );
        }
        assert!(prompt.contains("local_energy = 0.4200"));
        assert!(prompt.contains("ABSORB"));
    }

    #[test]
    fn llm_decision_converts_and_validates() {
        // A parsed LlmDecision converts into a runtime Decision that passes
        // WABIX validation when the action exists with valid params.
        let manifest = ManifestBuilder::new("w")
            .action("ABSORB", 1, vec![(0.0, 1.0)])
            .build();
        let d =
            LlmBackend::parse_decision("{\"action\":\"ABSORB\",\"params\":[0.7],\"why\":\"x\"}")
                .unwrap();
        let decision = Decision {
            action: d.action.map(|name| crate::wabix::Action {
                name,
                params: d.params,
            }),
            rationale: d.why,
        };
        assert!(validate_decision(&manifest, &decision).is_ok());
    }

    #[test]
    fn schema_violation_from_llm_output_fails_validation() {
        let manifest = ManifestBuilder::new("w")
            .action("ABSORB", 1, vec![(0.0, 1.0)])
            .build();
        // Model proposes an unknown action -> typed violation, safe failure.
        let d =
            LlmBackend::parse_decision("{\"action\":\"FLY\",\"params\":[],\"why\":\"x\"}").unwrap();
        let decision = Decision {
            action: d.action.map(|name| crate::wabix::Action {
                name,
                params: d.params,
            }),
            rationale: d.why,
        };
        assert!(validate_decision(&manifest, &decision).is_err());
    }
}
