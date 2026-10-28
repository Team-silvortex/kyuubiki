# Durable watchdog cancellation intent

Date: 2026-10-13. Source overlay on `c2f97f47`, daji 3.6.0.

Checkpoint note: the later [runner deadline extension](workflow-deadline-retirement-20261013.md)
reuses this protocol for execution-runner expiry. Results and watchdog-only scope
below are retained as the earlier checkpoint, not overwritten retrospectively.

## Closed window

The earlier [captured-request cancellation checkpoint](workflow-scoped-cancellation-20261013.md)
prevented job-wide cancellation from selecting later work, but its snapshot was
ephemeral. Loss after a periodic watchdog failure commit, before cancellation
delivery, could leave the original Agent request running with no retained intent.
Seed 68187 reproduced the missing intent: one test, one failure on the old path.

Periodic watchdog workflow retirement now records a bounded private cancellation
intent in the same transaction as job/recovery failure. The capacity gate is read
without changing ownership before this commit. Rejected writes leave live work
and its gate reservation intact. Only after the joint commit does local shutdown
occur and the bounded cancellation sweep start.

## Contract

- The sealed intent is bound to job ID and creation time, envelope digest,
  recovery generation, owner session and a fresh intent nonce. It stores original
  RPC IDs and endpoint fingerprints, not addresses, credentials, payloads or raw
  transport errors. Existing private recovery storage is reused; no new database,
  migration, backup or dependency is introduced.
- At most 128 entries, 32 endpoint identities and 256 KiB encoded intent bytes
  are accepted. Duplicate, malformed, oversized or mismatched records fail closed.
- Each attempt is reserved durably before observation. Only the still-configured
  original endpoint identity/address/session can be contacted. The first valid
  observation must match both original request and job. Its process instance and
  generation are committed before cancellation is authorized.
- Once bound, retries preserve that exact target and do not probe for a new
  generation. A restarted/replaced Agent can answer `target_not_observed`; this
  cannot authorize cancelling its newer execution. Later same-job requests remain
  outside the retained request set.
- Reservation, binding, authorization and receipt persistence pass through the
  owning coordinator and current Orchestra lease. Writes compare both job and
  result snapshots. Delivery nonces and coordinator sessions reject stale replies.
  This is not atomic remote lease fencing: authority may change after the final
  local authorization check and before the exact-target network operation.
- A lost reply or rejected acknowledgement write remains uncertain. A later
  sweep can resend the same target. At most three reserved attempts are allowed,
  including reservations interrupted by crashes. Exhaustion remains `unresolved`,
  not success. Public summaries expose bounded status counts, never target IDs or
  endpoint credentials; an acknowledgement is not proof of execution termination.
- Post-commit delivery, coordinator startup and explicit `recover_now` can start
  one supervised sweep per BEAM instance. A sweep considers at most 16 pending
  jobs with a 15-second delivery-start budget and bounded transport calls. This
  is not a hard wall-clock deadline covering storage, DNS or scheduling. Concurrent
  kicks while that worker is occupied are not queued; remaining intents require
  a subsequent startup or explicit recovery sweep. There is no retry daemon.

The engine and physical operators are unchanged. Exact Agent cancellation remains
in the generic transport/control layer rather than entering solver implementations.

## Verification

Added 21 tests across intent validation and durable cancellation recovery, and
updated two existing native watchdog tests to wait for persisted acknowledgements
before comparing restart state. Their original blocked evidence is still compared
unchanged; acknowledgement does not rewrite native dispatch observations.

- Focused SQLite, seed 68189: 47 tests, zero failures, 3.2 seconds.
- Expanded SQLite, seed 68190: 3,225 total, 3,152 passed, 73 conditional skips,
  zero failures, 58.7 seconds, warnings denied.
- Isolated memory, seed 68191: 213 total, 188 passed, 25 conditional skips,
  zero failures, 5.9 seconds, warnings denied.
- The crash-before-delivery test disables the delivery supervisor after a real
  controlled TCP request begins, commits watchdog failure, removes the old local
  owner and starts a later same-job request. Coordinator restart cancels only the
  original request; the later one returns normally.
- A saved target with a different generation is sent unchanged after restart,
  despite the Agent reporting an active current generation. It is acknowledged
  only as `target_not_observed`, never rebound.
- Fault injection rejects failure commit, attempt reservation, target binding and
  acknowledgement persistence. Failed pre-send writes cannot authorize delivery;
  a lost acknowledgement write permits only an exact-target retry. A token from
  the old coordinator cannot finalize the new attempt.
- Other cases cover malformed tokens without coordinator crashes, damaged intent
  hashes, changed endpoint sessions, exhausted retry budgets, parent identity
  mismatches, unknown outcomes and credential-free bounded public summaries.

The controlled Agent fixture is a TCP protocol double, not a Rust solver. These
tests do not establish remote shutdown, real power-loss durability, installed
desktop behavior, live PostgreSQL or numerical correctness.

Formatting and whitespace checks, tensor validation/self-test, the 26-file HTML
book, documentation inventory and project organization checks passed. Source and
document limits remain 800/2,000 lines with zero tracked debt. Tensor maturity is
unchanged: zero structural gaps, four maturity gaps, 20 evidence-grade gaps and
15 P0 gaps; the release gate remains blocked. No native binaries were rebuilt or
installed. The dedicated temporary memory-test directory was removed afterwards.

Reproduce the broad SQLite selection from `apps/web`:

```text
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery*test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage test/kyuubiki_web/jobs \
  --seed 68190 --warnings-as-errors
```

## Remaining limits

This durable path covers periodic watchdog retirement of managed workflows only.
Runner-local deadlines, ordinary failure cleanup and explicit user cancellation
are not promoted to this durable intent protocol. Historical failures are not
backfilled. Work whose original RPC identity was already lost before snapshot
capture cannot be reconstructed safely by guessing from its job ID.

The process/generation target is observed during cleanup, not durably attested at
dispatch admission. A missing or invalid observation remains unresolved; no
job-wide fallback, automatic replay, package cleanup or terminal proof is added.
Bounded sweeps do not guarantee automatic progress through an arbitrary backlog.
These local control-plane checks do not close the broader distributed recovery
or release-readiness gaps and do not resolve previously noted lease-admission
intermittency.

## Durable Directory Handoff Follow-Up

Date: 2026-10-26. Package metadata remains 3.7.1. This follow-up concerns the
durable expiry delivery worker, complementing the earlier
[immediate cleanup directory check](workflow-scoped-cancellation-20261013.md#live-directory-follow-up).
Earlier report results and scope remain historical; their counts are not added.

The worker previously read configured endpoints once per job, before reserving
individual attempts. A node could disappear or change address/session while an
observation, reservation, target binding or authorization was waiting. Later
requests, including cached targets in the same batch, could still be sent using
that stale list. A slow reservation could also start a probe after the sweep's
15-second delivery-start budget had expired.

The initial isolated regression contained 19 tests: 18 failed against the old
implementation and the unchanged membership control passed. After repairing the
test probe cleanup, the failures were reproduced with seed 271026 without the
cleanup-induced coordinator failures from the first test-development attempt.
The tests deliberately pause actual coordinator and pool calls, rather than
depending on sleep-based race timing. One budget control intentionally waits
15.1 seconds to exceed the production sweep budget without a test-only clock.

### Delivery Changes

- Immediate and durable cancellation reuse `AgentCancellationDirectory` to resolve
  the original fingerprint against current static or registry membership. Lookups
  use the original deadline, cap directory waits by the remaining budget and five
  seconds, and do not use stale addresses or static fallback in registry mode.
- Durable delivery resolves membership after each successful reservation and again
  after target binding and the existing authorization. A cached description does
  not retain membership authority. Each distinct endpoint is still described at
  most once within the job's sweep; bound retry targets are never reobserved.
- Since the final directory lookup may wait, the worker then authorizes the same
  persisted delivery token again. Reservation replacement or intent corruption
  during lookup prevents sending and does not overwrite the replacement/corrupt
  evidence. Membership and storage are separate point-in-time checks, not one
  atomic distributed transaction; either can change after its last check.
- Observed targets are bound before final membership validation. If membership is
  lost, subsequent explicit recovery keeps the original process/request/job and
  generation rather than discovering a new execution. Restoring membership and
  restarting the coordinator retries only that already bound target.
- New private reason values `endpoint_directory_unavailable` and
  `delivery_budget_exhausted` are sealed into the existing intent lifecycle.
  Reservations consume the existing three-attempt limit; exhaustion is unresolved,
  not acknowledged. This includes directory failure before observation. Existing
  record fields, identifiers, limits and old reason values are unchanged; older
  binaries are not promised to accept new reason values on downgrade.
- After a send, an exact positive acknowledgement, a negative acknowledgement and
  a disconnected reply retain requested, target-not-observed and unknown outcomes
  respectively. Failure of the next directory check cannot rewrite them as not
  sent. No new retry daemon, peer fallback, job-wide cancellation or execution
  replay is introduced. Acknowledgement still does not prove termination.

### Tests And Scope

There are 27 new tests: 25 durable directory/authority integration cases and two
intent reason lifecycle cases. They cover removed/replaced endpoint identity,
address and session; reservation/bind/authorization waits; already bound retries;
cached batches; independent peers; registry removal and re-registration; directory
process loss; genuine socket disconnect after cancellation; restored membership
after coordinator restart; replaced reservations/corrupt intent during final lookup;
budget expiry; and healthy exact-send controls with same-job bystanders excluded.

The fixture writes valid failed-workflow intents directly and uses a controlled
loopback TCP Agent. It does not execute a numerical model or prove initial
dispatch attestation. Existing gated watchdog and live-request recovery tests are
retained in the regression selections to exercise the normal capture path.
The new test-only pause helper releases when its owning test exits and is removed
after each test. The existing TCP fixture can now disconnect without replying,
allowing acknowledgement loss to be tested without killing its whole listener.

Completed selections with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| Initial focused immediate/durable cancellation selection | 85 passed, zero failures | 271027 |
| Final expanded workflow/API/storage/dispatch selection | 3,528 passed, 73 conditional skips, zero failures (3,601 total) | 271028 |
| Isolated memory cancellation/watchdog/deadline selection | 127 passed, zero failures | 271029 |
| Solver completion API and RPC envelope classification | 21 passed, zero failures | 271030 |

The expanded first run found two stale API assertions among 3,601 tests, with 73
conditional skips. Missing/null success results are already rejected by the
strict RPC envelope validator as protocol failures, before solver object checking.
The API tests now require that exact earlier failure classification; non-null
array/scalar/text/boolean results still require solver-result rejection. Assertions
for no stored result and successful subsequent requests remain. No production
validator was relaxed and no failed case was removed.

The complete expanded selection was rerun with the same seed after that test
alignment: zero failures in 137.3 seconds. Formatting, diff whitespace, the
26-file HTML book, documentation inventory, organization audit and tensor
validation/self-tests pass. Source/config and document limits remain 800 and
2,000 lines with zero tracked debt. Tensor status remains 13 modules and 11
paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade gaps and
18 P0 gaps; the release gate is still blocked. The isolated memory run's temporary
data directory is removed after verification.

These selections overlap and are not coverage percentages. This work does not
qualify live PostgreSQL, remote Linux, installed desktop operation, native solver
accuracy, credential rotation or physical power-loss durability. Storage calls,
DNS and scheduling are not forcibly preempted by the delivery-start budget.
No new cancellation storage bucket, backup, runtime dependency or engine/operator
coupling is added. Explicit public cancellation remains a separate path. No
commit, push, version bump, package rebuild or installation is performed.

## Lease Acquisition Authority Follow-Up

Reviewed October 26, 2026. The previous directory checks still had a distinct
ownership boundary: a delivery token bound the coordinator process session and
persisted intent/delivery nonce, but not the lease acquisition that issued it.
The same coordinator could lose ownership and acquire again while its separate
cancellation worker waited on observation, directory lookup or a TCP reply.
The worker could then use the new valid database lease with its old token.

The isolated pre-fix selection reproduced 18 failing cases out of 20 tests
(seed 281001): old bind/authorize/finish operations after release, foreign takeover
or storage outage, and missing/null/foreign acquisition identities. Ordinary
renewal and fresh exact-target reservation controls passed. These are multiple
observations of one authority-lifetime defect, not 18 independent vulnerabilities.

### Lifetime Boundary

- Each successful acquisition creates an ephemeral `lease_session` reference;
  loss or failed acquisition clears it. Ordinary lease renewal preserves it.
  It is separate from the existing coordinator session and workflow claim, so
  those identities and normal heartbeat behavior are not rotated unnecessarily.
- Reservation returns that reference with the existing coordinator session and
  delivery nonce. Bind, authorize and finish compare both local identities while
  retaining the database lease guard and sealed intent checks. An old or missing
  identity returns `stale_cancellation_delivery` without rewriting the intent.
- This also fences reacquisition when storage returns with the same numeric fence.
  The reference is never persisted or sent over RPC, and no schema, storage bucket,
  backup, credential or runtime dependency is added. It is a local lifetime fence,
  not a replacement for the database ownership guard or a restart checkpoint.
- After network submission, late positive, negative or disconnected replies from
  an old worker cannot settle the record under the new lease. The dispatching
  record remains unresolved; a fresh explicit sweep must reserve a new attempt
  and can only use its previously bound process/request/job/generation target.
  The existing three-attempt limit and non-terminal acknowledgement semantics stay.

### Regression Scope

The new 29-case suite covers all three store operations across release, foreign
takeover and storage return in the same coordinator process; malformed authority;
coordinator restart; ordinary renewal; fresh reservation and immutable targets;
absence of the private marker from persisted/public data; and numeric-fence reuse.
Controlled loopback TCP cases hold observation, the final directory lookup, or
cancellation replies across actual lease-store transitions. Old authority cannot
send after the checked transition or settle a late reply; renewal still sends only
the selected execution and a fresh retry neither re-describes nor targets a same-job
bystander. Failed workflow intents are seeded directly, not captured from a new
numerical solve. The test registry guard only suppresses competing startup sweeps.

Completed with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| Initial fixed cancellation selection | 68 passed, zero failures | 281002 |
| Complete new lease/TCP suite | 29 passed, zero failures | 281003 |
| Expanded workflow/API/lease/storage/dispatch selection | 3,566 passed, 73 conditional skips, zero failures (3,639 total) | 281004 |
| Isolated memory cancellation/watchdog/deadline/lease selection | 165 passed, zero failures | 281005 |

The expanded run completed in 140.0 seconds; isolated memory in 26.4 seconds.
These selections overlap and are not coverage percentages. Formatting and diff
whitespace checks, the 26-file HTML book, document inventory, organization audit
and tensor validation/self-tests pass. Source/config and document limits remain
800 and 2,000 lines with zero tracked debt. The new lease suite is 376 lines.
The isolated memory data directory is removed after verification.

The tensor gains the narrowly scoped
`local-durable-cancellation-lease-acquisition-authority` evidence claim, without
upgrading deployment qualification. It remains 13 modules, 11 paradigms, zero
structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; the
release gate stays blocked.

The lease-acquisition check is still point-in-time: it does not make final local
authorization atomic with sending, retract a sent request, prove Agent termination,
or qualify cross-host PostgreSQL, native numerical accuracy, installed applications
or physical power-loss durability. Explicit public cancellation is a separate path.
No commit, push, version bump, rebuild or installation is performed in this follow-up.

## Whole Sweep Authority Follow-Up

Reviewed October 26, 2026. The preceding fix fenced each delivery token, but the
worker's outer entry/job loops could request a fresh token after rejecting the
old one. Consequently a worker launched before lease reacquisition could continue
under new authority, including using an earlier cached Agent description for the
next unbound entry. Single-entry tests did not expose this batch-lifetime gap.

The first 22-case loopback TCP regression reproduced 20 failures with seed 291001;
the two ordinary-renewal controls passed. Failures observed actual unexpected
`describe_agent` or `cancel_execution` messages for later entries/jobs after
takeover or coordinator restart. They are variants of one batch-authority defect,
not 20 independent vulnerabilities.

### Batch Boundary

- Launch pins the initiating coordinator session and lease-acquisition identity.
  Watchdog retirement, runner deadline retirement and startup/explicit recovery
  scans pass their existing state identity directly, avoiding a coordinator
  self-call. Other internal launches capture it through a guarded coordinator
  call before starting the asynchronous worker.
- All worker store operations carry that fixed authority. The scoped store gate
  checks it under the existing lease guard before reading or mutating an intent.
  A stale batch returns `stale_cancellation_sweep`; it cannot consume a fresh
  reservation, bind a target or settle a receipt under the replacement authority.
- Loss of batch/delivery authority, coordinator/ownership unavailability or storage
  failure stops both loops. Unvisited entries keep their zero attempts and pending
  state, while already-reserved or sent entries retain their unresolved evidence.
  Ordinary per-endpoint membership failure still permits independent captured peers
  under unchanged authority, as covered by the retained directory regression.
- Initial discovery can wait too. An expanded 47-case run exposed six additional
  failures (seed 291003): loss during the first directory lookup still permitted a
  read-only probe before the later binding gate rejected the delivery. A scoped
  authority recheck now follows that lookup before observation/cache use. This
  does not claim an atomic relationship between that check and the next network call.
- Fresh explicit recovery starts a new batch and description cache, re-reads the
  retained records and preserves any already bound exact target. Unbound later
  entries use new observations. No solver resubmission, broader cancellation,
  new persisted field, external protocol, storage bucket or retry daemon is added.
  At this checkpoint concurrent kicks remained unqueued; a new kickoff discarded
  while the old worker was registered required a later recovery call. The bounded
  handoff follow-up below supersedes that limitation for retained valid triggers.

### Tests And Scope

The final 47-test suite covers multi-entry and multi-job batches across takeover,
coordinator restart and storage loss/return, while paused at initial directory
discovery, Agent observation, final directory lookup or positive/negative/lost
cancellation replies. It also covers delayed startup with retained authority,
scoped load/reservation rejection before persistence, malformed identities, normal
renewal and fresh-cache recovery with changed observed generations. The latter
keeps an already bound generation 7 target while newly observed unbound entries
use generation 9; same-job bystanders are never selected.

Fixtures seed valid failed-workflow intents directly and use controlled loopback
TCP Agents, actual lease-store release/takeover and coordinator/storage process
restarts. These are local source tests, not numerical model execution or a remote
multi-host deployment. The test-only registry guard suppresses competing startup
sweeps where an explicit delayed launch is being tested. Existing automatic
watchdog/deadline/recovery tests remain in the wider regression selections.

Completed with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| Initial fixed cancellation/registry selection | 122 passed, zero failures | 291002 |
| Final new whole-sweep suite | 47 passed, zero failures | 291003 |
| Expanded workflow/API/lease/storage/dispatch selection | 3,613 passed, 73 conditional skips, zero failures (3,686 total) | 291004 |
| Isolated memory cancellation/watchdog/deadline/lease selection | 212 passed, zero failures | 291005 |

The expanded selection completed in 145.1 seconds and the isolated memory selection
in 33.0 seconds. These overlapping selections are not a coverage percentage.
The memory run uses its own temporary data directory, removed after verification.
Formatting, diff whitespace, the 26-file HTML book, documentation inventory,
organization audit and tensor validation/self-tests pass. The new suite is 375
lines; source/config and document limits stay 800 and 2,000 with zero tracked debt.

The tensor adds `local-durable-cancellation-whole-sweep-authority` as narrow local
source evidence, not deployment qualification. Its aggregate status is unchanged:
13 modules, 11 paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps; the release gate remains blocked.

The final local authorization remains point-in-time, not atomic with sending.
No request already sent is retracted, no acknowledgement proves termination, and
no live PostgreSQL, remote Linux, installed desktop or physical power-loss
qualification is claimed. The existing three-attempt, 16-job and 15-second
delivery-start limits remain; synchronous waits are not forcibly preempted.
No version bump, commit, push, rebuild or installation is performed in this follow-up.

## Bounded Cancellation Handoff Follow-Up

Reviewed October 26, 2026. Whole-sweep fencing prevented an old worker from adopting
new authority, but a valid new trigger could exit on a busy Registry key and leave
no scheduling hint. This included coordinator-startup recovery while an old worker
was still awaiting a socket reply. This follow-up adds bounded handoff inside the
existing coordinator, not another supervised service or durable queue.

### Handoff Contract

- A worker rejected by an occupied sweep slot sends the incumbent PID, job IDs,
  captured authority and original wait deadline to the coordinator. Only current
  owner authority can enqueue. Retained job IDs are validated, deduplicated and
  capped at 128; follow-up workers receive at most 16 IDs each.
- The first retained trigger has at most 30 seconds to wait. Merging more triggers
  or draining later chunks cannot renew that deadline. Monitor and timer tokens
  isolate superseded queues, and exit-before-message ordering is supported.
  Registry cleanup races use a 10 ms bounded recheck under the same deadline.
  Registry lookup itself catches table loss; an earlier availability check alone
  cannot protect the coordinator from a Registry shutdown between the two calls.
- Each handoff rechecks authority and Registry availability, then starts a new
  scoped sweep. It does not transfer an old worker's cached observations or permit
  broader cancellation. Already bound process/request/generation/job identity is
  preserved. Unknown delivery alone does not create another trigger; duplicates
  queued during one attempt coalesce into one follow-up, within the existing
  three-attempt limit.
- Read-only coordinator snapshot `cancellation_handoff` exposes queue limits,
  pending count, waiting state, remaining wait and `last_error`. It excludes job
  IDs, authority and monitor references. Overflow, expiry, invalid identifiers and
  unavailable Registry/worker supervisor are visible; the last error remains until
  replaced or coordinator restart. Omitted or unattempted intents are not mutated.
- Ownership loss cancels timers/monitor and clears the queue. Coordinator restart
  uses retained durable intents under new authority, not old queue tokens. A killed
  active delivery can release the slot for other waiting work without fabricating
  its own acknowledgement. Discarded hints and unresolved records still require
  a later recovery trigger.

The internal `kick` return confirms only trigger-worker startup. It does not confirm
queue acceptance, delivery, remote termination or numerical execution. Waiting
expiry does not forcibly stop a started sweep or replace its existing 15-second
delivery-start budget. This is not global ingress backpressure, continuous retry,
solver resubmission, a new public cancellation API or an atomic network transaction.

### Handoff Validation

The new 22-test suite uses controlled loopback TCP Agents and actual coordinator,
Registry, TaskSupervisor and lease transitions. Cases include normal and killed
incumbent exit before deferred-message handling, restart while waiting, lease
reacquisition during a late reply, duplicate triggers, multiple 16-ID chunks,
overflow, invalid IDs, expired waiting work, stale timer/monitor/authority messages,
service outages, a killed in-flight delivery and unknown-outcome non-retry.

Existing old-worker authority tests now explicitly discard the independently queued
startup handoff while observing the retired worker. Their original no-send and
no-mutation assertions remain intact. The new suite separately verifies that the
legitimate current-owner worker actually runs after handoff. This test isolation
does not disable handoff in production.

The first expanded run (seed 301007) reported two failures in the existing solver
completion API suite while awaiting its first TCP request with ExUnit's default
100 ms limit. That unchanged 14-test suite passed on isolated recheck. Its three
controlled TCP handshakes now use the same explicit 2,000 ms wait as subsequent
completion synchronization; result, cancellation and persistence assertions are
unchanged. These tests establish lifecycle behavior, not a 100 ms latency promise.

Completed with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| New handoff suite | 22 passed, zero failures | 301006 |
| Final expanded workflow/API/lease/storage/dispatch selection | 3,635 passed, 73 conditional skips, zero failures (3,708 total) | 301007 |
| Initial isolated memory cancellation/watchdog/deadline/lease selection | 234 passed, zero failures | 301008 |
| Final isolated memory selection after Registry lookup hardening | 234 passed, zero failures | 301009 |

The final expanded selection completed in 188.2 seconds; the final memory selection
completed in 40.9 seconds. Its isolated temporary data directory was removed after
verification. The overlapping selections are not a coverage percentage or a
performance benchmark. Tests are local source
checks with controlled loopback peers, not live PostgreSQL, remote Linux, installed
desktop, physical power-loss or numerical model qualification.

Documentation book/inventory, formatting, whitespace, organization audit and tensor
validation/self-tests pass. Source/config and documentation limits remain 800 and
2,000 lines with zero tracked debt. The new
`local-durable-cancellation-bounded-handoff` claim adds narrow local evidence; the
tensor remains at 13 modules, 11 paradigms, zero structural gaps, two maturity
gaps, 23 evidence-grade gaps and 18 P0 gaps, with the release gate blocked.
No version bump, commit, push, rebuild or installation was performed.

## Idle Sweep Continuation Follow-Up

Reviewed October 26, 2026. Busy-slot handoff could drain several chunks, but an
idle-start sweep still discarded its unvisited tail on reaching 16 pending jobs
or its 15-second delivery-start budget. A separate inconsistency let duplicate
IDs in one direct trigger consume another cancellation attempt after an unknown
reply, while the queued path coalesced duplicates.

Five new loopback cases failed against the preceding implementation (seed 311001).
The failures showed the missing next Agent request after the job/time limit,
an unintended repeated exact cancellation for a duplicate ID, and absent expiry
or overflow diagnostics because the remaining jobs never reached the queue.
These are two traversal defects and their boundary cases, not five independent
vulnerabilities.

The direct sweep now remembers pending-job IDs visited by one trigger and returns
the unvisited later jobs when reaching its job/time limit. It hands those IDs to
the existing queue while still owning the sweep slot; monitoring handles exit
before or after that message. The queue retains the original authority and wait
deadline, including time spent in the first batch, and still caps pending IDs at
128 and follow-up batches at 16. Overflow/expiry use the same diagnostics and do
not consume omitted jobs' attempts. There is no additional service, storage
bucket, protocol field or unbounded retry loop.

Deduplication retains at most 16 visited pending-job IDs in the current sweep.
Continuation selection skips them, lazily deduplicates later IDs and stops after
128 retained IDs plus one overflow sentinel. It does not build another full-history
set. A sixth new test supplies a lazy tail that raises if traversed beyond that
sentinel, checking bounded selection as well as visited/duplicate exclusion. This
is a metadata-allocation bound, not a deadline for traversing arbitrary duplicate
input or a benchmark of the recovery scan itself.

Budget exhaustion inside a partially visited job yields only later jobs. The
current job is not implicitly requeued, since doing so would retry its unknown
deliveries along with its untouched entries. Those entries remain durable and
require a later recovery trigger. Authority/storage loss still stops traversal
without creating a continuation. Abrupt worker death before returning its private
tail likewise needs later durable recovery; this patch does not introduce a
persisted per-job/per-entry cursor or guarantee delivery across such a death.

The new tests cover an idle 18-job trigger, duplicate IDs after a lost reply,
expired original waiting deadline and overflow with an unattempted retained intent.
The budget test withholds three successful cancellation acknowledgements and uses
the real 15-second sweep budget, not a mocked clock. It verifies that the fourth
job proceeds while
the earlier unknown deliveries each remain at one attempt and a second, untouched
entry in the partially visited job stays at zero attempts. The complete handoff
suite passed all 27 tests with warnings denied (seed 311002, 19.9 seconds).

Completed regression selections with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| Initial expanded workflow/API/lease/storage/dispatch selection | 3,640 passed, 73 conditional skips, zero failures (3,713 total) | 311003 |
| Initial isolated memory cancellation/watchdog/deadline/lease selection | 239 passed, zero failures | 311004 |
| Final expanded selection including bounded continuation selection | 3,641 passed, 73 conditional skips, zero failures (3,714 total) | 311005 |
| Final isolated memory selection | 240 passed, zero failures | 311006 |

The final selections include all 28 handoff tests and complete in 243.3 and 71.1
seconds respectively. These overlapping local source selections are not a coverage
percentage or a performance benchmark. The temporary memory-backend directory was
removed after verification. No remote Linux, live PostgreSQL, installed desktop,
hard power-loss or numerical solver qualification is claimed.

Formatting, whitespace, the HTML book/inventory, project organization and tensor
validation/self-tests pass. The handoff test file is 644 lines; the 800-line source
and 2,000-line documentation limits retain zero tracked debt. The new
`local-durable-cancellation-idle-batch-continuation` evidence claim is local and
narrow: aggregate status remains 13 modules, 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps, with the release gate
blocked. No version bump, commit, push, rebuild or installation was performed.

## Partial Intent Fairness Follow-Up

Reviewed October 28, 2026. Partial-job recovery still required a later trigger, but
each such trigger visited entries in fixed capture order. Slow entries at the front
could spend the 15-second sweep budget repeatedly before untouched entries received
their first attempt. The existing three-attempt cap bounded this delay; it did not
make the per-intent recovery order fair to less-attempted entries.

The new 15-case baseline reproduced nine ordering failures with seed 281002; six
equal-count, exhausted, settled and corruption controls passed. Failures observed
an older exact cancellation before the untouched entry's expected observation/send,
or an older unbound entry selected ahead of fresh work. These are cases of one
ordering weakness, not nine independent vulnerabilities. An earlier test-helper
naming conflict prevented compilation and is not counted as a production failure.

### Read-Only Priority

- After the existing scoped load has verified the intent, select pending and
  dispatching entries and sort by persisted attempts, then original capture index.
  The intent is bounded to 128 entries. Same-count ties preserve capture order;
  entries with fewer nonzero attempts also precede more-attempted retries.
- Do not reorder or add fields to the persisted sealed bucket. Already bound
  process/request/generation/job identities remain unchanged, settled receipts
  are not revisited, and every new reservation still checks current authority and
  the existing three-attempt cap. An exhausted crash reservation cannot authorize
  a fourth send.
- Restart and lease reacquisition derive priority from retained attempt counts,
  without a new cursor, scheduling record, service or storage bucket. Unknown
  outcomes do not themselves create a recovery trigger; sorting does not broaden
  cancellation or replay numerical execution.

### Recovery Tests

The dedicated suite covers pending/dispatching retries with bound and unbound
targets, equal counts at zero/one/two attempts, stable non-lexical ties, and mixed
zero/one/two-attempt entries. A real worker kill leaves one unknown receipt and one
in-flight reservation; the next explicit trigger handles the untouched entry first
while retaining the old entries' exact generation-7 targets. Fresh unbound work
uses a new generation-9 observation. Coordinator restart and lease reacquisition
exercise the same ordering through actual startup handoff.

The real-budget case completes the fresh entry before three older retries consume
the 15-second budget without successful acknowledgements. Exhausted reservations,
settled receipts and corrupt attempt metadata are also checked. The existing
whole-sweep fresh-cache test now expects the untouched entry before the bound retry;
its generation-7/generation-9 target and persisted-attempt assertions remain intact.

The complete cancellation selection passed 167 tests with warnings denied, zero
failures (seed 281003, 62.1 seconds). This verifies ordering through controlled
loopback TCP peers, not remote numerical execution or industrial qualification.

Completed expanded regressions with warnings denied:

| Scope | Result | Seed |
| --- | --- | --- |
| Workflow/API/lease/storage/dispatch selection | 3,656 passed, 73 conditional skips, zero failures (3,729 total; 179.5 seconds) | 281004 |
| Isolated memory cancellation/watchdog/deadline/lease selection | 255 passed, zero failures (69.6 seconds) | 281005 |

These overlapping source selections are not a coverage percentage or a benchmark.
The memory selection does not establish durability after power loss. The change
only prioritizes entries within one verified intent on a recovery trigger; it does
not promise cross-job fairness, automatic partial-job continuation or guaranteed
delivery. No remote Linux, live PostgreSQL, installed desktop, hard power-loss or
numerical solver qualification is claimed.

Formatting, whitespace, the HTML book/inventory, project organization and tensor
validation/self-tests pass. The new suite is 344 lines, with zero tracked debt
against the 800-line source and 2,000-line documentation limits. The isolated
memory test directory was removed after completion. The evidence claim remains
local and narrow: aggregate tensor status is unchanged at 13 modules, 11 paradigms,
zero structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps;
the release gate remains blocked. No version bump, commit, push, rebuild or
installation was performed.

## Busy Duplicate Prefix Follow-Up

Reviewed October 28, 2026. The busy-slot path forwarded the complete incoming ID
list to the coordinator, whose bounded queue took the first 129 candidates before
deduplication. A repeated prefix could consume the candidate allowance, falsely
report overflow and omit later distinct jobs from that trigger. Their durable
intents survived, but they needed another recovery trigger. The idle continuation
path already selected distinct candidates before applying this bound.

Six new regression cases failed with seed 281101: missing later jobs with an
empty or already populated queue, repeated-only input misreported as overflow,
128/129-distinct-job boundary cases, and mixed invalid IDs after a duplicate
prefix. These are six cases of one coalescing defect, not six vulnerabilities.

The busy trigger worker now reuses the existing bounded continuation selector
before sending its handoff message. It retains first-occurrence order and at most
129 distinct candidates, including one overflow sentinel. The coordinator still
validates identifiers, merges against existing work and retains at most 128 IDs.
No full-history copy is sent to its mailbox. The existing lazy-tail regression
checks selection stops after the distinct-candidate sentinel; the selector does
not guarantee a traversal-time bound for arbitrary duplicate input.

The original authority, earliest wait deadline, per-batch limit and attempt cap
are unchanged. Actual overflow leaves omitted work unattempted and recoverable
by an explicit later trigger. A lost acknowledgement remains unknown after one
attempt; repeated IDs alone do not authorize another sweep. No durable schema,
cursor, service, credential or solver replay path is added.

All 34 handoff tests passed with warnings denied (seed 281102, 20.8 seconds),
including the six new cases and existing authority, expiry, worker death and
continuation controls. Expanded regressions with warnings denied also passed:

| Scope | Result | Seed |
| --- | --- | --- |
| Full Web suite | 3,862 total, zero failures, 77 existing conditional skips; 193.8 seconds | 281103 |
| Isolated memory cancellation/watchdog/deadline/lease/restart selection | 210 passed, zero failures or skips; 70.5 seconds | 281104 |

These overlapping local selections are not a coverage percentage or benchmark.
The memory test directory and its two generated analysis-state files were removed
after verification. Formatting, whitespace, HTML book/inventory, project
organization and tensor validation/self-tests pass. The handoff suite is 742
lines, below the 800-line source limit; documentation remains below 2,000 lines.
The narrow `local-durable-cancellation-busy-prefix-coalescing` evidence claim
does not raise maturity: the tensor still has 13 modules, 11 paradigms, zero
structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps, with
the release gate blocked. No version bump, commit, push, rebuild or installation
was performed in this mainline follow-up.

This is local source/loopback evidence, not remote Linux, live PostgreSQL,
installed-runtime, hard-power-loss or numerical-solver qualification.

## Cancellation Input Isolation Follow-Up

Reviewed October 28, 2026. Deferred handoffs already filtered non-binary, empty
and oversized job IDs, but an idle sweep sent each ID directly to the result
backend. A non-binary member raised `FunctionClauseError`, ending that worker
before later valid jobs could receive cancellation. The coordinator and stored
intents survived; this was a missing input boundary on the internal scheduling
path, not evidence of a remotely exploitable public endpoint.

The new 22-case suite reproduced ten failures with seed 281201 (19.3 seconds).
Five non-binary prefix cases, a middle member, invalid-only input, and the
unknown-outcome/stale-authority/storage-outage combinations exercised the same
worker exception. Twelve busy-path, binary-shape and stored-record controls
passed. These are cases of one defect, not ten independent vulnerabilities.

Idle sweeps now reuse the handoff's binary/1..1,024-byte shape predicate before
result lookup. This does not validate or relax Agent execution identities;
the existing sealed intent and exact target checks still authorize delivery.
The deadline check still precedes filtering, and the visited-ID set, job budget,
bounded continuation, queue diagnostics and attempt cap are unchanged. No broad
exception handler, durable schema, new cursor or automatic retry was added.

The tests compare idle and busy paths for nil, booleans, integers, maps, lists,
empty and oversized strings; they also exercise invalid middle members,
invalid-only batches, missing/legacy/corrupt records, the 16-job continuation
boundary, lost acknowledgements, stale authority and stopped storage. Loopback
peers verify the exact cancellation target and retained attempt count. A damaged
record is left untouched, unknown delivery remains pending after one attempt,
and neither storage failure nor stale authority becomes permission to continue
sending. The focused suite passed all 22 tests with warnings denied, zero
failures (seed 281202, 4.2 seconds).

Expanded regressions with warnings denied passed:

| Scope | Result | Seed |
| --- | --- | --- |
| Full Web suite with disposable SQLite | 3,884 total, zero failures, 77 existing conditional skips; 212.0 seconds | 281203 |
| Isolated memory/JSON cancellation, deadline and watchdog selection | 223 passed, zero failures or skips; 69.4 seconds | 281204 |

The first memory invocation was denied a local TCP socket during Mix startup;
no tests ran in that invocation. Re-running with the required permission produced
the result above. Its isolated data directory and two generated state files were
removed afterward. These overlapping source selections are not a coverage
percentage, a benchmark or proof of power-loss durability.

Formatting, whitespace, the HTML book/inventory, project organization and tensor
validation/self-tests pass. The new suite is 275 lines; tracked source/document
limit debt remains zero. The narrow `local-durable-cancellation-input-isolation`
claim does not change maturity: 13 modules, 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps, with the release gate
still blocked. No remote Linux, live PostgreSQL, installed desktop or numerical
solver qualification is claimed. No version bump, commit, push, rebuild or
installation was performed.

## Cancellation Admission Expiry Follow-Up

Reviewed October 28, 2026. Two admission defects remained after input isolation.
An empty/all-invalid handoff could rearm an already populated queue, replacing its
monitor and timer and shortening its deadline despite contributing no valid ID.
A second trigger could also inherit an elapsed queue's deadline if its message
was processed before the already-due expiry message. Valid new work was then
discarded with the old scheduling hint. Durable intents survived both cases;
neither defect itself authorized solver replay or forged a cancellation receipt.

The 11 new admission regressions reproduced five failing cases before the fix
(seed 281301, 3.6 seconds), representing two defects rather than five independent
vulnerabilities. The shorter-invalid-budget case actually expired the legitimate
queue; the longer and empty cases exposed unintended rearming. Two cases exposed
the expired deadline inheritance for a new job and an explicitly retriggered job.
Timer-first ordering, mixed valid input and rejected-trigger controls passed.

The coordinator now leaves queue scheduling state unchanged when filtering admits
no valid ID. For a valid still-live incoming trigger, it checks the previous
queue's monotonic deadline and retires elapsed state before merging. The new
queue contains only the incoming IDs, not the expired queue's unmentioned jobs.
Unexpired queues still merge at the earliest deadline; repeated triggers and
continuations cannot extend that live budget. Invalid-input and expiry diagnostics
remain visible, while stale tokens and monitor messages are harmless. Current
authority, incoming-deadline checks, 128/16-job bounds and the three-attempt cap
are unchanged. No durable field, service, backup or implicit retry is introduced.

Tests suspend the real coordinator to establish both timer/trigger mailbox orders,
then resume it and check exact-target RPCs against a loopback Agent. They verify
unmentioned old intents stay byte-for-byte unchanged until an explicit later
trigger, accepted work uses one attempt, and stale authority, expired incoming
deadlines and invalid-only input cannot revive elapsed work. Late expiry/ready/DOWN
messages cannot erase the new queue. The full input suite passed all 33 tests with
warnings denied (seed 281302, 5.4 seconds).

Expanded regressions with warnings denied passed:

| Scope | Result | Seed |
| --- | --- | --- |
| Full Web suite with disposable SQLite | 3,895 total, zero failures, 77 existing conditional skips; 217.1 seconds | 281303 |
| Isolated memory/JSON cancellation, deadline and watchdog selection | 234 passed, zero failures or skips; 71.6 seconds | 281304 |

These overlapping selections are not a coverage percentage or a benchmark. The
memory run used permission for local loopback sockets and an isolated temporary
data directory; its two generated state files and directory were removed after
the test process exited. Formatting, whitespace, HTML book/inventory, project
organization and tensor checks pass. Production handoff code is 191 lines and
the input suite is 481 lines, both under the 800-line source limit; document
limits and tracked organization debt remain satisfied.

The narrow `local-durable-cancellation-admission-expiry` evidence claim does not
raise maturity: the tensor remains at 13 modules, 11 paradigms, zero structural
gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps, release blocked.
No version bump, commit, push, rebuild or installation was performed.

This is local source/loopback admission evidence, not remote Linux, live
PostgreSQL, installed-runtime, hard-power-loss or numerical-solver qualification.

## Cancellation Capacity Admission Follow-Up

Reviewed October 28, 2026. A full waiting queue could reject every new ID yet
still rearm accepted work using the rejected trigger's holder and deadline.
A shorter incoming budget could then expire the entire retained queue before
its legitimate waiting budget elapsed. Durable intents survived, but delivery
needed another recovery trigger. This is one capacity-admission defect, not a
new cancellation authority or solver replay path.

Eight new regressions reproduced four failures before the fix (seed 281401,
2.0 seconds): shorter-budget overflow expired the retained queue, while
longer-budget overflow unnecessarily replaced its scheduling state. Both were
tested with the real job at the head and tail of a full 128-ID queue. Overlap,
partial-admission and expired-queue controls already passed.

Admission now checks whether any incoming valid ID survives in the retained
128-ID prefix. If none does, the full-queue diagnostic is recorded but the
existing holder, monitor, timers, token and deadline are left unchanged. Empty
and all-invalid inputs remain no-ops for scheduling. Partial admission preserves
the existing order; an incoming duplicate of a retained ID still counts as an
accepted trigger. Accepted triggers keep the earliest deadline. Expired waiting
state is retired before the capacity check, so it cannot block a fresh trigger.
No durable field, service, backup, limit increase or implicit retry is added.

Tests use a real coordinator and sweep-slot holder, persisted cancellation
intents and exact-target loopback RPCs. Syntactically valid missing-job IDs fill
unused queue slots without allocating 128 solver tasks. After rejected overflow,
the entire pending scheduling state must match its previous value, even after
the rejected deadline passes. Accepted jobs complete once; omitted intents stay
byte-for-byte unchanged and use one attempt only after a later explicit trigger.
Overlap controls cover both acknowledgement and a disconnected peer: unknown
delivery stays pending with its bound target and one attempt, without automatic
retry. The input suite passes all 41 tests with warnings denied (seed 281402,
7.6 seconds). These tests are queue-boundary evidence, not a load benchmark.

Expanded regressions with warnings denied passed:

| Scope | Result | Seed |
| --- | --- | --- |
| Full Web suite with disposable SQLite | 3,903 total, zero failures, 77 existing conditional skips; 230.8 seconds | 281403 |
| Isolated memory/JSON cancellation, deadline and watchdog selection | 242 passed, zero failures or skips; 74.5 seconds | 281404 |

The memory selection used permission for local loopback sockets and an isolated
temporary data directory. Its two generated state files and directory were
removed after the process exited. These overlapping selections are not a
coverage percentage or evidence of hard-power-loss durability. Existing skipped
qualification tests remain skipped, not counted as successful executions.

Formatting, whitespace, HTML book/inventory, organization and tensor checks pass.
Production handoff code is 194 lines and the input suite is 635 lines, below the
800-line source limit; document limits and tracked organization debt remain
satisfied. The narrow `local-durable-cancellation-capacity-admission` claim does
not raise maturity: 13 modules, 11 paradigms, zero structural gaps, two maturity
gaps, 23 evidence-grade gaps and 18 P0 gaps; the release gate remains blocked.
No remote Linux, live PostgreSQL, installed-runtime or numerical-solver
qualification is claimed. No version bump, commit, push, rebuild or installation
was performed in this capacity-admission follow-up.

## Cancellation Valid-ID Selection Follow-Up

Reviewed October 28, 2026. Bounded handoff selection counted distinct candidates
before checking their ID shapes. A prefix of 129 different malformed members
could exhaust selection, hide valid later jobs and report false capacity overflow.
The same ordering affected busy-trigger selection, direct internal hints and idle
sweeps handing off their next batch. Durable intents were not deleted, but the
omitted work needed a later trigger despite available valid-job capacity.

A separate 13-test selection suite reproduced 12 failures before the fix (seed
281501, 22.5 seconds). These are variants of one validation/selection-order defect,
not twelve independent vulnerabilities. The stale-authority control passed.

Selection now validates before counting, deduplicates valid IDs in first-occurrence
order and excludes already visited IDs. It stops after the 129th valid distinct ID,
retaining one valid overflow sentinel beyond the 128 queue slots. Encountered
malformed values collapse to one trailing `nil` marker so input diagnostics survive
without copying raw malformed members into the message. The coordinator applies
the same bounded selection before merging and only counts valid IDs for capacity.
At most 129 valid IDs plus one diagnostic marker cross the internal handoff; the
queue itself still retains at most 128 valid IDs. Invalid-only hints do not rearm
accepted work, and true overflow still takes diagnostic priority over invalid input.

The tests exercise integer, map and oversized-string prefixes on both busy and
direct-internal routes, followed by normal or forced holder exit. Boundary cases
retain all 128 valid jobs, reject a genuine 129th and preserve omitted intents
until a later explicit trigger. Real 16-job sweeps continue past invalid prefixes
after both acknowledged and unknown delivery; duplicate IDs cannot replay an
earlier unknown attempt. A lazy-input guard fails if selection reads beyond the
valid overflow sentinel. This bounds retained metadata, not total traversal time
for arbitrary malformed or duplicate input. Missing syntactically valid IDs still
count as candidates; this is not intent verification or public ingress admission.

The selection, input and handoff suites passed all 88 tests with warnings denied
(seed 281502, 37.6 seconds). Exact-target loopback RPCs and persisted counts were
checked; no attempt reset, deadline extension, new durable field or service was
introduced. These are local source regressions, not a scale benchmark.

Expanded regressions with warnings denied passed:

| Scope | Result | Seed |
| --- | --- | --- |
| Full Web suite with disposable SQLite | 3,916 total, zero failures, 77 existing conditional skips; 242.9 seconds | 281503 |
| Isolated memory/JSON cancellation, deadline and watchdog selection | 255 passed, zero failures or skips; 76.8 seconds | 281504 |

These overlapping selections are not a coverage percentage. The memory run used
local loopback permission and an isolated temporary directory; its two generated
state files and directory were removed after the test process exited. No live
development data or unrelated build cache was removed.

Formatting, whitespace, HTML book/inventory, project organization and tensor
checks pass. The production handoff is 207 lines and the separate selection suite
is 328 lines, within the 800-line source limit; document limits and tracked
organization debt remain satisfied. The narrow
`local-durable-cancellation-valid-id-selection` evidence claim does not raise
maturity: 13 modules, 11 paradigms, zero structural gaps, two maturity gaps,
23 evidence-grade gaps and 18 P0 gaps, with the release gate still blocked.

No remote Linux, live PostgreSQL, installed-runtime, hard-power-loss or numerical
solver qualification is claimed. No version bump, commit, push, rebuild or
installation was performed in this selection follow-up.

## Cancellation State Consistency Follow-Up

Reviewed October 28, 2026. Intent verification checked field domains, parent
binding and the digest but did not reject contradictory lifecycle combinations.
Resealed records could claim a failure before any reservation, retain failure
reasons while dispatching/acknowledged, remain pending after all attempts, or
claim premature/unexplained exhaustion. Summaries counted known status labels
without checking the entry, and `finish` accepted the reservation-only exhaustion
reason as a delivery outcome. The SHA-256 seal is an integrity checksum, not a
signature; this follow-up adds consistency checks, not tamper-proof provenance.

The state and recovery selections reproduced 25 failures among 32 tests before
the fix (seed 281601, 0.8 seconds); the seven existing recovery controls passed.
The new cases cover one related state-contract gap across verification, mutation
and presentation, not 25 independent vulnerabilities. A malformed fresh record
actually caused an exact cancellation RPC, and a spent-pending record was rewritten
to exhausted instead of being left invalid. Other failures exposed unsafe admission
and misleading summary status, not additional observed remote sends.

The entry validator now enforces reachable phase combinations: fresh pending has
zero attempts and no reason/nonce/target; retries have one or two attempts with a
delivery-failure reason; dispatching has one to three attempts with no reason;
acknowledged states also need an exact bound target and no reason; exhaustion
requires three attempts and a recognized reason. Crashed reservations remain
valid, including a third interrupted attempt, and a subsequent reservation can
record exhaustion without another send. Only reservation logic may produce
`retry_budget_exhausted`; `finish` now rejects it without mutation. Summaries reuse
entry validation and mark malformed phases invalid without exposing their content.
They still do not verify the entire parent's digest or authenticate its origin.

The finite projection crosses five states, four attempt counts, eleven reason
choices, two nonce-presence choices and two target-presence choices: 880 combinations.
It compares validation against projections generated by legitimate reserve/bind/
finish operations, including failure outcomes and crash reservations. Identity
strings, digest contents, multi-entry interactions and concurrent interleavings
are not exhaustively enumerated; this is a finite regression model, not a formal
proof of the whole recovery system. Separate TCP integration cases exercise startup
and explicit recovery with resealed contradictory records, check no mutation or
cancellation, and verify a healthy separate job still completes its exact request
with one attempt. Existing fairness/crash and immutable-target regressions remain.

The state, intent, recovery and fairness suites passed all 63 tests with warnings
denied (seed 281602, 17.6 seconds). No schema field, storage bucket, retry allowance,
automatic repair or backup was added.

Expanded regressions with warnings denied passed:

| Scope | Result | Seed |
| --- | --- | --- |
| Full Web suite with disposable SQLite | 3,941 total, zero failures, 77 existing conditional skips; 253.1 seconds | 281603 |
| Isolated memory/JSON cancellation, deadline and watchdog selection | 280 passed, zero failures or skips; 77.0 seconds | 281604 |

These overlapping selections and the 880 internal projections are not additive
test counts or a coverage percentage. The memory run used local loopback permission
and an isolated temporary directory; its two generated state files and directory
were removed after the process exited. No live database or unrelated cache was
changed. The data-lifecycle chapter also removes its obsolete statement that busy
kicks are not queued, aligning it with the existing bounded ephemeral handoff.

Formatting, whitespace, HTML book/inventory, organization and tensor checks pass.
The intent module is 277 lines, the finite-state suite 184 lines and the recovery
suite 414 lines, each below the 800-line source limit; document limits and tracked
organization debt remain satisfied. The narrow
`local-durable-cancellation-state-consistency` claim does not raise maturity:
13 modules, 11 paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps, with the release gate still blocked.

No remote Linux, live PostgreSQL, installed-runtime, hard-power-loss or numerical
solver qualification is claimed. No version bump, commit, push, rebuild or
installation was performed in this state-consistency follow-up.

## Cancellation Record Mutation Follow-Up

Reviewed October 28, 2026. This follow-up adds regression coverage for existing
durable cancellation boundaries; it did not reproduce a new production defect or
change runtime behavior. All 26 new cases passed their first run with warnings
denied (seed 281701, 1.6 seconds). This is not a red-to-green repair claim.

The separate `workflow_cancellation_mutation_test.exs` suite covers three layers:

- Eight real loopback TCP cases cross reserve/bind/authorize/finish with concurrent
  job or result annotation edits. The test pauses the coordinator before its
  handler re-reads storage, commits the other writer's edit, then resumes delivery.
  Both annotations and the exact-target receipt survive with one attempt, and a
  healthy second job also progresses. This is not a pause inside the atomic commit.
- Twelve TCP cases cross those same action boundaries with replacement of the
  intent, deletion of the result, or deletion of the job. Replacement uses a newly
  attached, valid intent with another request ID, not an invalid checksum. Old work
  cannot mutate the retained replacement or resurrect deleted data; a healthy
  sibling still receives its exact cancellation. A separate explicit kick handles
  the replacement at attempt one, or leaves deleted records absent. At the finish
  boundary the old RPC has already been acknowledged; rejecting the local stale
  receipt cannot retract that send and is not proof of remote termination.
- Six paired-commit cases cross reserve/bind/finish proposals with newer job or
  result metadata. They submit the stale snapshots directly to the production
  paired-store API, require `stale_job_snapshot` or `stale_analysis_result`, and
  assert neither record is partially overwritten. Subsequent explicit TCP recovery
  preserves the edits, uses attempt one for a rejected reservation or attempt two
  after a persisted reservation, and retains the exact target when already bound.

These tests use actual storage operations and bounded socket exchanges, not a mock
return value for delivery. The loopback peer is a test Agent fixture, not the native
Rust Agent. They do not exhaust every concurrent schedule, inject a write inside a
database transaction, or close the last authorization/send race.
The healthy sibling checks distinguish a per-record conflict from global authority
or storage failure, which must still stop the sweep.

Expanded regressions with warnings denied passed:

| Scope | Result | Seed |
| --- | --- | --- |
| New record-mutation suite | 26 passed, zero failures; 1.6 seconds | 281701 |
| Full Web suite with disposable SQLite | 3,967 total, zero failures, 77 existing conditional skips; 245.7 seconds | 281703 |
| Isolated memory/JSON cancellation, deadline and watchdog selection | 306 passed, zero failures or skips; 89.4 seconds | 281704 |

The selections overlap and are not additive counts or a coverage percentage. The
memory selection uses its own temporary data directory and local loopback permission;
its two generated state files and directory were removed after the process exited.
No retained research, live development data or unrelated build cache was removed.

Formatting, whitespace, HTML book/inventory, project organization and tensor checks
pass. The new test module is 314 lines, below the 800-line source limit. The narrow
`local-durable-cancellation-record-mutation` claim does not raise maturity: 13 modules,
11 paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade gaps and
18 P0 gaps, with the release gate still blocked.

No production API, schema, retry limit, backup or migration is added. Existing
uncommitted mainline changes remain intact. No remote Linux, live PostgreSQL,
installed-runtime, hard-power-loss or numerical solver qualification is claimed.
No version bump, commit, push, rebuild or installation was performed in this follow-up.

## Cancellation Process-Crash Follow-Up

Reviewed October 28, 2026. Previous cancellation regressions killed individual
workers or restarted the coordinator inside one VM. This follow-up starts independent
OS processes, sends real loopback cancellation RPCs, and SIGKILLs the entire child VM
at a verified receipt boundary. A fresh VM opens the same store, without reusing
the old process heap, lease-acquisition references or observation cache.

The first eight cases cross two stores (SQLite and memory/JSON), two crash points
(acknowledged reply before receipt commit, and committed receipt before the next
reservation), and two peer observations (original or replacement process identity).
The parent TCP fixture survives the Orchestra crash. A replacement observation is
simulated by that fixture; this does not restart or qualify a native Rust Agent.

- Before receipt persistence, the first entry remains `dispatching` at attempt one
  with its original process/request/generation/job target. Fresh recovery processes
  the untouched entry first, then retries only that original target at attempt two.
  A replacement peer returns `target_not_observed` for the old target; recovery does
  not borrow its new process identity or select another same-job execution.
- After receipt persistence, the first entry remains `requested` at attempt one and
  is not resent. Only the untouched entry is observed and cancelled. Workflow state,
  workflow generation/attempt and intent ID remain unchanged throughout recovery.
- Two further cases, one per store, kill three successive VMs before the same
  cancellation receipt commits. Durable attempts remain one, two and three; fresh
  work completes during the second VM. The next recovery records `exhausted` with
  `retry_budget_exhausted`, preserves the bound target, and sends no fourth request.
  The public summary remains `unresolved`, not acknowledged or terminal-confirmed.
- Every case starts one additional verification VM after recovery. The entire sealed
  intent is unchanged and no further RPC is emitted. Unexpected solver dispatches,
  job-wide cancellation, duplicate probes or extra exact cancellations fail the test.

One complete ten-case run launches 34 child VMs and forces 14 terminations. The test
checks the ready marker against the owning Port's OS PID before invoking the native
`kill` executable with arguments, not a shell command. Child execution is bounded,
and failure cleanup kills and waits for owned children before removing the exclusively
created temporary test directory. No database is copied or retained as a backup.

The initial eight-case run passed (seed 281801, 17.7 seconds); the final ten-case
selection including repeated crashes passed with warnings denied (seed 281802,
25.7 seconds). No new production defect was reproduced and no runtime behavior was
changed in this follow-up. These tests strengthen evidence for the existing durable
reservation and acknowledgement boundary; they are not a red-to-green repair.

The full Web suite also passed with warnings denied: 3,977 total tests, zero
failures and 77 existing conditional skips (seed 281803, 208.1 seconds). It includes
all ten new crash cases and their independently configured SQLite/JSON child stores;
these overlapping counts are not additive or a coverage percentage. Test-owned
child stores are removed after process exit, with no new retained runtime data.

Formatting, whitespace, HTML book/inventory, project organization and tensor checks
pass. The crash suite is 281 lines and its child helper is 218 lines, both within
the 800-line source limit. The narrow `local-durable-cancellation-process-crash`
claim leaves maturity unchanged: 13 modules, 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; the release gate is blocked.
No schema, production API, migration, backup, version bump, commit, push, rebuild
or installation was added. Existing uncommitted mainline work is preserved.

This is macOS source-level POSIX process-loss testing, not physical power loss,
disk-cache failure, all possible write-interruption offsets, installed-runtime,
remote Linux or live PostgreSQL qualification. The suite explicitly skips Windows
because it uses SIGKILL. Exact-target bounded cancellation retries remain allowed
after process restart; this is not exactly-once delivery or automatic solver replay.
An Agent acknowledgement still does not establish remote execution termination.
