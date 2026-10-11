# Typed workflow PostgreSQL persistence and restart recovery

Bounded source recovery: PASS. A disposable PostgreSQL 16.14 database on the
authorized Ubuntu lab host verifies the typed workflow envelope through actual
JSONB storage, transactions and process restarts. Orchestra and real Rust Agents
run from source on macOS, connecting to the remote database through a private
tunnel. This is not Linux Agent, installed-runtime or remote-compute acceptance.

## Defects corrected

- Raw lease queries used the repository module rather than its selected dynamic
  pool. Checkpoint writes already followed the selected pool. Lease checks now
  use that same pool and transaction, so ownership and protected writes cannot
  accidentally refer to different database scopes.
- Job, lightweight status, result and result-chunk reads raised known storage
  exceptions when the pool or memory store disappeared. Direct callers now get
  `{:error, :analysis_store_unavailable}`; those four GET routes return opaque
  HTTP 503 JSON. Restoration reads the original data without a write, automatic
  resubmission or retry instruction. Missing records retain their 404 semantics.
  Mutating routes keep their existing error mapping. Unrelated programming
  errors are not swallowed by the storage failure boundary.
- Ordinary test support loading accidentally ran opt-in live seed fixtures.
  Reusing a persistent test database then collided with those fixed record IDs.
  Ordinary tests now exclude the seed script; the opt-in live server explicitly
  loads it only when requested.
- Separate real PostgreSQL workflow rounds shared an Orchestra lease scope.
  Each round now creates its own owned schema, reuses it across its restart,
  and removes it afterwards. The fencing checks are not bypassed for tests.
- Live capture files were published directly, allowing readers to observe an
  empty or partial JSON file. Owned temporary captures now publish by rename.
  They capture actual authored tasks, never fabricated receipts or solve output.

## Database contract

Eleven isolated-schema tests check selected-pool lease acquire/read/renew/release
and fencing; exact execution reads; typed compare-and-swap; a fresh repository
pool restoring solve and branch checkpoints under a new claim; stale ownership;
joint terminal pruning; and four injected job/result write failures with complete
transaction rollback followed by a successful retry.

Actual PostgreSQL JSONB numeric equality folds `1` with `1.0`, and negative with
positive floating zero. The canonical execution string inside the v2 envelope
does not. Compare-and-swap rejects changed integer/float identity, signed zero,
tiny values and adjacent binary64 values without changing the saved result.
The correct expected snapshot can still commit. Checkpoint reads and failed
writes do not refresh or silently reseal the private execution payload.

Five existing PostgreSQL migration tests are also rerun: fresh schema startup,
explicit legacy migration with DDL rollback, incompatible revision/checksum/shape
rejection and competing migration ownership. This does not qualify the separate
revision-two PWDT checkpoint-retry lifecycle requirement.

## Actual workflow chains

Three explicit PostgreSQL live tests execute seven bounded rounds:

1. Five fresh thermal, electrostatic, magnetostatic, thermomechanical and Stokes
   solve/diagnose/condition/join/score chains cover both selected branch routes.
   Orchestra process loss and restart restore solve output, branch decisions and
   the original diagnostic receipt. Rust independently verifies the authored
   typed digest. Resumed scoring identity is unchanged; rejected-route outputs
   remain absent; original dispatch bytes are unchanged. Each solved round uses
   three Agent executions without repeating completed solve/diagnosis work.
2. A raw thermal-result round preserves input numeric types and signed-zero bits
   through admission, JSONB storage and restart.
3. A solved/branched thermal round also restarts the Agent, losing its memory-only
   receipt. Recovery blocks after checkpoint restoration without replay, another
   peer execution or downstream publication. Checkpoints cannot authorize an
   unknown native outcome.

Ordinary terminal transitions prune private execution and checkpoint payloads;
SDK waiting observes the completed result. No second payload archive is added.

The remote harness uses a test-only pool warm-up policy: queue target 1,000 ms,
queue interval 10,000 ms. An earlier run failed while connections were starting;
the final run uses this bounded warm-up, not relaxed production lease/query
timeouts. Default-policy cold startup under remote latency remains unqualified.
Earlier runs also exposed incomplete capture publication; the capture race and
the independent storage-read exception paths are now tested separately.

## Verification

- Final PostgreSQL selection: 23 tests passed, zero failures in 97.9 seconds.
  This includes the eleven typed database tests, five migration tests, five read
  outage cases, storage-outage health and concurrent atomic capture publication.
- Final explicit PostgreSQL live selection: three tests passed, zero failures in
  210.72 seconds, covering seven small workflow rounds, not seven large studies.
- Final SQLite real-process controls: three tests passed in 116.27 seconds,
  covering the corresponding seven typed workflow rounds. The three opt-in
  PostgreSQL tests were ignored in this control invocation, not counted twice.
- Expanded SQLite recovery/routing/read/API selection: 831 listed, 811 passed,
  20 storage-specific cases skipped, zero failures in 37.8 seconds.
- Isolated JSON recovery/routing/read/API selection: 534 listed, 523 passed,
  11 PostgreSQL-specific cases skipped, zero failures in 7.8 seconds.
- A second invocation on the same PostgreSQL database passed 29 read-outage,
  result-administration, job-audit, authorization, health and capture tests in
  24.1 seconds. No duplicate opt-in live fixture was seeded.
- CLI all-target Clippy passed offline with warnings denied.
- Elixir strict test compilation, formatting and runtime API/TaskIR contract
  checks passed. Documentation inventory/book, topology, function matrix,
  extension standard and version audits passed. Organization debt is zero at
  800 source / 2000 document lines. The tensor and its self-test pass with zero
  structural, four maturity, twenty evidence-grade and fifteen P0 gaps; overall
  Daji qualification remains blocked.

## Reproduction and ownership

Provision a disposable database; never point these tests at an active installed
runtime store. Connection values belong in the invoking environment, not this
repository. From `apps/web`, provide `DATABASE_URL`,
`KYUUBIKI_STORAGE_BACKEND=postgres` and `KYUUBIKI_TEST_DATABASE_URL`, then run:

```text
mix test test/kyuubiki_web/orchestra/workflow_typed_postgres_test.exs test/kyuubiki_web/storage/database_postgres_test.exs test/kyuubiki_web/api/analysis_read_outage_test.exs test/kyuubiki_web/api/storage_outage_health_test.exs test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs --seed 0 --warnings-as-errors
```

From `workers/rust`, provide `KYUUBIKI_HEADLESS_LIVE_TEST_POSTGRES_URL` and run:

```text
cargo test --offline -p kyuubiki-cli --test headless_live real_typed_postgres -- --ignored --nocapture --test-threads=1
```

These live tests are ignored without explicit invocation; an unexecuted ignored
case is not counted as passed. Each owned schema is removed after its round.
The operator must retire the disposable database and tunnel after verification.
No connection configuration, credentials or database dump is added to Git.
This run removed its disposable memory-backed container, cancelled only its
database port forward, and deleted its isolated JSON directory. No Docker
volume was created; unrelated containers and shared SSH sessions were untouched.

## Qualification boundary

This adds verified source macOS / remote PostgreSQL contract and recovery
evidence. Default digest adoption, installed mixed-version peers, Linux Agents,
remote mesh, arbitrary executable package hosts, distributed SDK packages,
default-policy cold startup and revision-two PWDT lifecycle remain separate
acceptance work. Agent receipts remain bounded and memory-only; there is no
exactly-once, power-loss, large-model or independent physical-accuracy guarantee.
Version metadata, installed applications and Git history are unchanged.
