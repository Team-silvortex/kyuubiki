# Original Receipt Availability Boundaries

Date: 2026-10-07. Base checkout: `76aaf10d`, with the prior acknowledgement-loss
follow-up preserved. Original-result retrieval now rejects incomplete or
contradictory envelopes, while real cache eviction, endpoint loss and Agent
restart keep the old attempt unknown without automatic execution or publication.
Evidence is source-chain verification, not durable recovery or installed acceptance.

## Receipt Contract Repairs

Test-first regressions reproduced three acceptance gaps: the Rust SDK accepted
an unknown reply missing the required `completion` field, ignored invalid optional
execution identity on unknown replies, and Orchestra accepted an incomplete
native envelope as a legitimate unavailable result. These replies did not pass
as successful computation, but violated the existing v1 formats and could conceal
malformed transport or an incompatible Agent.

The Rust SDK now requires all nine public-envelope fields and rejects keys
outside the nine required plus three optional incarnation fields. Optional
request/process IDs must be bounded strings, process ID cannot be `unavailable`,
and an optional generation must be a positive `u64`; recovered receipts require
all three. All nine unavailable statuses must retain unknown outcome, explicit
null completion and false replay/publication flags. Valid optional identity
metadata is still accepted according to the existing contract.

Orchestra now requires all 13 native-envelope fields, valid UTF-8 process identity,
null or positive `u64`-bounded generation, and the exact fixed retention policy.
The policy constants are read from `agent-task-result-retention.schema.json`
at compilation with an external-resource dependency, not from a runtime path.
Missing `response` no longer counts as explicit null. Bad policy, extra fields,
invalid generation and malformed process identity become `agent_receipt_invalid`
with unknown outcome and no completion. Existing task-bound completion/failure
gates remain unchanged. This is application validation of the fixed envelope,
not a general JSON Schema engine, authentication or cryptographic provenance.

Changed production files:

- `workers/rust/crates/headless-sdk/src/service_executor_dispatch_result.rs`
- `apps/web/lib/kyuubiki_web/orchestra/operator_dispatch_result.ex`

## Real Availability Loss Chains

`real_count_eviction_keeps_original_unknown_without_touching_peer_or_downstream`
starts a real TaskIR bar computation through Rust SDK and Orchestra. A test relay
loses its actual terminal transport reply; a separate query first recovers the
original result and checks the independent reference `FL/EA`. The test explicitly
admits 64 fresh correlated native computations, also checked against that
reference, to exercise the real 64-entry cap. Reading the oldest receipt just
before the last admission does not promote its reservation order. It is then
`not_retained`, not cancelled, completed or replayable.

Repeated original-result reads leave one unknown journal attempt, unchanged
journal bytes and project listing, and zero peer executions. The owner's 65
executions are exactly the original plus 64 explicit pressure admissions. Only
an explicitly requested rerun adds the 66th execution and a new journal attempt;
its separately recovered healthy result has a newer generation. The old receipt
stays unavailable. Direct pressure admissions do not fabricate journal records,
override cache policy or represent automatic recovery.

`real_endpoint_removal_and_agent_restart_never_substitute_the_original_receipt`
also begins with an actual computed, separately recovered original receipt.
Stopping its owned Agent yields `original_endpoint_unreachable`; restarting only
Orchestra with the peer configured instead yields `original_endpoint_not_configured`
without querying the old relay or executing on the peer. Restarting the same
owned Agent and restoring its endpoint yields `not_retained` because its cache
is empty. These stages keep the original journal bytes and unknown attempt,
no project creation and no automatic execution. A fresh explicit attempt can
return a healthy result with a different process instance and request ID, but
cannot replace the old attempt's absent receipt.

Two new native Store tests separately check reservation-origin expiry: repeated
reads do not renew TTL or eviction order, and a computation finishing at the
600-second boundary cannot resurrect its expired reservation. These use explicit
monotonic test instants, not a production clock override or a ten-minute wall-time
experiment. Existing count/byte/oversize, ambiguity and descriptor-policy tests
remain in the seven-test cache suite.

## Verification

Final macOS ARM64 source verification:

- Complete `headless_live`: 23/23 passed twice, in 17.60 and 25.60 seconds,
  including the previous three acknowledgement-loss cases and both new chains.
- Native `kyuubiki-cli` binary unit suite: 182/182 passed in 0.80 seconds.
- Complete Rust Headless SDK: 308 unit tests in 31.35 seconds and 25 integration
  tests passed. Protocol: 114 unit tests in 0.48 seconds and seven integrations
  passed. No ignored or filtered cases in these complete targets.
- Related Orchestra cancellation/result/journal/API/legacy-routing suite:
  47/47 passed in 4.6 seconds with warnings denied.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied; formatting,
  contract/API, topology, documentation and organization checks passed.

Remote Ubuntu x86_64 used a disposable, unprivileged Docker source-test container
with Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory, 512 PIDs, four Rust
test threads and two Erlang schedulers. No installed service was replaced or
restarted. Final results:

- Complete live source chain: 23/23 passed in 19.36 seconds, after a 28.15-second
  cold test build. This is correctness evidence, not a performance benchmark.
- The same selected Web suite: 47/47 passed in 2.2 seconds. An earlier attempt
  stopped before tests because five public root-level workflow fixtures were
  missing from the temporary copy; those explicit files were then supplied.
- SDK result-validation subset: 5/5 passed; 303 other SDK tests were filtered.
  Native cache subset: 7/7 passed in 0.19 seconds; 175 other binary tests filtered.
  These subsets are not a claim of full remote SDK or native unit coverage.
- SHA-256 matched for 13 selected production/test/fixture-support files, schemas
  and lockfiles between final local and remote copies. Lockfiles are unchanged.

Commands from `workers/rust`:

```text
cargo test --locked --offline -p kyuubiki-cli --test headless_live
cargo test --locked --offline -p kyuubiki-cli --bin kyuubiki-cli
cargo test --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo clippy --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

Remote live testing adds `-- --test-threads=4`. Sources and fixtures are temporary;
addresses, credentials and deployment configuration are not retained in this report.

## Qualification Limits

Safe unknown after restart is not successful recovery of the original result.
No durable cache, replay policy, cancellation-intent store, initial-process
attestation, installed App test, authenticated multi-host deployment or additional
Python/Elixir SDK parity is introduced. Numerical checks qualify only the tested
bar reference, not arbitrary materials or solver families. The broader standalone
Web and all-package CLI integration suites were not run. Existing API versions,
solver/engine separation and bounded in-memory retention remain unchanged.

The tensor remains verified execution/contract/recovery evidence: four maturity
gaps, 19 evidence-grade gaps, 14 P0 qualification gaps and blocked release status.
No version bump, commit, push, App rebuild or installed-service restart occurred.

The disposable test container was removed automatically. Final source hashes
matched before deleting the owned temporary source/build directory (about
483 MiB); its absence was verified. Prior deployments, shared toolchains,
unrelated caches and research data were not removed. An old template build cache
encountered during source synchronization was excluded from the temporary copy,
not deleted from the development machine. Only tests, repairs, guidance and
scoped evidence are retained.

## TaskIR Cache Readability Follow-Up

Date: 2026-10-26. This is an additional local source check, not a rerun of the
historical Linux or Orchestra results above. The previously edited native durable
store/SDK recovery sources remain in the checkout unchanged by this repair.

Three test-first regressions failed against the old process-memory cache:

- A serializable success or failure with deep nested output could be retained
  even though its JSON reader rejected that same response.
- A response readable by itself could exceed the reader limit once nested in
  the result-retrieval RPC reply. The success fixture reproduced this at 125
  payload containers, with the original response contributing another level.
- The generic cache writer could retain a null, scalar or array, contradicting
  the existing object-only response contract.

The writer now decodes the byte-bounded response before marking it retained.
It requires an object and at most 125 nested containers, leaving two levels for
the outer RPC and its retrieval result. The default JSON reader still imposes
its own recursion limit; no unbounded decoder or larger frame limit is enabled.
The temporary decoded tree and depth walk happen outside the cache lock, and
their allocation is additional to the 32-MiB retained-encoded-byte budget.
Valid bytes are kept unchanged rather than reserialized from the decoded tree.

Rejected new responses become `result_not_retained`; the original response is
unchanged. Fetch validates again after releasing the lock. Unexpected unreadable
bytes or response/status contradictions return `task_result_retention_unavailable`
without a success payload, not a successful `receipt_retained` with null response.
The read does not mutate or repair the cache. Ordinary pending, absent and
uncached results remain successful queries with no completion. The exact v1
policy, TTL, byte/count limits, generation and identity fences remain unchanged.
No automatic replay, retry, publication, new file or disk archive is introduced.

The separate eight-test readability module covers both success and failure depth
boundaries through the actual fetch-response builder and two wire decoders,
failure isolation from a healthy peer entry, unchanged state on repeated reads,
null/extension/escaped payload content, finite floating-point bits including
negative zero and subnormals, and exactly 8 MiB versus one byte over the cap.
Malformed cached bytes are injected below the checked private writer for defensive
read testing; this is not evidence that ordinary callers can mutate immutable
in-memory cache bytes. These tests do not qualify arbitrary solver output,
physical power loss, remote deployments or installed Apps.

Verification for this follow-up on macOS ARM64:

- Native CLI binary unit suite: 323 passed, including 19 cache tests (eight new).
- `agent_durable_task_ir_live`: five real TaskIR lifecycle cases plus two support
  checks passed. Committed success, panic, cancellation, commit failure and
  ordinary non-durable retention stay consistent with their existing boundaries.
- `agent_durable_response_limits_live`: three real large-output cases plus two
  support checks passed, with serial 100,000-element solves, compact receipt
  recovery after forced termination, failed commits and healthy subsequent work.
- `headless_live` selected original-receipt limits, dispatch inspection and
  cancellation-acknowledgement-loss modules: seven passed, 76 filtered. These
  run the real local SDK/Orchestra/Agent chain with isolated temporary data;
  they do not inject the new depth cases into a production solver.
- CLI all-target Clippy passed with warnings denied. No lockfile, public schema,
  package version, installed service, commit or push was changed by this repair.
- Formatting, diff whitespace, documentation book/inventory and organization
  checks passed after keeping the SDK guide within its 2,000-line limit. The
  added local cache evidence does not raise qualification: tensor structure
  passes with 13 modules, 11 paradigms, zero structural gaps, two maturity gaps,
  23 evidence-grade gaps and 18 P0 gaps; release status remains blocked.

Commands from `workers/rust`:

```text
cargo test --locked --offline -p kyuubiki-cli --bin kyuubiki-cli
cargo test --locked --offline -p kyuubiki-cli --test agent_durable_task_ir_live --test agent_durable_response_limits_live -- --test-threads=4
cargo test --locked --offline -p kyuubiki-cli --test headless_live -- original_receipt_limits:: orchestra_dispatch_inspection:: cancellation_ack_loss:: --test-threads=2
cargo clippy --locked --offline -p kyuubiki-cli --all-targets -- -D warnings
```

## Control Plane RPC Envelope Follow-Up

Date: 2026-10-26. After the cache readability repair, five test-first groups
reproduced control-plane envelope acceptance gaps (32 selected tests, five
failures before repair): a success carrying an error, a failure carrying a
result or malformed error text, terminal frames with progress fields, progress
frames with terminal fields reaching callbacks, and a mixed cached terminal
frame recovered as confirmed completion.

`AgentRpcTransport.validate_terminal_response/2` now checks both live terminal
frames and nested original-attempt responses in `OperatorDispatchResult`.
Success needs a non-null result and no non-null error. Failure needs no non-null
result and a map error with nonblank UTF-8 code/message. Terminal frames reject
the presence of event/progress fields; progress/heartbeat frames reject the
presence of ok/result/error fields before callback delivery, including null
fields. Rejections return protocol reason atoms, not the rejected payload.

Inactive fields can still be absent or explicitly null. Non-null scalar and
array success results, nested nulls, opaque extensions, typed TaskIR failure
details and normal ordered progress/heartbeat delivery remain supported. This
does not close all envelope extension fields, change the RPC version, parse
duplicate JSON keys, or replace domain-specific completion/failure gates.

Nine added managed-dispatch cases combine three malformed terminal replies
(contradictory success, contradictory capacity refusal, mixed terminal frame)
with idempotent, checkpointed and checkpoint-required policies. Each retains
one unknown attempt, releases its capacity lease, records protocol health failure
only for the original Agent and sends no task to the peer. In particular, a
claimed capacity refusal accompanied by a result is not proof of non-admission.
Existing valid connect/capacity refusal controls still permit failover. This is
the managed TaskIR path; generic standalone retry semantics are unchanged.

Cached malformed success/failure replies produce `agent_receipt_invalid`, unknown
outcome and null completion without changing journal bytes or publication/replay
flags. Valid recovered successes and task-bound failures remain available. The
new protocol-fault tests use local simulated Agent sockets and injected retained
reply payloads, not hostile access to a running production Agent. No installed
service, version, schema, lockfile, commit or push is changed by this follow-up.

Verification for this follow-up on macOS ARM64:

- The expanded five-file focused suite passed 103 tests. After strengthening
  the callback-order assertion, the three directly changed test files passed
  all 80 tests again. These counts overlap and are not additive coverage.
- The broader Playground, dispatch journal/retrieval/cancellation, workflow
  dispatch and selected public API suite passed 489 tests with warnings denied.
  This includes 19 new tests: seven transport-envelope cases, three retained
  receipt cases and nine managed-dispatch policy combinations.
- Complete `headless_live` target: 80 passed and three explicitly provisioned
  PostgreSQL tests remained ignored, with no filtering, in 311.37 seconds.
  Real local SDK/Orchestra/Agent solves, multi-domain cancellation, SQLite
  workflow process restarts, original-result reuse and acknowledgement-loss
  paths passed. This is correctness testing, not a performance benchmark or
  PostgreSQL/Linux/installed-App qualification.
- Formatting, diff whitespace, HTML book/inventory, source/document line limits
  and tensor structure checks passed. The tensor remains at zero structural,
  two maturity, 23 evidence-grade and 18 P0 gaps, with blocked release status.
  Local verified evidence is not promoted to installed or operational acceptance.

Targeted reproduction/verification from `apps/web`:

```text
MIX_ENV=test ERL_FLAGS='+S 2:2' mix test test/kyuubiki_web/playground/agent_rpc_envelope_test.exs test/kyuubiki_web/playground/agent_task_uncertain_dispatch_test.exs test/kyuubiki_web/orchestra/operator_dispatch_result_test.exs --warnings-as-errors
```

The complete live target ran from `workers/rust`:

```text
cargo test --locked --offline -p kyuubiki-cli --test headless_live -- --test-threads=2
```

## Control Plane Raw JSON Follow-Up

Date: 2026-10-26. The preceding envelope follow-up checked parsed values; this
follow-up closes the earlier raw JSON gap. Five test-first raw-socket groups
failed before repair: duplicate envelope fields, nested/escaped duplicate keys,
duplicate failure codes/details, duplicate progress data reaching callbacks,
and malformed JSON returning a decoder error containing the entire raw frame.
The original parser collapsed object pairs, so later checks could not see the
contradiction. A duplicate capacity code could appear to be a valid refusal.

Orchestra now uses Jason's ordered-object representation to retain pairs until
each object's decoded keys are verified unique. The transport parses terminal
frames only once rather than parsing the same reply twice. Checks cover arrays,
extensions, failure details and nested retained receipts. Duplicate equal values
are rejected too. Syntax/duplicate errors return fixed atoms only, not the raw
payload or offending field name. No dependency, schema or protocol version changes.

Legal repeated names across separate objects, escaped unique keys, distinct
Unicode spellings, opaque JSON-looking strings, nested nulls and array order
remain supported. Tests compare float bits and integer values against Jason's
native decoding, including negative zero, subnormals, maximum finite values and
integers beyond 64 bits; there is no numeric stringify/reparse step. Wide tests
include 20,000 scalar values, 2,000 independent objects and a 2,000-field map.
This is compatibility testing, not a large-model performance benchmark. Encoded
frame caps are unchanged and temporary decoded trees add memory; no decoded-heap
limit or whole-process resource isolation is claimed.

Nine managed-dispatch combinations exercise duplicate success, duplicate request
identity and duplicate capacity codes under all three retry policies. Each keeps
one unknown original attempt, releases its lease and records a failure only for
the original Agent without contacting a peer. Four real-loopback recovery cases
cover a valid original result, duplicate nested status, duplicate outer result
and malformed JSON. Invalid replies produce `agent_receipt_invalid`, not
`original_endpoint_unreachable`; valid recovery still succeeds. All preserve
original journal bytes, with no publication or replay authority. These raw faults
come from simulated test Agents, not a production deployment.

Initial verification on macOS ARM64: the selected transport/client/recovery
suite passed 65 tests; the expanded JSON, uncertain-dispatch and original-result
suite passed 95 tests. Counts overlap and are not additive coverage.

Final verification for this follow-up:

- The broader Playground, dispatch, workflow and selected public API suite passed
  511 tests with warnings denied. This includes 22 new tests: nine JSON boundary
  and compatibility tests, nine dispatch policy combinations and four raw-wire
  original-result recovery cases.
- The complete `headless_live` target passed 80 tests with no filtering in
  321.99 seconds. Three PostgreSQL cases stayed explicitly ignored because no
  disposable PostgreSQL test database was provisioned. Real local computation,
  SDK/Orchestra/Agent calls, cancellations, SQLite workflow process restarts and
  original-receipt recovery passed; this is not Linux or installed-App acceptance.
- Formatting, diff whitespace, documentation book/inventory and organization
  checks passed. Source/document limits remain 800/2000 lines. The tensor still
  reports zero structural, two maturity, 23 evidence-grade and 18 P0 gaps, with
  blocked release status. This adds local verified evidence, not operational
  acceptance, a memory benchmark or a maturity upgrade.

Focused reproduction from `apps/web`:

```text
MIX_ENV=test ERL_FLAGS='+S 2:2' mix test test/kyuubiki_web/playground/agent_rpc_json_test.exs test/kyuubiki_web/playground/agent_task_uncertain_dispatch_test.exs test/kyuubiki_web/orchestra/operator_dispatch_result_test.exs --warnings-as-errors
```

The complete live command is the same `headless_live` invocation documented in
the preceding follow-up. No version bump, commit, push, package rebuild or
installed-service change is part of this work.

## Control Plane Stalled Reader Follow-Up

Date: 2026-10-26. A raw-loopback regression sends a 16-MiB request to a peer
that accepts the connection but does not read. With a 150-ms request budget the
old transport still had not returned after one second. Its budget began only
after `send` and no send timeout was configured. Adding a shared network budget
and finite send timeout alone did not fix the test: a stack inspection found
the caller waiting in `prim_inet.close_pend_loop` while cleanup tried to drain
buffered output. The installed OTP source confirms this implicit drain wait;
the observation is not an assertion that every backend blocks inside `send`.

The transport now encodes once before opening its socket and uses the same
deadline for connecting, sending and receiving. Socket send waits use the
remaining time, with `send_timeout_close` enabled. Cleanup sets zero linger on
this single-use connection so abandoned buffered output cannot prolong the
caller or its execution lease. Backend send timeout tuples with an unsent-data
suffix become a plain timeout instead of retaining request bytes in diagnostics.
Deadline checks around decoding and terminal validation prevent already-late
outputs from becoming success; neither heartbeats nor fragments reset the budget.

Five direct transport tests cover the stalled reader, complete heartbeats,
partial frames, a slow local callback with an already-buffered terminal result,
and valid fragmented results plus a healthy request after protocol failure.
Three managed-dispatch tests combine a stalled reader with idempotent,
checkpointed and checkpoint-required policies. Every original attempt stays
unknown, its lease is released, only the original endpoint receives a health
failure, and the peer receives no replay. A new independent task then completes
on the healthy peer without rewriting the uncertain original observation.

The focused transport, uncertain-dispatch and transport-boundary suite passed
67 tests under the default inet driver and 67 tests under the socket backend on
macOS ARM64. These are overlapping local runs, not Linux/Windows acceptance.
During test development, completed-task cleanup initially called `Task.shutdown`
from ExUnit's separate cleanup process; the fixtures now terminate only their
own recorded test PIDs, avoiding that ownership error.

Scope limits: request encoding precedes the network budget; decoding and local
callbacks remain synchronous and cannot be forcibly preempted by these checks.
This is not a hard CPU/heap ceiling or proof that remote execution stopped.
A partial send is still uncertain, never proof of non-admission. Managed replay
policy, result retention, scientific validation and installed services are unchanged.

Focused commands from `apps/web`:

```text
MIX_ENV=test ERL_FLAGS='+S 2:2' mix test test/kyuubiki_web/playground/agent_rpc_deadline_test.exs test/kyuubiki_web/playground/agent_task_uncertain_dispatch_test.exs test/kyuubiki_web/playground/agent_rpc_transport_boundary_test.exs --warnings-as-errors
MIX_ENV=test ERL_FLAGS='+S 2:2 -kernel inet_backend socket' mix test test/kyuubiki_web/playground/agent_rpc_deadline_test.exs test/kyuubiki_web/playground/agent_task_uncertain_dispatch_test.exs test/kyuubiki_web/playground/agent_rpc_transport_boundary_test.exs --warnings-as-errors
```

Final verification on macOS ARM64:

- The broader Playground, dispatch, workflow and selected public API suite passed
  519 tests with warnings denied. This includes eight new tests: five transport
  cases and three managed-dispatch policy combinations. Focused counts above
  overlap this run and must not be added to it as distinct coverage.
- Complete `headless_live` passed 80 tests with no filtering in 302.15 seconds.
  Three PostgreSQL cases remained explicitly ignored because no disposable
  PostgreSQL test database was provisioned. Real local SDK/Orchestra/Agent
  calculation, cancellation, reply-loss and SQLite restart/recovery paths passed.
- Formatting, diff whitespace, documentation book/inventory and organization
  checks passed with source/document limits 800/2000. Tensor structure passed,
  retaining zero structural, two maturity, 23 evidence-grade and 18 P0 gaps and
  blocked release status. Evidence is locally verified, not installed or
  operational acceptance. No version bump, commit, push or package rebuild.
