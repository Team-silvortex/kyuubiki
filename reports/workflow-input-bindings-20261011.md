# Workflow input bindings before execution and recovery

The Rust graph executor and Orchestra now reject connected inputs that the
node would not consume. Previously a single-payload node could execute only
its first incoming edge while recording every available edge as a source.
Input nodes could also ignore upstream edges entirely. Both cases could look
like successful composition despite an incorrect binding.

This is a source macOS contract follow-up to the bounded solve-artifact and
branch-decision checkpoints. The base commit is `6d9e1094` (`daji 3.5.6`);
package metadata remains `3.5.0`. No release bump, commit, packaging or
installed-application acceptance is included.

## Binding rules

- Input nodes read `input_artifacts` and reject incoming edges.
- Solve, extract, export, condition and single-payload transform nodes reject
  more than one connected input. An operator name does not grant a different
  binding mode to another node kind.
- Declared but unconnected spare input ports remain allowed. Missing connected
  input behavior is unchanged; this gate does not impose a new minimum arity.
- Named multi-input transforms still assemble payloads by target port. Existing
  single explicit envelopes and sibling-data preservation are unchanged.
- `transform.first_available` still waits for all predecessors to resolve and
  then uses edge declaration priority. Its trace and lineage name only the
  selected source.
- Output nodes retain multiple incoming ports, copying each value with its
  own source lineage.
- The gate runs after endpoint, reference and cycle validation, before any
  node execution. Existing cycle diagnostics retain precedence.
- `on_error: "skip"` and compact responses cannot turn a structural binding
  error into a successful or partial workflow.

The Elixir `WorkflowGraphInputs` module owns the predicates used by both
preflight and execution, avoiding a separate duplicated multi-input whitelist.
Rust validation uses the existing executor binding predicates. Both single
payload resolvers also reject multiple edges defensively. The new validation
scans nodes and edges once and adds no retained snapshots or artifact copies.
Operator availability remains specific to each runtime's catalog.

## Controlled validation

The shared `tests/fixtures/workflow-graph-preflight.json` gains nine cases:
six invalid binding shapes and three allowed multi-input/spare-port shapes.
The six Elixir rejection cases failed against the old gate; the Rust shared
contract also exposed the old single-input acceptance.

There are 34 additional Elixir tests: nine shared cases, twenty-four binding
and lineage cases, and one public API admission matrix. They cover all twenty
existing named-input Orchestra transform modes in both node declaration
orders, custom single-payload callbacks, spare ports, fallback edge priority
and multi-port outputs. Invalid direct and restored-graph executions cannot
invoke operator callbacks, emit progress or consult solve/branch checkpoints.

The API matrix rejects all six shapes at both synchronous and asynchronous
endpoints, in full and compact response modes. It verifies HTTP 422, no
artifacts/job receipt and no newly admitted jobs. The Rust execution test
checks the six shapes under both node and edge declaration orders, retaining
the fixture's explicit skip policy without swallowing the structural error.

Final source validation:

| Selection | Result |
| --- | --- |
| Expanded SQLite workflow/recovery/dispatch selection | 644 listed, 635 passed, 9 JSON-only skipped, 0 failed |
| Isolated JSON checkpoint/recovery/binding/API selection | 310 passed, 0 failed |
| Complete Rust Engine library suite | 651 listed, 650 passed, 1 prebuilt-plugin test ignored, 0 failed |
| Rust Engine and CLI all-target Clippy | Passed with warnings denied |
| Elixir test-environment compilation | Passed with warnings treated as errors |

Selections overlap; their counts are not additive coverage percentages. The
Rust Engine suite includes existing physics, graph execution, projection,
budget and fuzz-smoke checks, but these are not an independent scientific
validation campaign. The ignored plugin case requires a prebuilt template
dynamic library and is not counted as passed.

## Actual service restart with a rebuilt join

`real_branch_workflow_restart_retains_route_before_native_receipt_recovery`
now routes both condition ports through a local `recovery-join` using
`transform.first_available` before native scoring. The thermal true route and
electrostatic false route each start real Rust Agents and source Orchestra,
persist SQLite/journal state, lose the actual BEAM after successful diagnosis,
and resume the original parent after lease expiry through the Rust Headless SDK.

The two joined restart rounds require exactly six Agent executions: one solve,
one diagnosis and one score per round. The peer performs no duplicate work.
The rebuilt join completes, consumes only the selected condition port and
retains that exact source in artifact lineage. Its output equals the recovered
diagnosis, and scoring consumes `recovery-join.summary`. The unchosen path and
its output stay skipped. Original successful dispatch evidence remains
byte-stable; normal completion prunes the private checkpoint payloads.

The focused service test passed in 37.35 seconds. It is two source macOS
restart rounds, not a freshly rerun full service suite, remote deployment,
installed app or cross-platform acceptance.

## Remaining limits

Local transforms and joins still rebuild under the original replay policy.
This is not general durable per-node continuation, exactly-once execution,
authenticated provenance or a universal custom-operator binding declaration.
Unknown native outcomes, unavailable receipts, invocation drift and
side-effect replay restrictions still block recovery. Input binding checks
also do not establish physical convergence, independent numerical accuracy
or scale performance.

The coverage tensor records this follow-up as verified contract evidence only.
It does not promote numerical, benchmark, operational or release readiness.
Topology, module/function matrix, extension standard, runtime API surface,
tensor and tensor self-test, document inventory/book and version-line checks
pass. The tensor still reports zero structural gaps, four maturity gaps,
nineteen evidence-grade gaps and fourteen P0 gaps; Daji qualification remains
blocked. Project organization passes the 800-line source and 2,000-line
document limits with no tracked debt.
