# Headless Runtime Binding Gate

Date: 2026-10-07. Base checkout: `76aaf10d`, with the previous cancellation and
original-receipt fixes preserved. Native batches now stop before dependent calls
when a declared output is absent or binding removes a required value. A completed
source computation remains completed; continuation is explicitly caller-authored.
This is source-chain verification, not installed acceptance or durable recovery.

## Binding Repairs

Test-first regression reproduced six failing cases: missing declared outputs,
null/blank required values, nested missing references, missing dry-run outputs,
approval of a destructive step with an unavailable binding, and dry-run preview
continuing beyond an unapproved source. The positive full-value binding case
already passed and remains covered.

`workflow_bindings.rs` supplies the shared parser, runtime resolver and failure
report. Preflight still checks declared top-level outputs and earlier-step
references. Execution additionally requires the referenced key in the actual
completed source object and rechecks required payload presence after resolution.
Optional null values remain available; this is not a general type/schema checker.
Exact whole-value references are expanded once. Inline template fragments and
template-looking strings inside bound results remain literal.

`kyuubiki.headless.binding_resolution` identifies `payload_resolution` failure
with no automatic retry strategy. Neither the dependent nor later steps reach
the executor. Earlier executed steps retain their results and success count.
Confirmation precedes binding resolution, and dry-run stops at the first
confirmation rather than claiming dependent preview work was completed.

Full results feed bindings before report compaction. Unbound payloads remain
borrowed even after an earlier result exists, avoiding the previous unnecessary
model copy; this is an ownership invariant, not a measured throughput claim.
The identical preview helper was consolidated in `run.rs`; `executor.rs` fell
from 785 to 601 lines without moving computation into the SDK.

Production changes are confined to the native Rust SDK:

- `workers/rust/crates/headless-sdk/src/workflow_bindings.rs`
- `workers/rust/crates/headless-sdk/src/workflow_batch.rs`
- `workers/rust/crates/headless-sdk/src/run.rs`
- `workers/rust/crates/headless-sdk/src/executor.rs` and `src/lib.rs`

## Real Service Chains

`real_collection_reply_missing_declared_binding_stops_before_writes` uses a real
Orchestra collection route through the public configurable `service_health`
read path. Its successful response has no declared `service` key. Both the
native SDK and actual CLI stop before project creation; the CLI exits nonzero,
reports a non-retryable failure, and writes the same report emitted on stdout.
Project listings remain unchanged and both owned Agents execute zero tasks.

`real_completed_task_null_binding_continues_only_after_explicit_downstream_repair`
executes a real built-in bar TaskIR through SDK, Orchestra and its owning Agent.
Displacement is checked against independent `FL/EA`. Its legitimate null
`package_ref` cannot become a required project name: the next step stops before
any HTTP write and the Agent has exactly one completed computation, zero on the
peer. A recording facade observes calls but never alters requests or receipts.

The caller separately fetches the original receipt, validates task identities
and actual solver data, then explicitly authors a one-step project continuation
with a valid name. Exactly that project is created. The source Agent remains at
one execution; its single original attempt and journal bytes are unchanged.
The forbidden later project is never created. Recovery envelopes can differ in
outer metadata from execute envelopes; their task identity and full Agent result
are checked separately rather than treating the whole JSON object as identical.

## Verification

Final macOS ARM64 source checks:

- Complete `headless_live`: 25/25 passed twice, in 27.80 and 20.64 seconds.
- Complete Headless SDK: 317 unit tests in 31.33 seconds and 25 integrations
  passed, including nine added binding tests.
- Complete Protocol: 114 unit tests in 0.46 seconds and seven integrations passed.
- Native CLI binary units: 182/182 passed in 0.78 seconds.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied.

Remote Ubuntu x86_64 used an unprivileged disposable source-test container with
Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory, 512 PIDs, four Rust test
threads and two Erlang schedulers. No installed service was restarted or replaced.

- Complete `headless_live`: 25/25 passed in 40.72 seconds.
- Complete Headless SDK: 317 unit tests in 31.29 seconds and 25 integrations passed.
- Complete Protocol: 114 unit tests in 0.40 seconds and seven integrations passed.
- SHA-256 matched 16 selected final source/support/schema/lock files. Lockfiles
  were unchanged. The container was removed and its 452 MiB temporary directory
  deleted; absence was verified. No model outputs or server settings are retained.

These complete targets had no ignored or filtered tests. Timings are correctness
regression observations, not a performance benchmark. From `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo test --quiet --locked --offline -p kyuubiki-cli --test headless_live -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

Formatting, runtime API contracts, module topology/matrix, document book/inventory,
organization audit and the coverage tensor self-test/check passed. The binding
claim is registered in `headless-batch-bindings.json` with grade `verified`.
Tensor structure has no gaps, but its four maturity gaps, 19 evidence-grade gaps
and 14 P0 release gaps remain; this source regression does not lift those gates.

## Scope

Native batch execution and dry-run share this gate. It does not introduce a
general resume API, rewrite an old report as a checkpoint, authorize automatic
source replay, provide exactly-once downstream writes, or guarantee Agent result
durability after restart. Original-result retrieval retains its separate strict
completion gates and bounded memory policy. Standalone Rust/Python/Elixir parity,
GUI automation and installed or authenticated deployment are not qualified by
these tests. No version bump, commit, push, App rebuild or installation occurred.
