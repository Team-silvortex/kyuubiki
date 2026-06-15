# Workflow Graph

This document defines the intended multi-operator workflow model for
`tamamono 1.x`.

The target is closer to a shader-style graph than to a pile of disconnected
buttons:

- operators become nodes
- typed outputs connect to typed inputs
- intermediate artifacts become explicit
- execution state becomes inspectable

The first goal is not a full visual node editor.
The first goal is a stable workflow model that can run headlessly, survive
automation, and later gain a richer UI.

## Why this exists

Kyuubiki now has enough operator depth that useful work increasingly spans more
than one solve call.

Examples already visible in the product direction:

- `heat -> thermo_mechanical`
- `generate -> solve -> inspect -> export`
- `solve -> extract -> export`
- `library import -> inspect -> open`

Without a workflow model, these chains remain:

- hard-coded
- UI-specific
- difficult to validate end-to-end

## Core workflow concept

A workflow should be a typed graph of nodes and edges.

Each node is an operator invocation or workflow utility step.
Each edge passes a typed artifact from one node to another.

Minimum graph entities:

- `workflow`
  top-level graph descriptor
- `node`
  one operator or utility step
- `port`
  typed input/output attachment point
- `edge`
  typed connection between ports
- `artifact`
  persisted or ephemeral intermediate value
- `run`
  one execution of a graph

## Node kinds

The first useful node kinds are:

- `solve`
  direct study/operator execution
- `transform`
  convert one typed artifact into another
- `extract`
  derive smaller structured views from larger results
- `export`
  turn typed artifacts into user-facing deliverables
- `condition`
  optional branching or guard logic
- `input`
  explicit graph entrypoint
- `output`
  explicit graph exitpoint

`tamamono 1.x` should probably avoid a large branching/control-flow language
at first. Linear and lightly branched graphs are enough to start.

## Typed ports

Ports are the whole point.

Every edge should connect only compatible port types.

Examples of useful artifact types:

- `study_model/frame_3d`
- `study_model/thermal_frame_3d`
- `result/frame_3d`
- `result/heat_plane_quad_2d`
- `field/temperature`
- `field/stress`
- `report/table`
- `export/csv`
- `export/json`

This is how the system avoids “just trust me” wiring.

The graph now also has a first-class dataset-contract slot so that ports and
edges can refer to named cross-operator values, not only loose artifact-type
strings. That is the current line's ONNX-like layer:

- the graph still carries topology
- the dataset contract carries value semantics

The graph now also has a first-class execution-hint layer so that distributed
automation does not need to infer scheduling intent from hidden state:

- `dispatch_policy`
  graph-level execution mode hint such as `central_fetch` or `direct_mesh`
- `operator_fetch_plan`
  explicit operator package-fetch metadata per runnable node
- `placement_tags`
  graph-level or node-level placement intent
- `required_capabilities`
  graph-level or node-level runtime capability contract

See [workflow-dataset.md](workflow-dataset.md).

## First workflow targets

The earliest graphs should be small and high-value.

### `heat -> thermo_mechanical`

This is the clearest seed workflow.

1. solve heat operator
2. extract temperature field
3. map field into thermo-mechanical input
4. solve thermo-mechanical operator
5. expose structural result summary

This should become the reference workflow for:

- field projection
- intermediate artifact persistence
- bridge validation

The first headless reference runner now exists in the Rust engine for:

- `heat_plane_quad_2d -> bridge.temperature_field_to_thermo_quad_2d -> thermal_plane_quad_2d`

That means this workflow is no longer only a UI direction:

- the graph shape has a portable schema
- the bridge step has a stable built-in operator identity
- the runtime can already execute the first linear reference path headlessly

The first generic executor layer now also exists for a small but real node set:

- `input`
- `solve`
- `transform`
- `extract`
- `output`

That executor now also supports a first export layer:

- `export.summary_json`
- `export.summary_csv`

That is enough to run both:

- `heat -> thermo_mechanical`
- `solve -> extract -> output`
- `solve -> extract -> export`

The orchestrator now exposes two workflow graph entrypoints:

- `POST /api/v1/workflows/graph/run`
  synchronous reference execution
- `POST /api/v1/workflows/graph/jobs`
  asynchronous job submission that reuses the standard
  `GET /api/v1/jobs/:job_id` polling path

The orchestrator now also exposes a first workflow catalog layer:

- `GET /api/v1/workflows/catalog`
  list built-in named workflows and their entry/output contracts
- `GET /api/v1/workflows/catalog/:workflow_id`
  inspect one named workflow descriptor including the backing graph
- `POST /api/v1/workflows/catalog/:workflow_id/jobs`
  submit a workflow job without embedding the full graph JSON in the client

The asynchronous job path now also exposes lightweight workflow runtime state
through the normal job/result payload:

- `current_node`
- `completed_nodes`
- `progress_events`

### `generate -> solve -> inspect -> export`

1. generate parametric model
2. solve selected operator
3. extract summary/result slices
4. export JSON or CSV

This is likely the best second workflow because it ties together generation,
runtime execution, and result handling.

### `solve -> extract -> export`

1. solve operator
2. extract hotspots or result table
3. export concise artifact

This is the simplest workflow and should become the easiest headless example.

## Built-in workflow catalog

The first built-in workflow descriptor is:

- `workflow.heat-to-thermo-quad-2d`

Its role is to make one reference multi-operator path discoverable and reusable
by Hub, Workbench, SDK callers, and automation without forcing every client to
ship the full graph inline.

## Workflow runtime contract

A workflow run should expose:

- `workflow_id`
- `run_id`
- `status`
  `queued`, `running`, `completed`, `failed`, `cancelled`
- `current_node`
- `completed_nodes`
- `artifacts`
- `progress_events`
- `failure`

This should feel familiar next to the current job runtime model.

## Caching and artifacts

The graph model should make intermediate artifacts explicit.

Each node run may produce:

- `ephemeral`
  used in-memory or for one run only
- `cached`
  safe to reuse if the same upstream inputs reappear
- `persisted`
  intentionally saved into project/workflow history

Useful first-class artifact metadata:

- `artifact_id`
- `producer_node`
- `artifact_type`
- `schema_ref`
- `created_at`
- `cache_key`
- `persistence_policy`

## Failure model

Workflow failure should be inspectable by node, not just by whole run.

Every failed run should say:

- which node failed
- which input artifact it consumed
- which operator/version was used
- whether downstream nodes were skipped or cancelled

This is one of the main reasons to prefer a real workflow model over chained UI
actions.

## Suggested storage shape

The graph should be storable as a portable project/workflow artifact.

Likely entities:

- `workflow_definition`
- `workflow_run`
- `workflow_node_run`
- `workflow_artifact`

This allows:

- replay
- comparison
- auditability
- CI-style regression workflows later

The first stable storage contract should live in:

- [workflow-graph.schema.json](../schemas/workflow-graph.schema.json)
- [examples.workflow-graph.json](../schemas/examples.workflow-graph.json)
- [workflow-dataset.schema.json](../schemas/workflow-dataset.schema.json)
- [examples.workflow-dataset.json](../schemas/examples.workflow-dataset.json)

## Suggested first headless API

Before a rich UI graph editor exists, the minimum useful API is:

- `create_workflow`
- `describe_workflow`
- `run_workflow`
- `get_workflow_run`
- `cancel_workflow_run`
- `list_workflow_artifacts`

This should be usable from:

- Rust tests
- Elixir orchestration
- SDK clients
- future Hub or Workbench workflow surfaces

## Relationship to the operator SDK

The workflow graph should not invent its own node contracts.

It should consume operator descriptors defined by:

- [operator-sdk.md](operator-sdk.md)

In short:

- operator SDK defines a node’s capabilities
- workflow graph defines how nodes connect and run together

The current execution-hint split is:

- graph-level fields
  broad orchestration intent and whole-run requirements
- `defaults`
  common node defaults for cache and placement/capability hints
- node-level fields
  precise per-node placement or capability overrides
- `operator_fetch_plan`
  concrete fetch provenance for operator binaries/packages

## 1.x delivery order

Suggested order for `tamamono 1.x`:

1. define operator descriptors and typed run contract
2. define minimal workflow schema
3. implement one headless linear workflow runner
4. ship `heat -> thermo_mechanical` as the first reference graph
5. add artifact caching and replay
6. only then design the richer graph UI

That order keeps Kyuubiki from building a beautiful graph editor on top of an
unclear execution model.
