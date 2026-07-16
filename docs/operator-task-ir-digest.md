# Operator task IR digest rules

Operator task IR uses SHA-256 digests so orchestrators, SDKs, and Rust agents
can detect descriptor or task-envelope changes before execution.

The reference implementations currently live in:

- Elixir control plane: `KyuubikiWeb.CanonicalJson` and
  `KyuubikiWeb.Orchestra.OperatorTaskIR`
- Rust data plane: `kyuubiki_protocol::canonical_json` and
  `kyuubiki_protocol::compute_operator_task_digest`

Both sides must keep the golden fixture below passing before changing TaskIR
field coverage.

## Digest algorithm

Both `descriptor_digest` and `task_digest` use:

1. Select the digest input fields.
2. Encode the selected JSON value using Kyuubiki canonical JSON.
3. Hash the UTF-8 bytes with SHA-256.
4. Encode the digest as lowercase hexadecimal.

## Canonical JSON

Canonical JSON is intentionally small and JSON-first:

- object keys are sorted lexicographically by their string form
- arrays keep their original order
- strings, numbers, booleans, and null use standard JSON encoding
- no whitespace is inserted
- finite floating-point numbers use fixed decimal notation with up to 15
  fractional digits, compacted by trimming trailing zeroes while preserving at
  least one fractional digit

Example:

```json
{"z":1,"a":{"b":true,"a":null},"list":[{"y":2,"x":1}]}
```

canonicalizes to:

```json
{"a":{"a":null,"b":true},"list":[{"x":1,"y":2}],"z":1}
```

Float examples:

- `160.0` -> `160.0`
- `1.2e-5` -> `0.000012`
- `7.0e10` -> `70000000000.0`

## Descriptor digest

`descriptor_digest` covers the operator snapshot carried in task IR.

It is intentionally narrower than the full task envelope and answers:

> Which operator descriptor did this task use?

## Task digest

`task_digest` covers the execution envelope fields listed in
`integrity.task_digest_fields`.

Current fields:

- `schema_version`
- `task_id`
- `operator`
- `descriptor_authoring`
- `node`
- `input_artifact`
- `config`
- `execution_program`
- `dataset_contract`
- `orchestration_context`
- `runtime_hints`

`integrity` itself is not included, avoiding self-referential digests.

## Mirror constraints

TaskIR intentionally repeats a few identity fields because different runtimes
need them at different layers: descriptor authoring, execution dispatch, and
agent package fetch. Repetition is allowed only when the values mirror each
other:

- `operator.kind` mirrors `execution_program.program_kind`
- `operator.kind` mirrors `execution_program.entrypoint.operator_kind`
- `operator.kind` mirrors `runtime_hints.operator_kind`
- `operator.execution.package_ref` mirrors `execution_program.package_ref`
- `execution_program.package_ref` mirrors `runtime_hints.package_ref`
- `execution_program.package_version` mirrors `runtime_hints.package_version`

The Rust protocol layer rejects digest-valid tasks when these mirrors disagree.
The JSON schema carries the same rule as `x-kyuubiki-mirror_constraints`, and
`make check-operator-task-ir-contract` keeps the schema extension, mirror
constraints, digest field order, and example `descriptor_digest` /
`task_digest` values aligned.

## Execution preview

TaskIR also has a Rust protocol preview helper:

- `kyuubiki_protocol::preview_operator_task_execution`

The preview does not run the task. It translates the language-neutral task
envelope into fields that agents, orchestra, Installer preflight, Workbench,
and headless SDKs can all inspect before dispatch:

- dispatch route: solver RPC, local operator task, or fetch-package-then-run
- package reference and package version
- whether package fetch is required
- which readiness gate should apply
- result serialization encoding
- authority mode, execution mode, cache scope, and agent fetchability
- whether the task can run offline
- warnings for incomplete centralized or remote dispatch hints

This keeps package fetch, readiness, dispatch, and result serialization visible
without making the GUI, Elixir control plane, or any one SDK the owner of the
executable task semantics.

## Golden fixtures

The basic cross-language fixture used by tests is:

- operator id: `transform.fixture`
- family: `fixture`
- kind: `transform`
- package ref: `orchestra://operator-package/transform.fixture`
- input: `{"x":1}`
- config: `{"alpha":true}`
- task id: `fixture-task`
- authoring mode: `rust_native`

Expected digests:

- descriptor digest:
  `b397ef3b203a0500a29aabe507868b4104ddf22faee5015df69cc0486ac35cd2`
- task digest:
  `86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f`

The float-heavy fixture protects fixed-decimal canonicalization:

- operator id: `transform.fractional_fixture`
- family: `fixture`
- kind: `transform`
- package ref: `orchestra://operator-package/transform.fractional_fixture`
- input:
  `{"temperature_delta":160.5,"thermal_expansion":0.000012,"poisson_ratio":0.33}`
- config: `{"constraint_factor":0.7}`
- task id: `fractional-fixture-task`
- authoring mode: `rust_native`

Expected digests:

- descriptor digest:
  `5473bf5ee7f3f01eaaf596c83f0ded9cf07c67c45277f6f77cd1cedc2f553d10`
- task digest:
  `77e991c9a4023bbb7c486ad3efc3726381e57671d0d33a4ae22cc6dbc0c849da`

The Elixir-authored fixture protects the hot authoring path while keeping the
agent-facing task language-neutral:

- operator id: `transform.elixir_quality_gate`
- family: `quality_gate`
- kind: `transform`
- package ref: `orchestra://operator-package/transform.elixir_quality_gate`
- input:
  `{"candidate_id":"candidate-a","score":0.82,"minimum_score":0.75}`
- config: `{"decision_key":"screening_pass","emit_reason":true}`
- task id: `elixir-control-plane-quality-gate-task`
- authoring mode: `elixir_control_plane`
- execution language: `language_neutral`

Expected digests:

- descriptor digest:
  `049f60146572e1611b407e2e5e1b9970fb9b406de8a0e2eb072ca436555d687c`
- task digest:
  `aa8334325e6eb3abed205450cf0233d39dfc563c4078c208f30c5e0ddf79da9d`

`make check-operator-task-ir-contract` now requires both `rust_native` and
`elixir_control_plane` TaskIR fixtures so the executable task surface cannot
silently drift into a single authoring runtime.

The same gate reads `schemas/operator-task-ir-golden-manifest.json`. That
manifest records release-line golden coverage by example path, task id,
descriptor authoring mode, operator kind, execution program kind, and runtime
execution mode. Changing an example now requires changing the declared coverage
manifest instead of silently weakening what TaskIR compatibility proves.
