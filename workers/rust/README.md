# Rust Workspace

`workers/rust` is the Kyuubiki data-plane workspace.

It currently contains:

- `crates/protocol`
  Shared RPC messages, job/result payloads, agent-wire contracts, and
  language-neutral operator TaskIR digest helpers.
- `crates/operator-sdk`
  Author-facing Rust traits and registry helpers for building workflow
  operators against the shared protocol contract, including external-local
  manifest discovery, cross-platform load-plan resolution, and activation
  hooks.
- `templates/operator-crate-template`
  Copyable starter crate for external-local Rust operators built on top of the
  author-facing operator SDK, including a `cdylib` export symbol for runtime
  loading tests and host integration.
- `crates/engine`
  Engine-facing solve entry points and chunk helpers that sit above raw solver
  kernels.
- `crates/solver`
  FEM kernels, sparse/dense solve paths, and numerical utilities.
- `crates/cli`
  TCP solver agent, local runtime entry point, and remote self-registration
  behavior. The `kyuubiki-headless` binary also exposes workflow
  template/init/inspect/validate/plan/run flows for service-only, browser-only,
  and hybrid headless automation. `plan` emits executor compatibility for
  `mock`, `service`, and `hybrid` before a live run is attempted. Guarded
  `--parameter-patch` inputs let every command observe the same atomic,
  baseline-checked research-round changes without shell rewriting. Research
  runs can also emit a contiguous `--research-round-out` evidence chain that
  rejects repeated inputs, missing prior rounds, mock execution, and `n/a`
  metrics. The
  `kyuubiki-material-report` binary turns material screening result payloads
  into ranked research reports.
- `crates/headless-sdk`
  Rust-first headless workflow templates, execution plans, dry-run support, and
  concrete R&D examples such as `material_heat_spreader_screening` for thermal
  material candidate comparison.
- `crates/deploy-server`
  Read-only Rust download/deployment metadata server for update catalogs,
  deploy descriptors, and artifact file distribution.
- `crates/benchmark`
  Benchmark profiles for medium, large, v2, and `10k`-plus scale targets up to
  `100k`, with `10k` now treated as the default local regression tier.
- `crates/installer`
  Cross-platform installer/deployment CLI reused by the Tauri installer GUI.

This workspace should remain frontend-agnostic and Phoenix-agnostic. The Rust
side is the reusable computation/runtime layer, not the control plane.

Crate-level test expectations stay close to the code:

- `crates/solver/tests/`
  solver behavior, numerical baselines, and progress/result validation
- other crate-local `tests/`
  engine, CLI agent, benchmark, and installer/runtime checks

Prefer colocated crate tests so runtime behavior stays near the code it
validates.

Native sweep scoring and quality mapping controls live in
`crates/engine/src/tests/workflow_sweep_*`. Scoring shares 25 JSON fixtures with
Elixir; graph controls cover blocked selection and explicit fresh-run recovery.
For positive objective scores, mapping uses an explicitly calibrated fixed
`score_reference`, not a per-batch offset. See the
[mapping contract and evidence](../../reports/native-sweep-selection-recovery-20261012.md)
for the scale policy, commands and limits. Run the complete Engine library suite
with `cargo test --locked --offline -p kyuubiki-engine --lib` in this workspace.

Collection/summary controls additionally share 41 fixtures with Elixir and check
final convergence before metric projection, explicit stale-summary cleanup and
provenance separation. Their fail/skip graph controls are explicit fresh-run
recovery, not durable restart or live Agent evidence. See the
[collection contract follow-up](../../reports/native-sweep-collection-recovery-20261012.md).

Quality composition/ranking/next-round controls share 49 fixture cases with
Elixir and add thirty controlled native graphs. Malformed inputs reject, partial
evaluation pauses expansion, and valid quality blockers retain repair planning.
See the [quality-decision audit](../../reports/native-quality-decision-recovery-20261012.md)
for admission rules, readiness thresholds and explicit fresh-run recovery limits.

Ranking integrity adds 31 shared cases, permutation/JSON-exchange controls and
sixteen native graph executions. Next-round input must already have canonical
ready/score/identity order, consistent rank positions and matching score copies.
It is not silently repaired. See the
[ranking consistency follow-up](../../reports/quality-ranking-consistency-recovery-20261012.md).

Coupled-readiness admission rejects conflicting aliases and malformed nested
reports. False readiness and blocked/missing domains cannot become a successful
stop; warning evidence keeps review active. See the
[coupled readiness recovery checks](../../reports/coupled-readiness-admission-recovery-20261012.md)
for 56 shared cases, alias-placement controls and explicit fresh-run limits.

Direct quality-sweep requests now check four readiness aliases and duplicate
hints before stopped/incomplete branches. Blocked evidence requires explicit
repair intent, while rejected candidates disable planning. See the
[sweep request recovery checks](../../reports/quality-sweep-request-admission-recovery-20261012.md)
for 74 shared cases, 625 alias placements and fail/skip recovery controls.

Existing plans entering quality materialization now validate execution intent,
repair metadata and declared axis counts using the same admission rules. See the
[direct plan recovery checks](../../reports/quality-sweep-plan-admission-recovery-20261012.md)
for shared cases, generated-plan mutations, fresh recovery and deferred-budget
limits.

Direct expansion now admits supplied quality case metadata before generating
cases, including raw callers that bypass materialization. Metadata aliases cannot
erase repair evidence; generic non-quality metadata keeps its prior behavior.
See the [expansion recovery checks](../../reports/quality-sweep-expansion-admission-recovery-20261012.md)
for 70 shared cases, 125 alias placements and corrected fresh task exports.

Expansion now checks axis counts, deferred declarations and count mirrors before
stopped or blocked returns, using the materializer's shared native counter. See
the [count consistency checks](../../reports/sweep-expansion-count-recovery-20261012.md)
for 85 shared cases, 125 count placements and 32 controlled graphs per runtime.
Caller-authored execution batches and producer-side readiness retention remain
separate audits; no new scale or physical qualification is claimed.

For the currently retained thermal-quad matrix optimization set and its
benchmark evidence, also read:

- `../../docs/solver-matrix-optimization-pack.md`

## Layout skeletons

- `linux/`
  repository-visible Linux install-layout skeleton used to keep expected
  runtime paths explicit even when no packaged output is committed yet
