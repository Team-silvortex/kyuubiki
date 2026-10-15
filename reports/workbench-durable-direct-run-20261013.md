# Single-Agent Workbench durable run closure

Version: daji 3.6.0. Source-tree validation, no release installation or version change.

## Implemented

- Exactly one direct endpoint routes the ordinary Run action and PWDT `job/run` to
  asynchronous durable admission, not the legacy synchronous solve/cache. The Agent
  must advertise an enabled execution ledger. No uncertain submission is retried or
  sent to another Agent. Multi-endpoint mesh selection is unchanged.
- Caller identities are committed before dispatch. Native desktop storage is a fixed,
  private, locked metadata journal (64 entries / 64 KiB); browser mode uses a separate
  IndexedDB journal. No input/result snapshots or credentials are stored. Native host
  failure cannot silently downgrade durability. One unresolved gateway/endpoint
  reservation blocks new IDs, including from another view.
- Automatic observation is bounded. Completed results can populate the current run;
  changed observation ownership cannot. A changed receipt producer/digest/request is
  rejected. Query errors retain the original handle without resubmitting.
- Ordinary Cancel uses the durable handle and freshly inspected process/digest, not
  Orchestra cancellation. Registration is not terminal confirmation and does not
  release the observation token or clear results.
- System > Runtime > Direct Execution Recovery offers local pending identities after
  reload, through `directExecution/pending`. Matching terminal inspection can retire
  the exact caller reservation without changing the model/result or deleting the
  Agent receipt. Unknown/not-found records remain pending.
- Desktop-managed Agent startup derives separate directories under runtime state,
  configurable with `KYUUBIKI_AGENT_EXECUTION_STORE_ROOT`, and reports the root in
  runtime status. Existing processes are not adopted or silently restarted.

## Initial Verification

- Frontend regression: 179 passing tests, including replaced receipt identity and
  endpoint admission checks.
- Native desktop-runtime: 74 passed, one existing whole-stack test ignored. Includes
  real child-process force termination after journal synchronization, reopening,
  duplicate admission prevention, lock release, capacity, corruption and symlink tests.
- Shared desktop bridges: eight passing tests, including origin/source fencing,
  metadata-only discovery, exact command allowlisting and missing-host fail-closed.
- Chromium: four passing scenarios, including ordinary run completion and lost reply
  followed by page reload, duplicate blocking and explicit terminal reconciliation.
  These exercise actual source GUI/PWDT routing and browser storage with mocked HTTP,
  not a real Agent or loaded Python interpreter in the browser.
- The first browser run failed because the new fixture omitted required runtime
  descriptor fields. The fixture now matches the Agent descriptor contract; rerun passed.
- Native strict unknown-field regression caught serde unit-variant acceptance of extra
  list fields; the request uses an empty struct variant and now rejects them.
- Workbench macOS Tauri `cargo check --offline`, native Clippy with warnings denied,
  frontend typecheck, shared-asset synchronization, documentation inventory/book,
  and 800/2000 source/document organization checks passed. No full app rebuild/install.
- Browser recovery and bridge tests are registered in the native test runner's UI
  suites; the browser entry is also in the desktop UI validation contract.
- Tensor structure passed with zero structural gaps. Global maturity/evidence gaps
  remain (four maturity gaps, twenty evidence-grade gaps, fifteen P0 gaps); this
  bounded source evidence does not relabel the entire system as operationally proven.

## Follow-up: Definitive Admission Rejection

This section records the first rejection iteration. Busy/drain persistence was
subsequently strengthened as described in the next section; transient capacity
decisions retain the safety boundary documented here.

The run chain now distinguishes a confirmed pre-admission rejection from an unknown
transport outcome. Previously busy/draining/capacity rejection left an unrecoverable
caller reservation even though no execution had started.

- Async Agent submission returns a separate admission-rejection schema only after
  checking existing identities and orphan receipts, and before any admission write.
  Duplicate identities still return their retained state; conflicts, corruption and
  current/prior write failures remain uncertain. The synchronous wrapper keeps its
  existing error contract. No rejected task is queued, run or persisted.
- The native RPC transport validates the rejection against the live request ID,
  execution ID and method, plus schema, producer/digest shape, bounded message and
  no-start/no-replay flags. It rejects lookup substitution, inconsistent envelopes
  and extra fields. This is a correlation check on a trusted transport, not a signed
  attestation or authentication of the raw Agent listener.
- The frontend accepts rejection only on submission, under the original gateway
  authority, and removes only its owned reservation. A locally detected authority
  change before dispatch also releases that new unsent reservation. An existing
  reservation owned by another view is never removed. No automatic retry is added.
- Lost replies, malformed decisions and cleanup failures remain blocked. Cleanup
  failure has an explicit `direct:journal_cleanup_pending` error, rather than a
  misleading completed run or permission to start another calculation.

Verification of this follow-up:

- 199 focused frontend regression tests passed, including malformed rejection,
  lost reservation acknowledgement, concurrent callers, authority changes and failed
  cleanup. Unknown/missing execution receipts still cannot retire a reservation.
- 463 Rust tests passed across CLI, the three durable Agent live suites, native
  desktop runtime and protocol; one existing whole-stack test remains ignored.
  Live Agent coverage includes busy/drain rejection, manual submission after restart,
  prior-write-failure blocking and existing kill/restart/no-replay cases.
- Five Chromium scenarios passed. The added scenario uses the actual PWDT run chain
  and IndexedDB, observes rejection, reloads, and explicitly starts a fresh successful
  execution. HTTP is mocked; this is not installed WebView-to-real-Agent qualification.

At that iteration, rejection was not durably recorded by the Agent. If its response was lost or the caller
dies before cleanup, a later `not_found` is still not proof that it never ran. This
follow-up does not solve that remaining reconciliation boundary by weakening safety.

## Follow-up: Retained Pre-Execution Failure

Busy/draining execution-slot rejection now commits the original identity and a
normal terminal failed receipt before sending its response. The nested error is
`durable_admission_rejected`, with `execution_started: false`. No execution slot,
solver invocation, queue or new persistence format is created. The existing locked,
checksummed, synchronized ledger stores two bounded files under its existing limits.
Each retained failure consumes one entry; rejection spam cannot grow an unbounded
new log. Capacity rejection remains transient and never evicts older identities.

The same execution ID remains failed after restart, including both sync and async
duplicate submissions. A new explicit attempt requires a new ID. The caller can
reconcile a lost rejection reply or interrupted local cleanup by inspecting the
retained terminal receipt; inspection does not replace the model/result. Failed
run feedback now also surfaces `direct:journal_cleanup_pending` when cleanup fails.
Identity or receipt write failure remains uncertain, blocks new admission, and
never falls back to a transient "safe to release" assertion.

Verification:

- 202 focused frontend regression tests passed, including retained failure handling,
  lost reply reconciliation, cleanup failure feedback and exact subsequent cleanup.
- 466 Rust tests passed across CLI, protocol, desktop runtime and the three durable
  live suites; one existing whole-stack test remains ignored. Added store tests
  exercise reopen/deduplication, bounded files, partial rejection commits and write
  failure fencing. The live test holds delivery after rejection receipt sync, force
  kills the Agent before the caller receives it, then verifies two restarts, index
  discovery and sync/async duplicate submission without executing the rejected ID.
- Seven Chromium scenarios passed, including lost rejection response and injected
  IndexedDB cleanup interruption, reload, pending-identity protection, explicit
  receipt inspection and a fresh manual run. Browser HTTP is mocked; actual native
  Agent termination is tested separately, not as one installed WebView end-to-end run.

Remaining boundaries: if storage is full, unavailable, or only partially committed,
no retained rejection is promised. Lost local reservation acknowledgement is also
not resolved by an absent Agent record. No physical power-loss certification,
automatic replay, new large input copies, or operational tensor promotion is claimed.

## Follow-up: Worker Startup Ownership Handoff

An accepted task previously lost its live owner when thread creation failed: dropping
the spawn closure released its captured slot and admission guard, leaving an unknown
record even though no solver had started. The submitting side now retains those guards
until worker creation succeeds and transfers them through a one-item ownership channel.
Failed transfer returns ownership; it does not enqueue another execution or retry.

Confirmed pre-dispatch owner/worker startup failure commits a normal failed receipt,
`durable_execution_start_failed`, with `execution_started: false` and phase
`before_engine_dispatch`. The slot remains held through commit; the watchdog records
the startup failure instead of a solver completion. Lost replies can be reconciled by
the retained receipt. Failed receipt commits remain uncertain and fence new admissions
in that process. Restart never converts an admission without a receipt into a safe retry.
The engine, operators and persistent file format remain unchanged.

Verification of this follow-up:

- 469 Rust tests passed across CLI, protocol, desktop runtime and the three durable
  Agent live suites; one existing whole-stack test remains ignored. Three new live
  scenarios cover retained startup failure and exact owner release, kill before reply
  followed by two restarts, and a failed receipt write followed by restart without replay.
  Old-identity cancellation cannot target a new execution reusing transport/job IDs.
- Thread startup failure is injected by an opt-in exact execution ID configured only
  for the test-owned Agent. It exercises the pre-spawn error path, not real OS resource
  exhaustion. The process termination is an actual kill of the owned child, not a mock.
  The existing post-sync hold verifies that drain does not declare the owner replaceable
  before release. No physical power-loss guarantee is inferred.
- 205 focused frontend tests passed. Both admission rejection and startup failure
  exercise failure feedback, lost-response inspection, interrupted cleanup and exact
  subsequent cleanup without automatic re-submission. HTTP fixtures are mocked.
- CLI/live-test Clippy with warnings denied, Rust formatting, frontend typecheck,
  documentation book/inventory, and the 800/2000 organization guard passed. The
  tensor structural check passed with zero structural gaps; its four maturity gaps,
  twenty evidence-grade gaps and fifteen P0 gaps remain, and Daji status stays blocked.

This adds bounded recovery evidence, not installed WebView qualification or a global
operational-grade tensor promotion. No additional model copies, backup generations,
package rebuild, version change or Git commit was performed.

## Follow-up: Async Worker Unwind Recovery

Previously an async worker panic released live ownership but left only an admission
and an in-memory watchdog diagnostic. The async owner now catches Rust unwinds around
request dispatch and commits a normal failed receipt, `durable_execution_panicked`.
Request-local results are discarded, the execution slot remains held through the
commit attempt, and the watchdog records failure rather than completion. No poisoned
shared lock is repaired or bypassed. Solver algorithms and the ledger format are unchanged.

The receipt explicitly says execution may have started and external effects are not
rolled back. It never grants automatic replay or copies arbitrary panic payloads into
the receipt. Existing local panic-hook logging is unchanged. Failure to write the
receipt keeps the original identity unknown and blocks new admissions in that process;
restart cannot turn that identity into a fresh execution.

Verification:

- 476 Rust tests passed across CLI, protocol, desktop runtime and four durable live
  suites; one existing whole-stack test remains ignored. Unit tests cover actual unwind
  cleanup, fixed error payloads and unchanged ordinary success/error responses. The new
  live suite covers result discard, exact owner release, old cancellation fencing,
  force kill after receipt sync with two restarts, and failed receipt commit/restart.
- The exact-ID opt-in hook panics after a real dispatch returns, before durable commit.
  It tests the wrapper's unwind path, not every numerical solver's internal panic site.
  Shared durable fixtures were extracted instead of growing one live test file past
  the source limit. Test processes and their private stores are cleaned up afterward.
- 208 focused frontend tests passed. Panic failures share exact journal acknowledgement,
  lost-reply inspection and interrupted-cleanup coverage with other retained failures.
- Eight Chromium scenarios passed, including detached observation, a later retained
  panic failure, page reload, exact inspection/cleanup and a fresh explicit run. No
  failure result is published and no automatic resubmission occurs. This uses actual
  source GUI/PWDT actions and IndexedDB with mocked HTTP, not an installed WebView
  connected to the real Agent or a loaded Python interpreter.
- CLI/live-test Clippy with warnings denied, Rust formatting, frontend typecheck,
  documentation book/inventory and the 800/2000 organization guard passed. Tensor
  checks retain zero structural gaps, four maturity gaps, twenty evidence-grade gaps
  and fifteen P0 gaps; Daji status remains blocked.

This boundary does not catch process kills, abort-on-panic builds, memory exhaustion,
double panics, ledger-commit panics or synchronous execution panics. Nor does it certify
the effects of an external operator subprocess. Those are not relabeled as successful
recovery, and this evidence does not promote the global tensor to operational grade.

## Follow-up: Durable TaskIR Cache Publication

A new live regression reproduced contradictory recovery responses: after native TaskIR
execution succeeded and the async worker panicked before durable commit, the ledger
reported `durable_execution_panicked` while `fetch_operator_task_result` exposed the
successful solver result. TaskIR had published its cache response inside dispatch,
before the durable owner decided and committed the authoritative outcome. Receipt
write failure had the same premature-success window in sync and async execution.

Both durable paths now retain the TaskIR cache reservation in the outer owner. Only
a committed durable receipt supplies the cached response; async unwind therefore
publishes the committed failure, not the discarded solver success. The response is
serialized directly from the existing report under the unchanged cache byte limit,
without cloning the full result into another snapshot. Publication still precedes
transport delivery, so a lost reply does not remove an already committed result.

An abandoned reservation clears only its matching pending entry to
`result_not_retained`. Cleanup cannot change another identity/generation, an ambiguous
attempt or a terminal receipt, nor resurrect an expired/evicted entry. Normal
non-durable execution retains its previous completion-before-delivery behavior.
Restart still discards transient cache entries. Duplicate durable submissions return
the existing ledger record without rebuilding the cache or re-executing the solver.
The descriptor exposes this boundary under `durable_execution.task_result_cache`.

Verification of this follow-up:

- The new panic/cache regression failed before the production fix and passed afterward.
  Five dedicated live scenarios use the actual native Agent and `solve.bar_1d` TaskIR:
  panic/result disagreement; failed commit in sync/async and panic paths; committed
  sync/async publication while reply delivery is held; cancellation; and ordinary
  execution without a durable ledger. Pending entries expose no response, successful
  receipts retain the expected stress value, and restart/duplicate checks forbid replay.
- Four new unit tests cover reservation abandonment, actual Rust unwind before a
  response exists, stale/ambiguous/expired/evicted/terminal entry fencing, and bounded
  encoding of committed success/failure/oversize responses.
- 487 selected Rust tests passed across CLI, protocol, desktop runtime and five durable
  live suites; one existing whole-stack test remains ignored. The new live binary
  includes its five scenarios and two shared support tests.
- 208 focused frontend tests passed. Frontend typecheck, CLI/live-test Clippy with
  warnings denied, Rust formatting, documentation book/inventory, the 800/2000
  organization guard and whitespace checks passed.
- Tensor structural checks passed with zero structural gaps. The existing four
  maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps remain; Daji status
  stays blocked. Evidence was extended, not promoted to operational grade.

No numerical algorithm, ledger file format, package, version or Git commit changed.
No additional durable copies or backup generations were introduced. This follow-up
did not rerun the installed GUI or the browser fixture; it adds real Agent cache/ledger
consistency evidence, not physical power-loss or whole-stack qualification.

## Follow-up: Synchronous Durable Owner Lifecycle

A live regression reproduced another contradictory status: a synchronous durable
TaskIR receipt write failed, the RPC correctly returned an error and the ledger
reported `outcome_unknown`, but the watchdog counted one successful execution. The
reply writer owned completion and treated delivery of that error response as success.
The regression failed with completed count 1 instead of 0 before the production fix.

Synchronous durable calls now retain their own execution guard through receipt commit.
Only after commit can completion pass back to normal reply delivery. A commit error
records the generation-bound live diagnostic `durable_receipt_commit_failed` before
the error response is sent. An earlier solver/cancellation failure is preserved and
not counted twice. Callers without a transport also defer completion until commit.
Unwinding drops the owner with `durable_execution_aborted`; it does not leave a phantom
active watchdog entry or attempt to manufacture a durable panic receipt.

The original synchronous heartbeat, disconnect cancellation and final-delivery lease
remain intact. A committed result whose reply is still held keeps the Agent unsafe to
replace until delivery finishes. Missing receipts remain unknown across restart;
neither a live diagnostic nor an error response grants replay permission. Operators,
numerical algorithms and persistent file formats are unchanged.

Verification:

- The original live TaskIR failed-commit regression now checks watchdog completion and
  failure counts in synchronous, asynchronous and async-panic paths and passes.
- Four new live scenarios cover native synchronous write failure and restart without
  replay; committed success held before reply with drain protection; exact cancellation
  followed by failed receipt commit; and actual TCP disconnect during held native and
  TaskIR execution, followed by a fresh successful execution reusing request/job IDs.
  The new live binary also includes two shared support tests. Its private Agent stores
  and processes are removed by fixture teardown.
- Five unit tests cover no-transport completion, commit failure, preservation of an
  earlier terminal cause, actual owner unwind with request-ID reuse, and network reply
  ownership. The unwind test covers owner cleanup, not every solver panic location.
- 517 selected Rust tests passed across CLI, protocol, desktop runtime, six durable
  live suites and the exact-cancellation/lifecycle/shutdown suites. One existing
  whole-stack test remains ignored. This is not a claim of total project coverage.
- 208 focused frontend tests passed. CLI/live-test Clippy with warnings denied,
  formatting, frontend typecheck, documentation book/inventory, the 800/2000
  organization guard and whitespace checks passed. Tensor structure passed, retaining
  four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps; Daji stays blocked.

This follow-up adds no backups, result snapshots or extra ledger files. It does not
qualify physical power loss, installed WebView execution or remote deployment. No
package rebuild, version change or Git commit was performed.

## Follow-up: Isolate Unrecordable Responses From Storage Failure

A new regression reproduced an isolation bug: a response over the 8-MiB receipt
limit triggered the store's write-failure latch even though receipt preparation
had not touched the filesystem. A subsequent unrelated admission was rejected.
The test failed before the fix. Restart-unreadable, deeply nested responses had
the same failure mode. The previous conservative policy is refined here, not
treated as successful large-result persistence.

Receipt preparation now has typed byte-limit, serialization and restart-decoding
errors, separate from the immutable disk commit. An unrecordable response is
discarded and replaced with a compact failed response, code
`durable_response_unrecordable`. Its details explicitly deny rollback and automatic
replay and acknowledge that execution may have started. No original payload,
partial result, hidden artifact or backup copy is retained.

The replacement must pass the same preparation, checksum, file-sync, rename and
directory-sync boundary before it becomes an authoritative failed receipt. Only
then may unrelated admissions continue in the same process. A failed replacement
commit still leaves `outcome_unknown` and fences new admissions; real storage
errors are never retried with another payload. Both durable owners now account for
the committed response in the watchdog, and TaskIR publishes that same response
through its existing deferred cache. An earlier solver/cancellation diagnostic
keeps its first cause and is not counted twice.

Verification:

- Store tests cover the red-to-green admission regression, bounded compact receipts,
  restart and duplicate identity, deep restart-decoding rejection, failed replacement
  writes, sibling receipt reservations, exact serialization boundaries and a fake
  serializer whose misleading error text must not be classified as a byte limit.
- The new native Agent live suite runs actual 100,000-element bar solves, one at a
  time, in synchronous/asynchronous and native/TaskIR combinations. All produce a
  compact failure, no successful watchdog count, and a successful small follow-up
  task in the same process. Duplicate submissions never replay; restart preserves
  the failure while discarding the optional process-local TaskIR cache.
- A real child-process kill after compact receipt commit but before reply delivery
  retains the failed receipt across restart. Drain remains unsafe until the owner
  releases. Synchronous and asynchronous obstructed replacement writes stay unknown
  and block admissions, including after a successful solver return. The live binary
  has three scenario tests and two shared support tests; fixture teardown removes
  its temporary stores and processes, not user data.
- 528 selected Rust tests passed across CLI, protocol, desktop runtime, seven durable
  live suites and cancellation/lifecycle/shutdown suites. One pre-existing whole-stack
  test remains ignored. All 211 focused frontend tests passed, including failed-response
  feedback, lost-reply inspection, and retrying only exact local journal cleanup.
- Frontend typecheck, Clippy with warnings denied, Rust formatting, documentation
  book/inventory and the 800/2000 organization guard passed. Tensor checks retain zero
  structural gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps;
  Daji status remains blocked. Evidence is extended without a maturity promotion.

The 8-MiB limit, ledger format, operators and numerical algorithms are unchanged.
This is response-failure isolation, not large-result artifact support or numerical
qualification of the large fixture. No installed-GUI, remote-host or physical
power-loss qualification is claimed. No package rebuild, version change or Git
commit was performed.

## Follow-up: Validate Durable Response Semantics End to End

New regression tests exposed a gap between decodable JSON and valid terminal evidence.
The store accepted unsupported RPC versions and contradictory response states. It also
accepted a success with `Some(Value::Null)`, although a serialization round trip turns
that into an absent result. All three regression tests failed before the fix. These
are synthetic boundary cases, not claims that an existing numerical solver has been
observed returning these malformed responses.

Write preparation now validates the response envelope and forbids a null top-level
successful result before committing it. A rejected new response follows the existing
compact `durable_response_unrecordable` policy with reason `invalid_response`. Its
replacement must still commit successfully; otherwise the outcome stays unknown.
Wrong original request identity remains a strict identity error, not permission to
manufacture a terminal failure. Valid nested nulls, non-null scalar/array results and
ordinary solver failures are preserved.

Retained receipts are validated on read as well as checksum verification. Invalid old
records cause inspection, duplicate submissions and exact cancellation to fail. They
are not rewritten into new failures, erased or replayed. Admissions remain indexable,
and an unrelated healthy execution can still run. This is not numerical result-schema
validation or a data-repair/migration operation.

The native gateway and frontend had related weaker checks: the gateway accepted empty
failure code/message strings, while the frontend accepted whitespace-only ones. The
new regressions failed before both were tightened. Neither layer may use such evidence
to authorize terminal acknowledgement or caller-journal cleanup. No automatic retries
or extra disk artifacts were introduced.

Verification:

- Six new store tests cover eight malformed response variants, immediate/reopened
  lookup and duplicate fencing, valid result preservation, replacement write failure,
  and wrong-request identity. The null test explicitly demonstrates the `Option<Value>`
  round-trip difference rather than relying only on a fixture label.
- A new actual-Agent live scenario exercises eight synthetic checksummed old-writer
  variants. Each starts from a real bar solve; fixture mutation occurs only while its
  owned Agent is stopped. Across two subsequent restarts, invalid records remain
  unchanged and unreadable as terminal evidence, while a new task reusing the old
  request/job names completes with the expected stress. Cancelling the invalid old
  identity cannot affect that new task. The binary also runs two shared support tests.
- A native gateway TCP-peer test rejects eight empty/whitespace failure combinations
  with a non-replayable error. Eight frontend tests retain the exact pending journal
  through failed inspection, rejected acknowledgement and service recreation, and
  block a replacement submission. These are protocol/unit tests, not installed GUI tests.
- 538 selected Rust tests passed; one pre-existing whole-stack test remains ignored.
  All 219 focused frontend tests passed. These totals include earlier regression
  suites, not 538 newly added cases or complete project coverage.
- Frontend typecheck, Clippy with warnings denied, Rust formatting, documentation
  book/inventory, whitespace and source/document line-limit checks passed. Tensor
  checks retain zero structural gaps, four maturity gaps, twenty evidence-grade gaps
  and fifteen P0 gaps; Daji status remains blocked, without maturity promotion.

The durable file format, version and numerical implementations are unchanged. This
follow-up does not certify physical power loss, full GUI deployment or numerical
accuracy of arbitrary payloads. No package rebuild or Git commit was performed.

## Remaining Boundaries

Not qualified: physical power loss, browser storage eviction, installed WebView through
the full live Agent stack, remote hosts, full durable workflow graphs or multi-Agent
durable selection. Tests do not certify numerical accuracy beyond existing solver tests.
The raw Agent listener still needs a trusted network boundary. No automatic replay,
mid-solver continuation, arbitrary journal deletion control or operational-grade
tensor promotion is introduced. Clearing an unknown reservation is not offered as a
normal retry action. Native caller identities store gateway-address hashes, not auth
tokens; server authorization is still required for every read/cancel.
