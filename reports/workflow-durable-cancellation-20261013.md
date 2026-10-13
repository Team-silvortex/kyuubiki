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
