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

## Legacy digest algorithm

`descriptor_digest` and legacy `task_digest` use:

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

This v1 encoding is not lossless numeric identity. For example, `1e-30` and
`2e-30` both canonicalize to `0.0`, and adjacent floats can round to the same
15-place value. A matching v1 digest alone does not establish that every
original floating-point input is unchanged.

## Native workflow invocation guard

Orchestra's bounded native diagnostic/scoring adapter additionally computes
`workflow_native_invocation_sha256` in `orchestration_context`. It hashes sorted
lossless JSON for the actual input artifact, config and context, with an
internal `kyuubiki.workflow-native-invocation/v1` domain tag. The binding field
itself is omitted from that input so recomputation is idempotent; supplied
binding values are replaced, not trusted. Other authority and selected-path
fields remain unchanged and participate in the binding.

The resulting hexadecimal string participates in the existing TaskIR digest.
Original payload/config numbers are not rounded, rewritten or duplicated in
retained records. This prevents the workflow recovery adapter from adopting an
old receipt for a different exact invocation despite a fixed-decimal collision.
Missing old bindings block reuse rather than being silently migrated or replayed.

The Agent still verifies the existing v1 TaskIR digest for default/legacy tasks;
it does not independently recompute this Elixir-adapter binding. This is not
authentication or a lossless digest upgrade for arbitrary TaskIR authors.
Explicit typed-v2 tasks use the independently recomputed encoding below instead.
Default adoption and installed workflow recovery rollout remain open. See the
[native precision report](../reports/workflow-native-invocation-precision-20261011.md).

## Typed binary64 task digest

Task structure remains `kyuubiki.operator-task-ir/v1`. Authors can explicitly
select `integrity.task_digest_encoding = kyuubiki.task-digest/typed-ieee754-v2`
with `digest_algorithm = sha256`. The task field projection is unchanged and
compiled into the verifier; a supplied `task_digest_fields` list cannot narrow
coverage. Descriptor digests retain their existing fixed-15 contract.

SHA-256 consumes the ASCII domain `kyuubiki.task-digest/typed-ieee754-v2`, one
zero byte, then the projected object encoded by this recursive grammar. Every
length/count is an unsigned 64-bit big-endian integer; tags are ASCII bytes.

| Value | Bytes |
| --- | --- |
| null | `n` |
| true / false | `t` / `f` |
| string | `s`, UTF-8 byte length, original UTF-8 bytes |
| integer | `i`, decimal byte length, minimal ASCII decimal with optional minus |
| float | `d`, the original eight big-endian IEEE-754 binary64 bytes |
| array | `a`, item count, each encoded item in order |
| object | `o`, pair count, encoded string key and value for each sorted pair |

Object keys sort by UTF-8 bytes, including within nested objects. No Unicode
normalization, decimal rounding, integer-to-float conversion, whitespace or JSON
escape spelling participates. Type tags and lengths prevent user objects or
strings from impersonating encoded numbers. `1` and `1.0`, positive/negative
floating-point zero, tiny values and adjacent binary64 values have distinct
preimages. The root projected object has depth zero; values deeper than 128 fail.

The numeric model is **decoded finite binary64 plus int64/uint64-range integers**
(`-9223372036854775808` through `18446744073709551615`). This is not exact decimal
arithmetic or preservation of every possible JSON number token. Decimal tokens
that decode to the same binary64 have the same identity and execution value.
Elixir rejects larger integers, non-JSON values, non-string keys and invalid
UTF-8 instead of converting them. Existing ingress size/depth limits still apply.

Authors opt in explicitly:

```rust
use kyuubiki_headless_sdk::{TASK_DIGEST_TYPED_V2, seal_operator_task_digest};
seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2)?;
```

```elixir
{:ok, task} = OperatorTaskIR.seal_task_digest(task, KyuubikiWeb.TaskDigestV2.encoding())
```

Rust sealing updates integrity only after successful computation. Elixir returns
an error without a changed task. Existing integrity metadata is otherwise
preserved. Both hash incrementally rather than creating another encoded input
copy; sorted objects need transient key-entry storage. No new retained artifacts,
snapshots or database tables are introduced.

Missing encoding means legacy `kyuubiki.task-digest/fixed15-v1` only. Explicit
null, non-string or unknown encoding and unsupported algorithms fail closed.
Verifiers never guess encoding by trying multiple digests. Removing/changing a
v2 encoding marker without resealing fails; the distinct domain also prevents
interpreting a typed digest as a legacy digest. Control plane, Rust SDK and
Agent independently recompute the selected digest before execution admission.
This detects changes, not the identity or authorization of the author.

`schemas/examples.task-ir-typed-digest.json` retains 26 shared vectors plus a
1,024-value deterministic binary64 corpus. Real source macOS transport tests
cover Rust/Elixir authors, HTTP verification and independent Agent recomputation
across five diagnostic domains, without the native adapter binding. This does
not qualify all executable package hosts, installed peers, Python/Elixir SDK
distribution or all persistence backends. Bounded source workflow recovery under v2
is described below; installed and remote qualification remain open.
Builders stay legacy by default; old records are not automatically resealed. See the
[typed digest report](../reports/task-ir-typed-digest-20261011.md).

### Agent capability routing and retained receipts

Rust Agents advertise the exact encoding string as a capability ID with role
`task_digest` and methods `run_operator_task_ir` / `fetch_operator_task_result`.
Orchestra derives this requirement from the integrity selector, in addition to
the task's declared runtime requirements. It does not alter the signed task or
trust a caller option that asks for a different encoding. The generic task RPC
entry point also retains this requirement if caller capabilities are empty.

Typed tasks require an explicit capability ID and a declared RPC method.
An old Agent, a role-only advertisement or an endpoint without metadata cannot
be a typed fallback. A failed capable endpoint cannot fail over to those peers.
No match returns `no_matching_agent` before dispatch. Operators using legacy
encoding retain existing routing behavior. Advertisements are compatibility
metadata, not authenticated proof of peer identity; the Agent still verifies
the digest independently. Static endpoints must carry truthful descriptor
metadata, or use registered discovery, rather than assume support from an address.

Source macOS tests execute ten typed tasks through HTTP and a real Agent:
Rust/Elixir authors across thermal, electrostatic, magnetostatic,
thermomechanical and Stokes diagnostics. One real Orchestra restart retains the
dispatch journal and retrieves all ten original Agent receipts without another
execution. Tiny config drift is rejected before dispatch. After Agent restart,
the same queries return unknown outcomes and never grant automatic replay or
result publication. This is bounded read-only receipt recovery, not durable
Agent results, whole-workflow continuation or exactly-once execution. See the
[typed routing and recovery report](../reports/task-ir-typed-routing-20261011.md).

### Explicit native workflow encoding and recovery

Workflow authors can set a graph-bound policy:

```json
{
  "defaults": {
    "native_task_digest_encoding": "kyuubiki.task-digest/typed-ieee754-v2"
  }
}
```

This selects the native diagnostic/scoring TaskIR adapter only, not solve RPCs,
local transforms or executable package hosts. Missing selection stays legacy;
null, non-string or unknown selection fails preflight. Rust authors use
`WorkflowDefaults.native_task_digest_encoding` and
`WorkflowNativeTaskDigestEncoding::TypedIeee754V2`; typed serialization preserves
the setting. Internal execution options cannot override the graph policy.

Typed asynchronous jobs use `kyuubiki.workflow-execution-envelope/v2`: one
canonical `execution_json` string containing the graph, input artifacts,
orchestration context and response options. It is data, not a path or executable.
The existing recovery digest binds this representation; no parallel payload
copy is retained. JSONB cannot normalize numbers inside a string. Decode retains
integer/float distinctions, signed zero, adjacent values and tiny floats before
initial execution and after restart. Legacy v1 envelopes keep their existing
numeric normalization; no old job or receipt is resealed. Encoding is visible in
public recovery metadata, but private execution data is not.

Seven source workflow rounds cover five solved physical-domain chains with
both branch routes, one raw thermal-result input, and one lost-Agent-receipt
negative case. Actual Orchestra restart reuses successful solves, retained
decisions and original native receipts; downstream scoring retains its
pre-restart TaskIR digest. Losing the Agent receipt blocks continuation without
replay or publication. Checkpoint verification decodes the execution once per
operation, not once per retained entry. Completed, failed and cancelled jobs
prune the execution payload; blocked recovery retains the existing single copy.

The [PostgreSQL follow-up](../reports/workflow-typed-postgres-recovery-20261011.md)
also verifies seven source macOS HTTP/Agent rounds against a remote PostgreSQL
store, including JSONB numeric identity, checkpoint rollback and selected-pool
lease fencing. Four read APIs report storage loss without throwing or rewriting
data. This is bounded recovery, not exactly-once, large-model qualification or
durable Agent receipts. Default adoption, installed mixed-peer, remote mesh,
Linux Agents, default cold-start policy and arbitrary package-host acceptance
remain open. See also the
[SQLite workflow recovery report](../reports/workflow-typed-execution-recovery-20261011.md).

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
