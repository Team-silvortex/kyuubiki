# Operator SDK

This document defines the intended extension contract for new operator
capabilities in the `tamamono 1.x` line.

The goal is not to create a plugin marketplace first. The goal is to stop
adding new operators as one-off vertical slices and instead give them a stable
shape across:

- solver/runtime execution
- protocol exposure
- orchestration
- workflow composition
- validation

## Why this exists

Kyuubiki now has enough operator breadth that adding one more solver family is
not the hard part anymore.

The hard part is making sure each new capability can:

- describe itself clearly
- run through a stable runtime boundary
- emit predictable progress and results
- compose with other operators
- carry a verification story from the start

That is what the operator SDK is for.

## Design goals

An operator added through the Kyuubiki SDK should be:

- explicit
  no hidden assumptions about domain, family, or runtime mode
- composable
  outputs should be usable as workflow inputs
- headless-first
  operator definition should work before any specific UI exists
- validation-aware
  benchmark and smoke expectations should attach to the operator family early
- transport-stable
  protocol/runtime boundaries should not be re-invented per operator

## Core operator contract

Every operator should declare the following metadata:

- `id`
  stable unique operator identifier such as `solve.frame_3d`
- `version`
  operator-contract version, not just the repo release version
- `domain`
  `mechanical`, `thermal`, `thermo_mechanical`, or a future explicit domain
- `family`
  a stable family identifier such as `frame_3d`, `heat_plane_quad_2d`
- `kind`
  one of:
  - `solver`
  - `transform`
  - `extract`
  - `export`
  - `workflow_bridge`
- `summary`
  short human-facing description
- `capability_tags`
  stable tags such as `sparse`, `structured_mesh`, `result_projection`,
  `temperature_field`, `headless_safe`
- `input_schema`
  structured input contract reference
- `output_schema`
  structured output contract reference

For multi-operator chains, these schema refs should also be reusable from a
workflow dataset contract so that cross-operator intermediate values are not
described only by ad-hoc JSON convention. See
[workflow-dataset.md](/Users/Shared/chroot/dev/kyuubiki/docs/workflow-dataset.md).

## Runtime contract

Each operator should expose one stable run interface:

- `input`
  typed payload matching the declared input schema
- `context`
  runtime context such as:
  - current project/model identity
  - execution mode
  - orchestrated vs direct-mesh
  - artifact/cache policy
- `progress`
  typed progress events rather than free-form strings
- `result`
  typed summary payload plus optional chunked detail payloads
- `error`
  typed failure model with operator-stable error codes

That gives every operator the same broad lifecycle:

1. validate input
2. advertise execution intent
3. emit progress
4. produce typed result
5. optionally publish chunked result details

## Operator classes

The first useful SDK split is by operator class.

### `solver`

Examples:

- `solve.axial_bar_1d`
- `solve.frame_3d`
- `solve.thermal_plane_quad_2d`

These consume a study/model payload and produce result summaries plus detailed
element/node fields.

### `transform`

Examples:

- mesh generation from a higher-level model recipe
- result projection from heat into thermo-mechanical input
- model normalization

These consume one typed artifact and produce another typed artifact.

### `extract`

Examples:

- hotspot extraction
- result-table extraction
- selected-field summary extraction

These should be cheap, stable, and workflow-friendly.

### `export`

Examples:

- JSON export
- CSV export
- future report/package exports

These should be treated as operators too, not as UI-only side effects.

### `workflow_bridge`

Examples:

- `heat -> thermo_mechanical`
- future chained post-processing bridges

These should declare both their upstream accepted output shape and downstream
produced input shape.

## Minimum validation payload

A new operator should not be treated as real support until it has:

- a formal family baseline in
  [accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)
- at least one official sample or fixture
- at least one orchestrated or direct-mesh smoke path
- a stable result summary shape

The SDK should make this visible, not leave it implied.

Suggested validation metadata per operator family:

- `baseline_status`
  `verified`, `partial`, or `unverified`
- `baseline_cases`
  named benchmark/sample references
- `smoke_paths`
  orchestrated, direct-mesh, UI, or workflow paths
- `tolerance_profile`
  pointer to family-level tolerance expectations

## Packaging model

The SDK should support three operator origins:

- `built_in`
  ships in the repository and runtime by default
- `external_local`
  added from a local bundle, workspace, or extension path
- `external_remote`
  discovered through a trusted remote catalog or runtime registry

`tamamono 1.x` should prioritize `built_in` and `external_local` first.

Remote operator discovery should only be added when:

- trust and signing rules are explicit
- capability negotiation is stable
- failure modes are understandable

## Suggested Rust-side shape

The operator SDK likely wants a Rust-first representation because the runtime
contract already lives closest to the Rust data plane.

An eventual shape could look like:

- `OperatorDescriptor`
- `OperatorInputSchemaRef`
- `OperatorOutputSchemaRef`
- `OperatorRunRequest`
- `OperatorRunResult`
- `OperatorProgressEvent`
- `OperatorError`

That does not mean every operator must execute in Rust forever. It means the
contract should be runtime-neutral and transportable.

## Suggested protocol shape

The public protocol should grow toward:

- `describe_operator`
- `list_operators`
- `run_operator`
- `cancel_operator_run`

This can live alongside the current explicit solver RPC methods while the SDK
is still maturing.

`tamamono 1.x` does not need to delete the current explicit methods first.
It needs to stop painting itself into a corner when operator count grows.

## First 1.x priorities

The operator SDK should first support families that already have strong
verification momentum:

1. `frame_3d`
2. `thermal_frame_3d`
3. `thermal_truss_2d / 3d`
4. `heat_plane_triangle_2d / quad_2d`
5. `spring_1d / 2d / 3d`

These are good seed families because they already have:

- sample-backed orchestrated smoke
- formal solver baselines
- a clear result summary shape

## Relationship to workflows

This SDK is only half the story.

The operator contract defines how one capability behaves.
The workflow graph defines how multiple capabilities compose.

Use:

- [workflow-graph.md](/Users/Shared/chroot/dev/kyuubiki/docs/workflow-graph.md)

when the question becomes:

- how outputs connect to downstream inputs
- how caching and intermediate artifacts work
- how graph execution should fail, resume, and report state
