# Workflow Dataset Contract

This document defines the storage philosophy for workflow-carried data in
`tamamono 1.x`.

The goal is intentionally close to the ONNX idea:

- operators should not pass ad-hoc JSON blobs forever
- workflow edges should point at named data values
- those values should carry stable type, shape, and semantic metadata
- the same contract should survive solver/runtime/UI boundaries

## Why this exists

The workflow graph already gives Kyuubiki:

- nodes
- edges
- typed artifact categories such as `result/thermal_plane_quad_2d`

That is useful, but still too coarse for long-lived multi-operator workflows.

For example, these are very different things even when they both look like
`json_object` on the wire:

- a heat study model
- a temperature field result
- a thermo-mechanical seed model
- an extracted summary table

Without a dataset contract, those distinctions live only in convention.

## Design philosophy

The workflow dataset contract should do for multi-operator payloads what ONNX
value info does for tensors and graph edges:

- name the value
- classify the value
- describe its shape
- describe its encoding
- attach its semantic schema

This lets different operators agree on the same intermediate artifact without
being compiled as one vertical slice.

## Core entities

### `workflow_dataset_contract`

One portable dataset contract attached to a workflow definition or stored as its
own artifact.

It should declare:

- `schema_version`
- `id`
- `version`
- `values`
- optional `metadata`

### `value`

A named dataset value such as:

- `heat_model`
- `heat_result`
- `temperature_field`
- `thermo_model`
- `thermo_result`
- `summary_json`

Each value should declare:

- `id`
- `data_class`
- `element_type`
- `shape`
- `semantic_type`
- optional `unit`
- optional `encoding`
- optional `schema_ref`

## `data_class`

This should stay small and stable.

The first useful set is:

- `study_model`
- `result`
- `field`
- `table`
- `report`
- `export`
- `scalar`
- `metadata`

This is how a workflow can distinguish “a solver result” from “a summary table”
without overloading one string field.

## `element_type`

This says what a single stored element looks like.

Examples:

- `json_object`
- `f64`
- `i64`
- `string`

For now, most current Kyuubiki workflow values will stay `json_object`.
That is fine. The contract still adds value because the meaning and shape are
no longer implicit.

## `shape`

Shape should be present even when it is sparse or partly unknown.

It is defined as an ordered list of axes, each with:

- `id`
- optional `label`
- optional `size`
- optional `semantic`

Examples:

- `elements`
- `nodes`
- `components`
- `time_steps`
- `load_cases`

This is the part that makes the philosophy feel ONNX-like rather than “just
another JSON envelope”.

## `semantic_type`

This is the stable cross-operator identity of the value.

Examples:

- `study_model/heat_plane_quad_2d`
- `result/heat_plane_quad_2d`
- `study_model/thermal_plane_quad_2d`
- `result/thermal_plane_quad_2d`
- `field/temperature`
- `export/json`

`semantic_type` is what tells downstream operators what they are consuming.

## `schema_ref`

When the value corresponds to an existing operator IO contract, it should point
at it explicitly:

- input schema of `solve.heat_plane_quad_2d`
- output schema of `solve.thermal_plane_quad_2d`
- future extract/export schemas

This is what turns the dataset layer into a real bridge across operators
instead of a second parallel type system.

## Relationship to workflow graph

The workflow graph should continue to describe:

- topology
- node kinds
- port wiring

The dataset contract should describe:

- what concrete data values travel through that wiring

That means:

- `WorkflowGraph.dataset_contract` declares the portable value catalog
- each port may reference one `dataset_value`
- each edge may also reference the same `dataset_value`

So the graph says:

- “node A output connects to node B input”

and the dataset contract says:

- “that connection carries `thermo_result`, which is a
  `result/thermal_plane_quad_2d` value encoded as JSON and governed by this
  schema”

## First reference workflow

The first built-in reference contract is:

- `dataset.heat_to_thermo_quad/v1`

It backs:

- `workflow.heat-to-thermo-quad-2d`

and names the intermediate values:

- `heat_model`
- `heat_result`
- `thermo_model`
- `thermo_result`

This is intentionally small, but it is the first real proof that workflow
storage can be made portable and cross-operator instead of remaining
hard-coded.

## Storage shape

The first stable schema lives in:

- [workflow-dataset.schema.json](../schemas/workflow-dataset.schema.json)
- [examples.workflow-dataset.json](../schemas/examples.workflow-dataset.json)

## Material Score Value

Material screening workflows use `transform.score_material_candidates` to
produce a report-style dataset value. Its cross-runtime shape is pinned in
[material-score-contract.md](material-score-contract.md).

That contract is deliberately more specific than a generic `report/json`
dataset value because downstream experiment planning, reports, SDK clients, and
agent-native execution all depend on the same fields:

- `material_score_policy`
- `material_score_ranges`
- `material_score_rankings`

Workflow dataset entries that carry material scoring output should use a
semantic type such as `report/material_score` and point their schema reference
or documentation link at that contract.

The first runtime validation layer now also exists in the Rust engine:

- `WorkflowGraph.dataset_contract` is checked against workflow JSON security
  budgets before semantic validation
- dataset contract `id` and `version` must be non-empty
- dataset value ids inside one contract must be non-empty and unique
- `data_class` must stay inside the stable workflow dataset class set
- `element_type`, `schema_ref`, shape axis ids, and optional shape axis text
  fields must not be empty
- shape axis ids must be unique inside each dataset value
- ports that reference `dataset_value` must resolve inside the contract
- edges cannot disagree with connected ports about the named dataset value
- `artifact_type` and dataset `semantic_type` must stay aligned

It should be possible to store this contract:

- inside a workflow definition
- alongside a workflow definition
- or inside future workflow catalog and project bundle artifacts

## `tamamono 1.x` priorities

The current line should keep this contract narrow and useful:

1. name cross-operator values
2. make port/edge contracts explicit
3. reuse existing operator schemas where possible
4. avoid inventing a giant new IR before the workflow runtime needs it

The right first outcome is not “become ONNX”.
The right first outcome is “adopt the same discipline ONNX brought to graph
data: explicit values, explicit shapes, explicit semantics, and transportable
contracts”.
