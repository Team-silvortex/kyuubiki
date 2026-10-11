# Analysis export snapshot consistency

The three composite SQL read paths now return records from one database snapshot:
project-bundle export, database export and workload discovery. Concurrent edits
cannot combine old model pointers with new version rows, or retained jobs with
already-deleted results. This is source service acceptance for SQLite and
PostgreSQL, not JSON cross-store or full-backup qualification.

## Reproduced failures

Six deterministic SQLite controls failed before the production change. Each
export first read a populated fixture, then committed a writer between its SELECTs
on a second actual connection. Model/version publication changed exported version
counts and could disagree with a project's embedded model pointer. Joint job/result
deletion left the old job alongside missing results and a false `has_result` flag.
There were no timing sleeps, mock repositories or solver-accuracy claims.

## Shared read boundary

`Storage.ReadSnapshot.run/1` establishes one transaction around each composite
read. PostgreSQL sets `REPEATABLE READ, READ ONLY` before the first data query,
using the currently selected dynamic repository. SQLite explicitly uses a
deferred transaction; the first SELECT establishes its WAL snapshot. The tests
commit a separate writer before the export proceeds, proving reads do not require
an early SQLite writer lock to preserve the old generation.

Response maps, bundle schema and attachment publication remain unchanged. The
storage failure boundary runs outside the transaction so a known SQL error rolls
back first and returns `{:error, :analysis_store_unavailable}` without partial data.
Unexpected programming errors remain visible. No persistent snapshot copy,
automatic retry or new data directory is introduced.

Call these composite helpers outside an existing SQL transaction. An ambient
transaction could already have queried at weaker isolation, so the boundary
refuses it with the unavailable error rather than quietly downgrading consistency.
Even an ambient transaction that happens to have stronger isolation is refused;
its isolation is not adopted or inferred. Subsequent ordinary reads/writes keep
their normal connection state. The later
[JSON consistency follow-up](analysis-memory-snapshot-20261011.md) covers the
separate generation-validation boundary for lightweight memory stores.
The later [history follow-up](history-read-reliability-20261011.md) permits nested
reads only inside this boundary's owned snapshot on the same repository. It also
explicitly rolls back contained nested query failures with their original error.

## Executed controls

Eleven composite-read cases cover the three exports during model/version
publication, joint job/result deletion and project deletion. The in-flight export
must equal its original projection, except regenerated top-level timestamps; the
next export must see the completed change. Database-export security events share
the same snapshot. All three public helpers also refuse ambient transactions.

Six read-boundary cases cover exact successful/domain-error reply shapes,
unexpected exceptions, actual query failures after an earlier successful read,
rollback/connection release, ambient-transaction refusal and dynamic-repository
selection. PostgreSQL directly checks its transaction settings and verifies that
an accidental write fails without publishing a project or leaving a read-only
connection behind. The write-refusal case is PostgreSQL-specific; SQLite callers
are the existing read-only export functions, not a general enforced write sandbox.

## Verification

- SQLite recovery/routing/API/library selection: 925 listed, 899 passed, 26
  backend-specific cases skipped, zero failures in 36.3 seconds.
- Isolated JSON selection: 628 listed, 594 passed, 34 backend-specific cases
  skipped, zero failures in 8.6 seconds. SQL concurrency/isolation cases are
  intentionally skipped; this is regression coverage, not JSON snapshot proof.
- Remote PostgreSQL selection: 118 listed, 101 passed, 17 backend-specific cases
  skipped, zero failures in 91.6 seconds. PostgreSQL 16.14 ran on the Ubuntu lab
  host in a disposable Docker container. The macOS source control plane used a
  private loopback tunnel; this is not a Linux control-plane execution claim.
- Different-order follow-up with seed 2718: all 17 new PostgreSQL cases passed
  in 18.6 seconds; SQLite's focused selection listed 27, passed 26 and skipped
  its PostgreSQL-only write-refusal case, with zero failures in 0.3 seconds.
- Strict Elixir test compilation, formatting, diff whitespace, runtime API
  surface, documentation inventory/book, topology, matrix, extension standard
  and version checks pass. Organization audit retains zero debt at 800 source
  and 2000 document lines.
- The tensor and its self-test pass with zero structural, four maturity,
  twenty evidence-grade and fifteen P0 gaps. Overall Daji qualification remains
  blocked; this evidence does not broadcast SQL acceptance to other backends.

These selections also cover the preceding collection/read outage controls,
authorization, result administration, project/version reconstruction, checkpoint
transactions, joint job/result writes, typed PostgreSQL recovery and migrations.

Run the new focused cases from `apps/web`:

```text
mix test test/kyuubiki_web/storage/export_read_snapshot_test.exs test/kyuubiki_web/storage/read_snapshot_test.exs test/kyuubiki_web/api/analysis_collection_outage_test.exs --seed 0 --warnings-as-errors
```

SQLite uses the test runner's owned temporary database. JSON requires
`KYUUBIKI_STORAGE_BACKEND=json` and a fresh temporary `KYUUBIKI_DATA_DIR`.
PostgreSQL requires `KYUUBIKI_STORAGE_BACKEND=postgres` and a disposable database
passed through `DATABASE_URL`; do not use retained research or production data.
Connection values and host configuration do not belong in the repository.

The remote database binds only to loopback, uses bounded 256 MiB tmpfs storage,
512 MiB memory, two CPUs and a 128-process limit, and creates no Docker volume.
The owned container, port forward and JSON directory were removed after acceptance.
Absence was checked through the container label, local listener and directory.
Other server services and shared SSH connections remained untouched.

## Qualification boundary

The guarantee covers database records consumed by these three SQL helpers in the
same process and selected repository. Derived runtime timing fields are computed
when serialized, not frozen database columns. Export keeps its existing public
projection, including security-event filtering/limits and private-recovery removal;
it is not a complete database backup and cannot be used as one.

The SQL controls do not qualify JSON reads. The later JSON follow-up establishes
live read consistency, not a global durable snapshot or new library persistence.
These controls do not qualify standalone history. The later
[history follow-up](history-read-reliability-20261011.md) covers independent
project/model/version and security reads, not snapshots across separate requests.
External result files, Kcore artifacts,
distributed agent state, installed WebViews, power loss and large-export memory,
transaction lifetime and latency remain separate acceptance work. No installed
app, version metadata or Git history changes.
