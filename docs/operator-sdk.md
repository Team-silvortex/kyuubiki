# Operator SDK

This document defines the intended extension contract for new operator
capabilities in the `moxi 2.x` line.

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
[workflow-dataset.md](workflow-dataset.md).

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
- `solve.solid_tetra_3d`
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
  [accuracy-baselines.md](accuracy-baselines.md)
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

`moxi 2.x` should prioritize `built_in` and `external_local` first.

Remote operator discovery should only be added when:

- trust and signing rules are explicit
- capability negotiation is stable
- failure modes are understandable

## Current Rust-side shape

The operator SDK now has a Rust-first representation because the runtime
contract already lives closest to the Rust data plane.

This SDK is the Rust-only surface for extending operators. It is intentionally
separate from the headless SDKs, which are for UI-free control and execution.
The closest frontend analogue is that wasm Python DSL authoring is not the same
thing as headless Python automation.

The current shape includes:

- `OperatorDescriptor`
- `OperatorPortDescriptor`
- `OperatorValidationProfile`
- `OperatorValidationStatus`
- `OperatorSchemaRef`
- `OperatorRunRequest`
- `OperatorRunResult`
- `OperatorArtifactRef`

### Architecture And Tensor Status

The machine-readable module owner is `sdk-operator`. It owns
`workers/rust/crates/operator-sdk` and `workers/rust/templates`; Installer owns
admission and deployment actions but no longer owns the SDK itself. Engine and
Installer depend on this module through explicit topology edges.

The `sdk_operator` tensor paradigm is distinct from `sdk_headless`. Current
authoring, manifest, readiness, host-policy, workflow-extension, fuzz, and local
dynamic-library evidence qualifies the Rust extension contract. A real Agent
loads both external-local and Installer-managed packages, dispatches over the
TaskIR RPC boundary, rejects package-integrity substitution, and executes again
after rejection. The bound-Orchestra path now resolves and serves exactly one
current-target package, while Installer downloads, verifies, and atomically
installs it. These are retained local qualifications, not a three-platform
operational claim. Promotion to the Daji target still requires automatic TaskIR
fetch handoff and the supported platform ABI matrix. A focused benchmark must measure package
discovery, admission, load, first dispatch, and steady dispatch without
borrowing general solver throughput numbers.

### Author-facing crate

The first author-facing Rust SDK crate now also exists in
`workers/rust/crates/operator-sdk` and provides:

- `OperatorHandler`
  low-level trait for fully custom operator handlers
- `JsonOperator`
  typed JSON-input helper trait for most operator authors
- `OperatorRegistry`
  in-process registration, descriptor lookup, and run dispatch
- `OperatorSdkError`
  duplicate-registration, decode, and handler failure surface
- `OperatorDescriptorBuilder`
  builder-style descriptor assembly so authors do not need to hand-write large
  `OperatorDescriptor` structs
- `operator_summary_result`
  helper for common summary-only operator outputs
- `operator_descriptor_readiness(...)`
  pure Rust author-side self-check for descriptor completeness before runtime
  loading is involved
- `operator_package_manifest_readiness(...)`
  manifest-only readiness report for external-local packages
- `operator_package_descriptor_readiness(...)`
  package/descriptor alignment check so manifest operators cannot silently drift
  away from the registered Rust descriptors
- `operator_sdk_surface_manifest()`
  machine-readable index of the Rust-only operator authoring, runtime,
  readiness, package-manifest, package-loading, and platform ABI surfaces

There is now also a minimal example operator at
`workers/rust/crates/operator-sdk/examples/minimal_operator.rs` that shows:

- declaring a descriptor with ports and validation metadata
- implementing `JsonOperator` with a typed input struct
- registering the operator in `OperatorRegistry`
- running it through the shared `OperatorRunRequest` path

There is also a more domain-shaped example at
`workers/rust/crates/operator-sdk/examples/electrostatic_peak_field_operator.rs`
showing a real electromagnetic post-processing operator that accepts
`SolveElectrostaticPlaneQuad2dResult` and emits a compact peak-field summary.

### External-local packaging

For external-local authoring, there is now a copyable starter crate at
`workers/rust/templates/operator-crate-template/` with:

- a ready-to-rename `Cargo.toml`
- a `kyuubiki-operator.json` manifest for external-local discovery using
  `{lib_prefix}` / `{lib_extension}` placeholders for cross-platform library
  naming
- required package-level industrialization fields:
  `sdk_api_version`, `minimum_host_version`, `validation_status`, and
  `validation_notes`
- strict manifest validation, so packages that do not declare their SDK
  contract or verification posture fail before runtime activation

The current package manifest contract is:

```json
{
  "schema_version": "kyuubiki.operator-package/v1",
  "sdk_api_version": "kyuubiki.operator-sdk/v1",
  "package_id": "operator.template.summary",
  "package_version": "0.1.0",
  "minimum_host_version": "1.15.0",
  "validation_status": "partial",
  "validation_notes": "Describe smoke, baseline, or qualification evidence.",
  "runtime": "rust_crate",
  "entrypoint": "target/debug/{lib_prefix}my_operator.{lib_extension}",
  "operators": [
    {
      "operator_id": "extract.my_summary",
      "kind": "extract",
      "entry_symbol": "register_my_operator"
    }
  ]
}
```

For `moxi 2.x`, `validation_status` should normally stay `partial` unless the
operator family has explicit baseline evidence and release checks.
The template keeps `minimum_host_version` at `1.15.0` as a backward-compatible
floor for the sample package; new moxi-only packages may raise that floor to
the first host line they actually require.
The engine host checks `minimum_host_version` before activation, so an operator
package built for a newer host line fails at load time instead of reaching an
undefined runtime path.
Successful activation also produces a package admission summary with the host
version, SDK API version, validation status, validation notes, runtime, and
operator ids so Installer and future CLI/UI surfaces can explain what was
accepted without reparsing the full package manifest.
Hosts can also call the external-operator preflight path to collect accepted
and rejected package summaries without activating dynamic code. Runtime loading
remains fail-fast, while preflight is the user-facing explanation surface.
The installer exposes the same read-only path as machine-readable JSON:

```bash
cargo run -p kyuubiki-installer -- operator-package-preflight ./operator-packages
cargo run -p kyuubiki-installer -- operator-package-preflight ./operator-packages --out tmp/operator-package-preflight.json
cargo run -p kyuubiki-installer -- operator-package-preflight ./operator-packages --fail-on-rejected
cargo run -p kyuubiki-installer -- operator-package-preflight ./operator-packages --fail-on-readiness-warnings
```

After preflight, Installer can own the package instead of asking Agent to load
from an authoring tree:

```bash
cargo run -p kyuubiki-installer -- install-operator-package ./operator-packages/example
cargo run -p kyuubiki-installer -- fetch-operator-package http://127.0.0.1:4000 operator.example 0.1.0
cargo run -p kyuubiki-installer -- operator-package-status
cargo run -p kyuubiki-installer -- uninstall-operator-package operator.example
```

The default store is the platform preference directory under
`kyuubiki/operator-packages`; every command accepts `--store-root path` for an
explicit isolated store. Installation first applies strict package readiness,
then copies only the manifest and current-platform dynamic library into a
staging directory, records SHA-256 provenance, verifies the staged package,
and atomically activates `packages/<package_id>`. Status re-verifies installed
content and exposes the exact `packages_root` that Agent should use. Uninstall
removes the selected package and prunes an empty store, so package lifecycle
does not leave a hidden authoring-tree or cache burden.
If a partial installation or damaged receipt is encountered, uninstall still
removes the validated package-ID target and reports `receipt_verified: false`
plus the receipt error instead of trapping the user with an unremovable store.

The same package ID is idempotent only when version, manifest, and binary
digests match. Different content requires an explicit uninstall before
replacement. The current lifecycle intentionally has no multi-version slots or
rollback yet; cross-platform packages must also carry a valid entrypoint for
each platform on which they are installed.

### Orchestra-owned distribution

Publish-time platform variants use
`kyuubiki.operator-package-distribution/v1`, separate from the execution
manifest. The index lives at
`<package-id>/<version>/kyuubiki-operator-distribution.json` under the directory
configured by `KYUUBIKI_OPERATOR_PACKAGE_DISTRIBUTIONS`. Each target such as
`linux-x86_64` or `macos-aarch64` names a package manifest and dynamic library by
strict relative path, exact byte size, and lowercase SHA-256 digest.

Orchestra resolves one exact target as
`kyuubiki.operator-package-resolution/v1`. It never substitutes another target
and serves only the indexed manifest and entrypoint. Installer disables HTTP
redirects, limits response sizes, rechecks identity, path, size, and digest, then
passes the materialized package through the same preflight and atomic managed
store used for local installation. If the control plane requires authentication,
the command reads `KYUUBIKI_ORCHESTRA_TOKEN`; tokens are not accepted as command
arguments and are not written to package receipts.

The schemas and portable examples are:

- `schemas/operator-package-distribution.schema.json`
- `schemas/examples.operator-package-distribution.json`
- `schemas/operator-package-resolution.schema.json`
- `schemas/examples.operator-package-resolution.json`

TaskIR already emits `kyuubiki.operator-package-fetch-request/v1`; wiring that
request directly to the Installer fetch API during Agent admission remains the
next runtime step. Until then, the explicit Installer command proves the supply
chain but is not described as automatic Agent fetch-on-demand.

For the repository template package, use the Make target:

```bash
make operator-package-preflight
make operator-package-preflight OUT=tmp/operator-package-preflight.json
make operator-package-preflight FAIL_ON_REJECTED=1
make operator-package-dynamic-smoke
make check-operator-package-dynamic-smoke IN=tmp/operator-package-dynamic-smoke.json
```

The JSON schema is `kyuubiki.operator-package-preflight/v1` and includes
accepted package summaries, rejected package reasons, host version, registry
kind, package readiness issues, readiness warning/error counts, and a safety
block that confirms dynamic libraries were not loaded. Use
`--fail-on-readiness-warnings` in CI when a package should be release-ready
rather than merely discoverable.

`make operator-package-dynamic-smoke` goes one step further for the repository
template package: it runs the template crate tests, runs strict package
preflight, builds the template `cdylib`, executes the engine host dynamic
loading smoke, then starts a package-enabled Agent and dispatches the operator
through RPC. The Agent stage also rejects a TaskIR whose package digest does not
match the activated entrypoint and proves recovery with a subsequent valid run.
The final stage installs the same package into an isolated Installer-managed
store, repeats the Agent execution/tamper/recovery journey, uninstalls it, and
requires the store to be residue-free.
It writes
`tmp/operator-package-dynamic-smoke.json` by default; override with
`OUT=tmp/custom.json`. Use it when changing the SDK host, package manifest,
template, or dynamic-library activation path.
`make check-operator-package-dynamic-smoke` validates the retained report
schema, package/operator summary, stage order, stage descriptions,
repo-local working directories, reproducible command vectors, stage success,
and repo-local evidence paths.
The retained report contract is also documented as
[operator-package-dynamic-smoke.schema.json](../schemas/operator-package-dynamic-smoke.schema.json)
with a fixture at
[examples.operator-package-dynamic-smoke.json](../schemas/examples.operator-package-dynamic-smoke.json).
The retained `v3` contract has six ordered stages, ending in
`installer_managed_agent_lifecycle`. Repository-local paths in retained
preflight evidence are normalized to project-relative paths.
- a minimal typed operator in `src/lib.rs`
- a tiny runnable `src/main.rs`
- a crate-local smoke test so authors start with a feedback loop immediately

The Rust operator SDK now defines the first external-local manifest,
discovery, and activation-boundary helpers:

- `OPERATOR_PACKAGE_SCHEMA_VERSION`
  stable schema id for external-local package manifests
- `OPERATOR_PACKAGE_MANIFEST_FILE`
  canonical manifest filename
- `OperatorPackageManifest`
  external-local package manifest contract
- `read_operator_package_manifest(...)`
  single-manifest parser and validator
- `discover_operator_packages(...)`
  scans a directory tree for `kyuubiki-operator.json`
- `OperatorPackageLoadPlan`
  resolved activation plan with manifest, root directory, and entrypoint path
- `OperatorPackageActivator`
  runtime-host hook that turns a load plan into registered operators
- `discover_and_activate_operator_packages(...)`
  convenience path for discovery plus host-driven activation
- `operator_package_descriptor_readiness(...)`
  non-loading readiness gate that checks manifest fields, Rust-only runtime
  declaration, descriptor fields, validation evidence, port shape, and
  manifest/descriptor id alignment

This means external-local support has now moved past simple discovery and into
an explicit activation boundary:

1. discover package roots
2. parse stable manifests
3. run non-loading readiness checks
4. resolve package entrypoints into load plans
5. let the runtime host activate those plans into an `OperatorRegistry`
6. bind the activated registry to the Agent and dispatch matching TaskIR

### Agent package execution

When `operator_packages_root` is configured, Agent startup now creates the real
dynamic host before publishing its descriptor. The descriptor reports the
actual activated package count; an empty root, load failure, or configured-count
mismatch fails startup rather than advertising a host that cannot execute.

External-local TaskIR uses the same protocol request as built-ins, with an
`OperatorTaskInputEnvelope` containing separate `payload` and `config` values.
Before dispatch, the Agent requires all of the following to match the activated
manifest and binary:

- `package_ref` is exactly `bundle://<package_id>`
- package version and operator kind match the manifest
- `package_integrity.algorithm` is `sha256`
- `package_integrity.digest` matches the activated entrypoint bytes

Successful execution returns a path-free package receipt with package, SDK,
runtime, operator, validation, and entrypoint digest identity. Failures return a
typed stage and recovery action. The retained integration test
`live_agent_loads_executes_rejects_tamper_and_recovers` proves valid execution,
fail-closed digest handling, and post-failure recovery through the Agent RPC
surface.

What remains intentionally host-owned is platform policy, not the full loading
mechanism. The SDK still does not hardcode one universal desktop/headless
loading policy, but the engine runtime now has a real dynamic-library host for
trusted local packages. The final mount-and-bind step is still represented by
the `OperatorPackageActivator` host hook so desktop, headless, and future
sandboxed runtimes can adopt different loading policies without changing the
operator author contract.

The current load-plan resolver also supports cross-platform library naming in
two ways:

- manifest placeholders such as `{lib_prefix}` and `{lib_extension}`
- fallback candidate resolution that maps library names onto the current
  platform when the manifest uses a portable basename

### Runtime migration status

The first runtime migration is also in place:

- built-in `extract.*` operators now dispatch through the Rust operator SDK
  registry path inside `workers/rust/crates/engine`
- built-in `export.*` operators now dispatch through the same registry path
  instead of direct hard-coded executor branches
- built-in non-bridge `transform.*` operators now dispatch through the same
  registry path
- built-in bridge-oriented `transform/workflow_bridge` operators now dispatch
  through the same registry path, so coupled heat/thermo and electrostatic/heat
  bridge execution already shares the author-facing Rust SDK boundary
- the engine now also exposes a host-side assembly boundary for external-local
  packages:
  `built_in_operator_registry(...)` for built-ins,
  `built_in_registry_with_external_packages(...)` for built-ins plus discovered
  packages, `load_external_operator_packages_with_dynamic_host(...)` for the
  real dynamic-library path, and `DeferredDynamicLoadActivator` as the explicit
  default when platform dynamic loading has not been enabled yet
- external-local activation now runs through an explicit trust policy surface
  in the engine host config:
  package-id allowlists, runtime allowlists, absolute-entrypoint toggles, and
  package-root boundary checks are all evaluated before activation
- the Agent now owns a live dynamic-host session for `external_local` TaskIR,
  publishes the actual activated count, verifies package identity and binary
  digest, and emits a sanitized execution receipt

### Model-assisted Rust authoring

The Rust Operator SDK exposes a machine-readable authoring surface for models
without turning model output into executable authority:

- `operator_model_authoring_manifest()` describes accepted operator kinds,
  handler traits, workflow data classes, the required authoring sequence, and
  hard trust boundaries.
- `OperatorModelDraft` carries the descriptor, input/output JSON schemas, Rust
  handler shape, and a bounded algorithm outline.
- `validate_operator_model_draft(...)` reuses descriptor readiness and adds
  model-specific limits for ports, tags, algorithm steps, side effects, and
  validation posture.
- [`operator-model-draft.schema.json`](../schemas/operator-model-draft.schema.json)
  is the portable draft contract; the matching fixture is
  [`examples.operator-model-draft.json`](../schemas/examples.operator-model-draft.json).

The output is deliberately a draft, not Rust source, a package, or a loaded
operator. An author or controlled code-generation pipeline must still implement
the Rust handler, run descriptor and package preflight, build the package, pass
dynamic smoke, and let the engine host apply trust policy before activation.
Models cannot load dynamic libraries or grant their own qualification status.

Built-in descriptors now carry:

- typed input/output ports
- artifact types
- dataset-value hints
- validation metadata such as `baseline_status`, `baseline_cases`, and
  `smoke_paths`

That does not mean every operator must execute in Rust forever. It means the
contract should be runtime-neutral and transportable.

## Current control-plane shape

The orchestrator now exposes built-in operator descriptors over HTTP:

- `GET /api/v1/operators`
- `GET /api/v1/operators/:operator_id`

These endpoints currently describe the built-in operator catalog in a shape
aligned with the Rust-side `OperatorDescriptor`.

## Suggested protocol shape

The public protocol should grow toward:

- `describe_operator`
- `list_operators`
- `run_operator`
- `cancel_operator_run`

This can live alongside the current explicit solver RPC methods while the SDK
is still maturing.

`moxi 2.x` does not need to delete the current explicit methods first.
It needs to stop painting itself into a corner when operator count grows.

## First 1.x priorities

The operator SDK should first support families that already have strong
verification momentum:

1. `frame_3d`
2. `thermal_frame_3d`
3. `thermal_truss_2d / 3d`
4. `heat_plane_triangle_2d / quad_2d`
5. `spring_1d / 2d / 3d`
6. `solid_tetra_3d`

These are good seed families because they already have:

- sample-backed orchestrated smoke
- formal solver baselines
- a clear result summary shape

## Relationship to workflows

This SDK is only half the story.

The operator contract defines how one capability behaves.
The workflow graph defines how multiple capabilities compose.

Use:

- [workflow-graph.md](workflow-graph.md)

when the question becomes:

- how outputs connect to downstream inputs
- how caching and intermediate artifacts work
- how graph execution should fail, resume, and report state
