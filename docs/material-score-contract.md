# Material Score Contract

This document pins the current cross-runtime result contract for
`transform.score_material_candidates`.

The operator exists in two execution paths:

- Elixir workflow/runtime execution inside the control plane
- Rust agent-native TaskIR execution for headless agent dispatch

Both paths must keep the same externally visible result fields so that UI,
reports, workflow templates, and SDK clients do not need runtime-specific
parsers.

## Input Shape

The input payload should contain candidate summaries:

- `candidates`
  object keyed by candidate id
- each candidate summary
  object containing numeric metric fields referenced by criteria

The config should contain:

- `criteria` or `objectives`
  non-empty list of scoring criteria
- each criterion:
  - `field`
    numeric metric path
  - `goal`
    `min` or `max`
  - `weight`
    positive number, default `1.0`

Optional policy fields:

- `status_field`
  candidate summary field used for feasibility, default `material_status`
- `feasible_status`
  accepted status value, default `pass`
- `infeasible_penalty`
  non-negative penalty subtracted from infeasible candidates, default `1.0`
- `include_candidate_summary`
  include original candidate summary in ranking entries when `true`

## Result Shape

The result must include:

- `material_score_candidate_count`
- `material_score_feasible_count`
- `material_score_best_candidate_id`
- `material_score_best_score`
- `material_score_criteria`
- `material_score_policy`
- `material_score_ranges`
- `material_score_rankings`

`material_score_policy` records the scoring context used to reproduce the
ranking:

- `total_weight`
- `infeasible_penalty`
- `status_field`
- `feasible_status`

`material_score_ranges` maps each criterion field to:

- `min`
- `max`

These ranges are the normalization basis for each criterion. They must be
present even when the UI does not show them directly, because reports and SDK
clients need them to explain why a candidate scored as it did.

Each ranking entry includes:

- `candidate_id`
- `feasible`
- `weighted_score`
- `final_score`
- `metrics`
- `criteria_breakdown`

Each criteria breakdown entry includes:

- `field`
- `goal`
- `weight`
- `actual`
- `normalized_score`
- `weighted_score`

## Scoring Semantics

For each criterion, values are normalized over all candidate metrics:

- `goal = max`
  `(value - min) / (max - min)`
- `goal = min`
  `(max - value) / (max - min)`
- when `min == max`
  normalized score is `1.0`

Candidate `weighted_score` is:

`sum(criteria_breakdown.weighted_score) / material_score_policy.total_weight`

Candidate `final_score` is:

- `weighted_score` when feasible
- `weighted_score - infeasible_penalty` when infeasible

Rankings are sorted by descending `final_score`, then descending
`weighted_score`, then ascending `candidate_id`.

## Error Codes

Runtime implementations should keep these failures stable:

- `missing_material_candidates`
- `missing_material_score_criteria`
- `invalid_material_score_criterion`
- `missing_material_score_metric`
- `invalid_material_score_policy`

`invalid_material_score_policy` is currently used when
`infeasible_penalty < 0.0`.

## Runtime Parity

The Elixir control-plane workflow runtime and the Rust agent-native TaskIR
runtime are expected to agree on:

- accepted criteria fields and goals
- non-negative infeasible penalty
- custom `status_field` and `feasible_status`
- range output
- policy output
- ranking sort order

Tests that protect this parity currently live in:

- `apps/web/test/kyuubiki_web/workflow_material_runtime_test.exs`
- `workers/rust/crates/cli/src/tests/operator_task_ir_rpc/agent_native_material.rs`
