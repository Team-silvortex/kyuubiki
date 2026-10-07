# Dispatch Cancellation Acknowledgement Loss

Date: 2026-10-07. Base checkout: `76aaf10d`, with this uncommitted follow-up.
Scope: verified source-chain recovery, not deployed qualification or a benchmark.

## Real Recovery Chains

Three new Rust SDK -> Orchestra -> native Agent tests exercise actual TaskIR
execution with a separate, never-selected peer. The tests use the existing
placement-tag contract to choose the held owner, not pool ordering. A test-only
TCP/HTTP relay consumes an actual upstream reply before disconnecting the caller;
it does not manufacture acknowledgements or introduce a production fault switch.

1. `lost_native_cancel_ack_recovers_original_cancelled_receipt_without_replay`:
   the Agent registers the exact cancellation. The native acknowledgement and
   original execution's terminal transport reply are discarded. Orchestra
   returns `cancellation_outcome_unknown`, while a separate result read recovers
   the original typed `cancelled` failure with matching identities.
2. `lost_negative_cancel_ack_does_not_relabel_completed_execution_as_cancelled`:
   the original computation completes before exact cancellation is sent. The
   real `target_not_observed`/`cancel_registered: false` acknowledgement is lost.
   The same unknown cancellation outcome must recover the original successful
   bar result, checked against the fixture's independent closed-form reference.
   Completion must not be relabeled cancelled.
3. `lost_http_cancel_ack_keeps_registration_unconfirmed_and_recovers_original_receipt`:
   native exact cancellation is registered and Orchestra produces its real
   acknowledgement, but the HTTP relay discards it. The public Rust SDK returns
   an error stating that registration is unconfirmed and automatic retry is not
   authorized. A separate result read recovers the original cancelled receipt.

Assertions include task/digest/attempt/request/process/generation identity,
exactly one cancellation request, no `cancel_job` fallback, one original Agent
execution, zero peer executions and no downstream project creation. Repeated
reads return the same original receipt. Native-transport-loss cases preserve
journal bytes and retain `outcome_unknown`; reading is not journal repair.
Each chain restarts only its owned Orchestra with the same journal, verifies
the original receipt again while the Agent remains alive, and explicitly starts
one healthy new attempt. New generation and execution counts demonstrate that
no pending job-wide cancellation leaked into the rerun. The terminal transport
reply of that rerun is also discarded in native-loss cases and checked through
result retrieval, not reported as synchronous success.

Tests and support:

- `workers/rust/crates/cli/tests/support/headless_cancellation_ack_loss.rs`
- `workers/rust/crates/cli/tests/support/ack_loss_proxy.rs`
- `workers/rust/crates/cli/tests/headless_live.rs`
- `apps/web/test/support/headless_live_server.exs`

## Cross-Platform Test Fixes

The first macOS parallel runs failed intermittently with `WouldBlock` (OS error
35) before an RPC frame was received. Accepted sockets could inherit the
nonblocking listener mode, while the relay expected blocking reads with bounded
deadlines. Explicitly restoring blocking mode resolves that mismatch. The
regression `accepted_nonblocking_socket_waits_for_delayed_rpc_without_rewriting_bytes`
forces the inherited state on every platform, delays the caller until upstream
connection, and checks byte-for-byte preservation, including JSON number text.
Relay errors now identify read/send stages rather than hiding their origin.

The same omission was corrected in the older SDK cancellation HTTP test server
and real-Agent HTTP completion adapter. These are test fixtures, not new runtime
dependencies. Owned live Orchestra test servers default to two Erlang schedulers
per child and preserve caller-provided `ERL_FLAGS`; this avoids multiplying the
host's entire scheduler count across concurrent tests. Read-only target polling
is bounded and must still match the original held identity; cancellation itself
is never retried or retargeted. Existing numerical checkpoint waits are unchanged.

## Verification

Final macOS ARM64 source tests:

- Complete `headless_live`: 21/21 passed three consecutive default-parallel
  runs in 18.15, 16.67 and 17.50 seconds. All three loss chains and the delayed
  accepted-socket regression executed; no ignored or filtered cases. After also
  bounding the mock upstream listener's accept wait, the final suite passed
  again in 17.41 seconds.
- Real-Agent `headless_task_completion`: 5/5 passed in 0.74 seconds.
- Complete Rust Headless SDK: 306 unit tests in 31.33 seconds and 25 integration
  tests passed. Protocol: 114 unit tests in 0.44 seconds and seven integration
  tests passed. Neither crate had ignored or filtered cases.
- Related Orchestra cancellation, result, journal, inspection, legacy routing
  and API suite: 46/46 passed in 2.1 seconds with warnings denied.
- CLI/Headless SDK/Protocol test-target Clippy passed with warnings denied.

Remote Ubuntu x86_64 source tests used an owned disposable Docker container,
Rust 1.88.0 and the existing Elixir 1.19 image (Elixir 1.19.5, OTP 28), with
8 CPUs, 8 GiB memory, 512 PIDs, four Rust test threads and two Erlang schedulers.
This is a remote source-test run, not Installer deployment or installed acceptance.

- Complete `headless_live`: 21/21 passed twice in 21.83 and 20.54 seconds.
  After bounding the mock listener's accept wait, the final suite passed again
  in 23.50 seconds.
- Real-Agent `headless_task_completion`: 5/5 passed in 0.11 seconds.
- SHA-256 matched for seven selected fixture/support files plus `Cargo.lock`
  and `mix.lock` between local and remote copies. Both lockfiles are unchanged.

Documentation book/inventory, API surface, module topology/matrix/tensor,
formatting and organization audits passed. Source files remain within 800 lines,
documents within 2,000 lines, with zero tracked organization debt.

Commands from `workers/rust` (loopback permission required for live tests):

```text
cargo test --locked --offline -p kyuubiki-cli --test headless_live
cargo test --locked --offline -p kyuubiki-cli --test headless_task_completion
cargo test --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo clippy --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

The remote live invocation adds `-- --test-threads=4` and includes both CLI
integration targets. Server addresses, credentials, SSH configuration and
deployment configuration are not included in source or this report.

## Limits And Storage

The original result remains in a bounded in-memory Agent cache. Orchestra restart
with an unchanged live Agent is covered; Agent restart, eviction/expiry durability,
machine reboot, durable cancellation intent, atomic cancel-versus-send and
initial-process attestation are not. Recovered computation receipt is not proof
that the missing cancellation acknowledgement was delivered. An absent or invalid
receipt remains unknown. This is not numerical qualification beyond the tested
bar/modal fixtures, Python/Elixir SDK parity, installed App acceptance or an
authenticated multi-host deployment. No production API, solver, ABI or engine
dependency changed, and no version bump, Git commit/push or App rebuild occurred.

Coverage evidence is `verified` for execution/contract/recovery only. The tensor
still has four maturity gaps, 19 evidence-grade gaps, 14 P0 qualification gaps
and blocked release status; executing tests remotely does not promote them to
deployed-grade evidence.

The disposable container was removed automatically. After final source-hash
comparison, the owned temporary source/build directory (approximately 427 MiB)
was deleted and its absence verified. No prior deployment, shared toolchain,
existing Docker service or unrelated research data was removed. Only this compact
report, source tests, recovery guidance and scoped tensor evidence are retained.
