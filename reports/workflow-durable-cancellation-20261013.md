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
