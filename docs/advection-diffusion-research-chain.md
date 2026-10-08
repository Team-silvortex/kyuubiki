# Stabilized Transport Research Chain

Explicit [upwind transport](advection-diffusion-upwind.md) can feed a complete
solve -> diagnose -> score -> objective -> decision workflow. This path retains
artificial diffusion instead of treating a stable answer as a resolved answer.
The scope is steady 1D transport, not general CFD or material qualification.

## Operators and Ownership

Use these graph nodes in order:

1. `solve.advection_diffusion_bar_1d` with an input model declaring `scheme: upwind`.
2. `extract.transport_result_diagnostics` on the complete physical result.
3. `transform.score_transport_quality` on the diagnostic summary.
4. `transform.compose_quality_objective` with the quality summary as a named input.
5. An output node retaining the objective summary and `composite_quality_ready`.

A separate output may retain the unmodified solver result; an independent
branch may continue when diagnostics fail under explicit `on_error: skip`.
See `workers/rust/crates/engine/tests/support/diagnostic_chain.rs` for a typed
graph builder and `workers/rust/crates/cli/tests/support/headless_transport_research.rs`
for its real service execution regression.

In the service path, Orchestra owns scheduling, graph recovery and objective
composition. Transport diagnosis and quality scoring use the existing
language-neutral TaskIR RPC and execute through the Rust engine. The Agent
adapter selects an explicit diagnostic/scoring whitelist; it does not reimplement
their mathematics or admit arbitrary operator IDs. Task digest, operator kind,
node binding, parent-job cancellation and task-bound completion remain checked.
The catalog declares these engine built-ins as unbundled `agent_native` entries,
not downloadable replicas of the central operator library.

The same adapter now supports six result-diagnostic extracts: transport, thermal,
electrostatic, magnetostatic, thermo and Stokes flow. Its five quality scorers are
transport, thermal, electrostatic, magnetostatic and CFD; thermo uses the thermal
scorer with explicit thermo terms. All eleven IDs keep exact kind checks and
task-bound completion. Other workflow operations are not admitted by prefix.
See [native domain service verification](../reports/native-domain-diagnostic-chains-20261008.md)
for five real study chains, Stokes quad/triangle RPC alignment and standalone
diagnostic cancellation recovery. Study solves still use typed solver RPCs,
not additional TaskIR solver capabilities or full-graph Agent execution.

## Diagnostic Metrics

Default prefix `transport` produces these additional metrics:

| Base Metric | Meaning |
| --- | --- |
| `transport_artificial_diffusivity` | Nonnegative `abs(v) * h / 2` |
| `transport_stabilization_flux` | Signed artificial-diffusion flux |
| `transport_numerical_flux` | Signed conservative upstream numerical flux |

Each base metric has `_peak`, `_peak_magnitude` and, when supplied, `_peak_id`.
Peaks select the largest magnitude while retaining its sign; equal magnitudes
use the last record, consistently with existing physical peak extraction.
`output_prefix` customizes the metric prefix. `diagnostic_scheme` records the
declared scheme; `diagnostic_metric_groups` includes `stabilization` only when
that complete group has been measured.

Declared upwind results require all three finite fields in every element, a
nonnegative artificial diffusivity and a finite canonical physical `total_flux`.
Unknown/null schemes, partial groups, malformed records and contradictory flux
identities reject. Stabilization cannot silently select upwind for an undeclared
or Galerkin input. Legacy bare partial results without stabilization remain usable.

The local identity is `numerical_flux = total_flux + stabilization_flux`. Its
check normalizes by the largest magnitude of the three fields before summing,
with a `64 * f64::EPSILON` roundoff allowance and no unit-sized tolerance floor.
That avoids overflow and tiny-flux false acceptance. It checks record consistency,
not nodal balance, PDE accuracy, material validity or authenticated provenance.
Physical total flux is never replaced by numerical flux.

The additional extraction pass is linear, keeps three peak references and does
not clone the element array. Healthy samples do not allocate error-path strings.
No remote-scale performance claim follows from these bounded source tests.

## Opt-In Quality Policy

The original four default quality terms and their defaults remain unchanged.
Three extra terms are available through `enabled_terms`:

- `transport_artificial_diffusivity_peak`
- `transport_stabilization_flux_peak_magnitude`
- `transport_numerical_flux_peak_magnitude`

Their fallback target and weight are 1, and their goal is minimization. Those
fallbacks are not physical acceptance limits. Supply unit-consistent targets
and weights for the actual study; numerical flux is not universally a quantity
that should be minimized. Selected missing metrics block readiness rather than
becoming zero; malformed metrics reject, while an explicit measured zero is valid.

An illustrative mesh-diffusion profile is:

```json
{
  "enabled_terms": ["transport_artificial_diffusivity_peak"],
  "targets": {"transport_artificial_diffusivity_peak": 1.0},
  "weights": {"transport_artificial_diffusivity_peak": 1.0},
  "max_ready_score": 2.0
}
```

For a unit-length, unit-area model with diffusivity 1, endpoint concentrations
0/1 and `abs(v) = 20`, two uniform elements give artificial diffusivity 5;
eight give 1.25. This configured objective blocks the coarse example and accepts
the finer one, for both velocity signs and connectivity orientations. It is a
policy demonstration, not evidence that eight elements resolve the boundary
layer or that reducing numerical diffusion proves physical accuracy.

## Retained Research and Recovery

The native batch is `workflow_submit_graph -> job_wait -> result_fetch`, using
the service executor. Submission is sensitive and requires explicit approval.
A research metric may select:

```text
/steps/2/result_preview/result/artifacts/diagnose.summary/transport_artificial_diffusivity_peak
```

Use research objective `minimize`, not the quality-configuration term `min`.
Persist the original batch and run report together. Rebuilding research evidence
requires the report's execution-input fingerprint to match the submitted batch;
a coarse batch cannot claim a refined result. This example's metric unit `1`
describes its dimensionless illustrative model, not every physical transport study.

With `on_error: skip`, a corrupt diagnostic node is failed, its quality/objective
dependents are skipped, and raw/independent outputs remain available. A completed
workflow job or Headless `status: ok` then describes execution of the recovery
policy, not scientific readiness. Inspect `failed_nodes`, `skipped_nodes` and the
actual decision artifact before continuing research. A fresh healthy workflow
is explicit; failed calculations are not automatically replayed.

## Cooperative Diagnosis Cancellation

Borrowed-record diagnostic scans use the existing per-execution control token,
not a second watchdog or a frontend-owned cancellation mechanism. They expose
`result_diagnostics` at scan start, every 64 successful records and scan end,
including empty/short arrays and the stabilization pass. Counts are local to
each scan and may restart for the next metric group; they are not global progress
or a resumable snapshot. Existing numerical stage codes retain their values.

The native transport TaskIR failure receipt reports `reason_code: cancelled` and
`failure_stage: execute_diagnostics` with the original task identity. A rejected
or cancelled scan publishes no partial summary. Inspect the failure before an
explicit rerun; cancellation is not automatic retry permission. The original
failed attempt remains failed when a separate healthy attempt completes.
An already-detected invalid sample keeps its original error if cancellation
has not yet been observed at a safe point.

The same control helper now covers both independent Stokes diagnostic loops.
Engine regressions also check thermal, electric, magnetic and thermo-mechanical
vector scans plus signed stress-component fallback. An enclosing execution
scope rejects cancelled graph publication even with `on_error: skip`; internal
raw/independent artifacts are not a successful cancelled-call return. These
Engine tests do not establish non-transport native TaskIR routing or public
Orchestra graph-cancellation acceptance. See the
[cross-domain follow-up](../reports/cross-domain-diagnostic-cancellation-20261008.md).
The separate native domain service verification above now establishes routing
and standalone diagnostic cancellation for these five additional domains,
without qualifying public whole-job cancellation or durable continuation.

This covers shared `Samples::scan` reductions and Stokes loops, not every extraction operator,
JSON decode/clone/digest, serialization, plugin execution or thread preemption.
The 64-record polling cadence is not a wall-clock latency bound or a large-mesh
benchmark. See [diagnostic cancellation verification](../reports/diagnostic-cancellation-reliability-20261008.md).

## Protocol Boundaries

Typed Rust graphs omit unset optional authoring fields instead of emitting
schema-invalid `null` values. Explicit false values and configured zeros remain.
Typed dataset contracts always emit `kyuubiki.workflow-dataset/v1` and reject
missing/unsupported wire markers; sparse value/axis metadata omit unset fields.
Existing Rust dataset struct construction is unchanged. Stored JSON without the
required marker must be corrected, not silently upgraded into a valid contract.

Elixir TaskIR float encoding now matches Rust's exact fixed-15, ties-to-even
rounding, including signed zero. This corrects valid-task digest mismatches for
solver outputs without rounding or modifying the submitted physical data.
The legacy digest still rounds to 15 decimal places: tiny values can collide.
It is not the lossless research input fingerprint or an authenticated signature.

See the [verification report](../reports/advection-diffusion-research-chain-20261008.md).
