# Prior Art Survey

Status: **working draft** — compiled from web-search-verified sources.
Entries marked UNVERIFIED could not be confirmed to exist as described;
we say so rather than inventing details. This document is maintained
incrementally; "to the best of our current survey" applies throughout.

Transubstrate's core intended contributions, against which each system is
compared:

1. SUBJECT ≠ MODEL — persistent subject abstraction with cognition as a
   replaceable organ
2. Measured continuity across controlled substrate migrations
3. WABIX — a world ABI (capability discovery, typed observations/actions)
   shared by simulation worlds and future physical bodies
4. Primordial ALife world with costly cognition and no semantic leakage
5. Shared cognition model (VRAM does not scale with organism count)
6. Deterministic replay / event-log reproducibility

---

## Verified systems

### OpenLife (Masumori & Ikegami)

- **Source**: arXiv:2606.31046
- **Purpose**: LLM-driven artificial life; agents whose "life" is grounded in
  LLM autopoiesis — self-producing conversational/organizational closure.
- **Architecture**: LLM agents interact without predefined tasks; the research
  observes whether life-like organization self-maintains. Explicit framing as
  an autopoiesis experiment rather than an agent benchmark.
- **Maturity**: research paper + code; academic.
- **Overlap**: strongest adjacent work on the "LLM organisms in a shared
  world" axis; shares the interest in organization emerging from interaction
  rather than prompts.
- **Differences**: OpenLife treats the LLM as the organism's essence;
  Transubstrate treats the model as a replaceable organ and focuses on
  continuity measurement across model swaps. OpenLife does not build
  migration/continuity instrumentation.
- **Lessons to reuse**: their careful operational-language discipline; the
  observation that semantics leaking through prompts contaminates ALife
  claims (motivates our no-semantic-leakage rule).
- **Remaining gap**: no substrate-independence, no measured continuity, no
  resource-costly cognition, no deterministic replay.

### Lenia (Bert Chan)

- **Source**: github.com/Chakazul/Lenia (≈3.8k stars)
- **Purpose**: continuous cellular automata producing life-like, self-
  organizing "orbium" creatures; mathematical ALife substrate.
- **Architecture**: PDE-like continuous CA with kernel-based convolutions;
  no agents, no cognition — emergent morphology from update rules.
- **Maturity**: mature, well-documented, award-winning ALife research.
- **Overlap**: shares determinism and emergence-without-semantic-leakage
  values; a candidate inspiration for future field dynamics.
- **Differences**: no subjects, no identity, no cognition, no migration —
  purely morphological emergence.
- **Lessons to reuse**: parameterized rule families as first-class
  experiment variables; beauty/analysis of single-run trajectories.
- **Remaining gap**: everything subject-related; Lenia has no subjects.

### Generative Agents / Smallville (Park et al., 2023)

- **Source**: arXiv:2304.03442
- **Purpose**: 25 LLM agents in a Smalltown simulation with memory stream,
  reflection, and planning producing believable social behavior.
- **Architecture**: per-agent memory stream (recency/importance/relevance
  retrieval), reflection tree, environment scheduler; LLM called per
  decision.
- **Maturity**: famous paper, widely reproduced; Stanford code released.
- **Overlap**: multi-agent LLM simulation with per-agent memory — the
  closest "society simulator" work.
- **Differences**: agents are defined BY their prompt+memory; replacing a
  model is not studied; no resource economics (LLM calls are unbounded
  infrastructure); no continuity measurement; no ALife framing (believability
  is the target).
- **Lessons to reuse**: memory retrieval scoring (recency/importance/
  relevance) is a proven design for our M7 per-subject memory.
- **Remaining gap**: substrate replacement, cost-of-thought, replay
  determinism (their runs are not reproducible bit-for-bit).

### Shachi (Sakana AI)

- **Source**: arXiv:2509.21862
- **Purpose**: framework for LLM agent-based modeling studying emergent
  collective behavior; configuration-driven agent societies.
- **Architecture**: modular agent config (memory/tools/knowledge) ×
  population scaling for studying emergence of collective behavior.
- **Maturity**: recent (2025), actively maintained by a serious lab.
- **Overlap**: systematic ABM methodology with LLM cognition; configuration
  as an experimental variable (similar to our genome/config separation).
- **Differences**: focused on collective behavior, not individual subject
  continuity; no migration of agents across substrates; no resource-cost
  model of cognition.
- **Lessons to reuse**: their config taxonomy (memory/tools/knowledge)
  informs our state taxonomy (genome/lifetime/memory/culture/weights).
- **Remaining gap**: subject persistence across model replacement; costly
  cognition; deterministic replay.

### AlphaEvolve (DeepMind)

- **Source**: deepmind.google/blog/alphaevolve (2025)
- **Purpose**: evolutionary coding agent: LLM proposes code mutations,
  evaluators select; has improved matrix multiplication and datacenter
  scheduling algorithms.
- **Architecture**: population of programs; LLM-driven mutation; automated
  evaluators as selection pressure.
- **Maturity**: production research system with published wins.
- **Overlap**: evolution where the genome is configuration/code and
  selection is automated — analogous to our genome + environment-as-selector.
- **Differences**: evolves *artifacts* (code), not *subjects*; no identity
  continuity question; evaluators are fixed objective functions, not a
  lived environment.
- **Lessons to reuse**: strict separation of variation (LLM) from selection
  (evaluator/environment); artifact-quality regression harnesses.
- **Remaining gap**: persistent subjects; continuity across substrate
  replacement is not a meaningful question for artifact evolution.

### MemGPT / Letta

- **Source**: arXiv:2310.08560; github.com/letta-ai/letta
- **Purpose**: LLM context management as an operating system: paged memory
  tiers (main context / external storage), self-directed memory paging.
- **Architecture**: virtual-context management; the model calls tools to
  page memories in/out; personas persist across sessions.
- **Maturity**: mature; Letta is a funded company product.
- **Overlap**: closest prior art for bounded memory + persistent persona;
  validates that memory architecture (not model identity) carries behavioral
  continuity.
- **Differences**: persistence serves chat continuity, not experimental
  continuity measurement; no migration across heterogeneous models with
  measured outcomes; no world/embodiment dimension.
- **Lessons to reuse**: memory paging discipline; explicit memory tiers
  (core/archival) maps to our planned bounded per-subject memory design.
- **Remaining gap**: migration science; world ABI; resource economics.

### W3C Web of Things (WoT)

- **Source**: w3.org/TR/wot-thing-description11
- **Purpose**: standardized Thing Descriptions: metadata + interaction
  affordances (properties/actions/events) for IoT devices.
- **Architecture**: JSON-LD Thing Description documents; binding to
  protocols (HTTP/MQTT); discovery via directories.
- **Maturity**: W3C Recommendation (TD 1.1); industrial backing.
- **Overlap**: the closest prior art to WABIX — explicit capability
  discovery and typed interactions between software and world hardware.
- **Differences**: WoT is a document/integration standard for the IoT web;
  no subject concept, no observation semantics (uncertainty/frames), no
  simulation worlds, no action validation semantics as strict as our
  manifest-range validation. WABIX targets the ABI role (compile-against-
  and-execute) rather than the interoperability-description role.
- **Lessons to reuse**: affordance taxonomy (property/action/event)
  validates our discovery-then-act flow; TD's security-scheme separation
  informs our future CapabilityGrant design.
- **Remaining gap**: WABIX should explicitly position itself against WoT
  TD and consider a TD compatibility profile rather than reinventing
  discovery serialization.

### ROS2 / ros2_control

- **Source**: ros2.org; control.ros.org
- **Purpose**: robot middleware (DDS-based graph) and a real-time control
  loop framework (controller_manager, hardware_interface).
- **Architecture**: nodes/topics/services; ros2_control separates hardware
  interfaces from controllers via lifecycle management.
- **Maturity**: dominant open robot stack; production.
- **Overlap**: hardware abstraction and controller lifecycle — the
  engineering godfather of the "world boundary" idea.
- **Differences**: ROS is a *middleware graph* (many nodes, dynamic
  discovery, QoS), not a per-subject ABI; nothing persists across machines
  as a subject; no cognition concept; heavy runtime footprint.
- **Lessons to reuse**: lifecycle states (unconfigured/inactive/active)
  for our future CapabilityGrant lifecycle; strict real-time boundary
  separation.
- **Remaining gap**: subject abstraction and continuity are out of scope
  by design; WABIX intentionally avoids ROS's deployment complexity.

### Viam

- **Source**: viam.com; github.com/viamrobotics
- **Purpose**: commercial robotics platform: component (motor/camera/arm)
  abstraction over gRPC, cloud-managed fleet, modular resources.
- **Architecture**: gRPC component APIs + module system; client SDKs in
  many languages.
- **Maturity**: commercial, funded, production customers.
- **Overlap**: typed component APIs for heterogeneous hardware — same
  engineering instinct as WABIX capability manifests.
- **Differences**: closed platform core; no subject persistence; cloud-
  management-first (conflicts with our no-paid-cloud constraint).
- **Lessons to reuse**: component/resource naming taxonomy; the observation
  that modular hardware registries age well.
- **Remaining gap**: subjects, ALife, continuity — out of scope for Viam.

### Model Context Protocol (MCP)

- **Source**: modelcontextprotocol.io (Anthropic, 2024)
- **Purpose**: standard protocol connecting LLM applications to tools and
  context (servers expose tools/resources/prompts).
- **Architecture**: JSON-RPC over stdio/HTTP; typed tool schemas; capability
  negotiation at initialization.
- **Maturity**: rapid industry adoption; de-facto standard for tool-calling.
- **Overlap**: capability negotiation + typed tool invocation is exactly
  the WABIX action-validation instinct; MCP servers for devices exist.
- **Differences**: MCP is LLM-application plumbing — it has no subject, no
  world semantics, no observation metadata (uncertainty/frames/freshness),
  no resource economics. WABIX actions carry world-physics metadata and
  validation ranges; MCP tools are schemas for API calls.
- **Lessons to reuse**: initialization-time capability negotiation pattern;
  strict schema-typed parameters (already mirrored in our WABIX validation).
- **Remaining gap**: everything world-and-subject; but a future WABIX↔MCP
  adapter is plausible (both are schema-typed).

### "Autopoiesis of the artificial: from systems to cognition" (Ciucci/et al., 2023)

- **Source**: sciencedirect.com/science/article/pii/S0303264723001119
  (Biosystems)
- **Purpose**: connects autopoiesis theory to artificial systems and modern
  AI; asks what artificial cognition would need for organizational closure.
- **Architecture**: conceptual/theoretical; no implementation.
- **Maturity**: peer-reviewed theory paper.
- **Overlap**: theoretical foundation for the subject/organization framing;
  supports taking "artificial subject" seriously as a scientific object.
- **Differences**: no instrumentation proposal — it does not say how to
  *measure* continuity; that bridge is Transubstrate's engineering bet.
- **Lessons to reuse**: operational-definition discipline; the danger of
  anthropomorphic drift in terminology.
- **Remaining gap**: theory→measurement translation.

### Parfit — "Reasons and Persons" (1984)

- **Source**: Oxford University Press (book)
- **Purpose**: psychological-continuity theory of personal identity: identity
  consists in overlapping chains of psychological connectedness rather than
  a further fact.
- **Architecture**: philosophy.
- **Maturity**: canonical.
- **Overlap**: the philosophical grounding for our central claim — identity
  is not a UUID but (something like) connectedness of memory/character/goals.
  Our continuity vector is an operationalized, narrow-scope descendant of
  psychological-continuity criteria.
- **Differences**: Parfit's scope is persons and normative questions;
  Transubstrate measures narrow behavioral/memory proxies in artifacts and
  claims nothing about personhood.
- **Lessons to reuse**: connectedness-vs-continuity distinction (degrees of
  connectedness, not binary identity) maps directly onto our vector design.
- **Remaining gap**: the philosophical question stays open by design; we
  only instrument proxies.

---

## UNVERIFIED systems

The following were named in our early project framing but could not be
confirmed via web search to exist as described. We keep them listed so the
gap is explicit; if they exist under other names, a future revision will
fold them in.

- **Exuvia** (persistent agent identity/memory across model swaps):
  UNVERIFIED as a distinct named project. The *concept* — molting a model
  while keeping the agent — is represented in practice by Letta/MemGPT
  personas and by model-agnostic agent frameworks (LangGraph checkpointer
  etc.); no dedicated project under this name was found.
- **Enoch** (persistent/model-independent agent): UNVERIFIED. Closest real
  work: Generative Agents (memory persistence) and Letta (persona
  persistence).
- **OpenRGD**: UNVERIFIED. Possibly a confusion with OpenRGD/OpenRGD-like
  robotics data projects; no authoritative source found. Replaced in this
  survey by ROS2/Viam as the robotics-abstraction references.
- **Luis Arana / "semantic autopoiesis"**: UNVERIFIED as attributed; the
  published autopoiesis-of-the-artificial paper above covers the concept
  space.
- **OpenRAL**: UNVERIFIED in current form (an older RAL — Robot Abstraction
  Layer — exists in academic robotics, e.g. Rock/RAL lineage, but a specific
  "OpenRAL" project could not be confirmed as active). Treated via ROS2/Viam.

---

## Synthesis: contribution-by-contribution positioning

| Transubstrate contribution | Closest prior art | To the best of our current survey... |
|---|---|---|
| SUBJECT ≠ MODEL (subject as persistent abstraction, model as organ) | Letta/MemGPT personas; Generative Agents | Personas persist but are prompt+memory constructs; no system treats model replacement as a controlled experimental variable with the subject defined against it. **Narrow novelty: the abstraction is not new, the experimental treatment is.** |
| Measured continuity across controlled migrations | None directly; Parfit supplies theory, Letta supplies persistence | No system we found quantifies continuity across heterogeneous cognition swaps against controls. **This is our clearest open lane.** |
| WABIX world ABI | W3C WoT TD; MCP; ROS2/ros2_control; Viam | Discovery + typed actions exist in all four; WABIX's distinguishing moves are simulation-worlds-behind-the-same-ABI and physics-aware observation metadata. **Incremental over WoT/MCP; should seek TD-profile compatibility rather than a new format.** |
| Primordial ALife world with costly cognition, no semantic leakage | OpenLife; Lenia; Shachi | ALife+LLM exists; "thinking costs energy as an experimental variable" and channel-level semantic hygiene are not present in surveyed systems. **Narrow novelty on the economics + hygiene axis.** |
| Shared cognition model (VRAM ∌ organism count) | Generative Agents (per-agent calls, shared model implicitly) | LLM ABMs already share one model process; the contribution is making the cost *explicit, measured, and cadence-limited*, not the sharing itself. **Engineering discipline, not novelty — document honestly.** |
| Deterministic replay from event logs | (General software practice; no ALife/agent system surveyed commits to it) | Generative Agents/Shachi runs are not bit-replayable. **A real differentiator if maintained.** |

## Survey gaps (things we have not yet examined)

- Open-ended evolution literature in depth (Stanley/Lehman novelty search,
  minimal criteria) — relevant to our extinction/selection experiments.
- Voyager/AgentVerse/AutoGen-style agent frameworks — likely redundant with
  Shachi/Generative Agents for our purposes but should be skimmed for
  memory designs.
- Sim-to-real / domain randomization — deferred until WABIX physical work.
- Model-agnostic agent checkpoints (LangGraph etc.) — worth one pass for
  migration-mechanics ideas.
