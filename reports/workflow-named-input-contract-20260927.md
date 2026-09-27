# Workflow Named Input Contract

Date: 2026-09-27. Source: daji 3.4.0 (`b0148c62`) plus the current
uncommitted reliability changes. No version bump, commit, push, app installation,
production database reset, or remote deployment is part of this work.

## Reproduced Defects

The graph executors selected multi-input assembly from incomplete operator lists.
Rust omitted all nine physical benchmark-pair transforms. Elixir omitted eight
of them and the parameter-sweep result join. The underlying operators were
callable directly, but their named graph inputs were not assembled correctly.

The shared fixture reproduced both a rejected valid graph and a silent wrong
winner: a first source containing nested `left`/`right` fields was treated as the
whole pair, ignoring the independently connected second source. With costs 10
and 4, minimization must select the right source, but the nested costs 1 and 99
incorrectly selected the left source in both runtimes.

After correcting the test harness to use the public Rust operator registry and
explicit electrostatic criterion fields, the pre-fix check had 12 failing
Elixir tests and three failing Rust test functions. These are regression
assertions, not a count of distinct production defects.

## Routing Contract

- The fixture lists nine benchmark pairs: structural, acoustic, modal, dynamic,
  transport, coupled heat, electrostatic, magnetostatic, and CFD.
- Parameter-sweep join binds `cases` and `results` separately; the fixture sends
  result IDs in reverse case order and verifies their identity-based association.
- Named assembly follows each edge's target port, not insertion order or nested
  fields in the first artifact. Both node and edge orders are reversed in tests.
- A single declared `input` or `payload` port can carry a complete operator
  payload. This exception does not apply to a multi-port node missing an edge.
- One-field legacy source-port envelopes are unwrapped exactly once. A complete
  artifact with sibling data or status is not implicitly reduced to one field.
  Full regression initially caught the existing wrapped thermal-summary API
  case; that original test remains unchanged and now passes.
- A shared source may feed both named ports while remaining available to an
  independent output. Rust tests exercise both full and outputs-only retention.
- Invalid second inputs fail fast, or isolate only under explicit skip policy;
  they emit neither a winner nor result lineage, and healthy replay still works.

The graph/direct comparisons are within each runtime. Shared fixed assertions
check expected winners, scores, join counts and missing IDs; they do not assert
byte-for-byte equivalence of all result fields across the two implementations.
Existing routing for other transforms and custom callbacks remains unchanged.

## Deleted-Job Progress Race

Full-suite log inspection also exposed an `Ecto.StaleEntryError` in the SQL job
backend: another operation deleted a record after its last read but before the
progress update. This exception restarted the watchdog despite passing ordinary
assertions. Two deterministic tests use synchronous SQL query telemetry to
delete the record precisely between read and update, without sleeps or
production-only test hooks. Both failed before the fix.

Progress updates now use Ecto's `stale_error_field: :job_id`, returning a failed
changeset rather than throwing. The watchdog counts only successful writes,
retains its PID across the race, and still marks the surviving stale job failed.
The test also confirms a new healthy job can complete after a rejected write.
No `allow_stale` success override or broad exception swallowing was introduced.

This does not establish compare-and-swap safety for all concurrent progress writers.
The shared SQL implementation serves SQLite and PostgreSQL; this turn executes
the race on isolated SQLite only, not a live PostgreSQL server.

## Reproduction

Storage-independent routing and branch-recovery checks:

```sh
make test-workflow-named-input-contract
```

For API/job tests, create a fresh disposable directory outside the repository,
set `KYUUBIKI_STORAGE_BACKEND=sqlite` and point `SQLITE_DATABASE_PATH` to a new
database inside that directory. Then run from `apps/web`:

```sh
mix test test/kyuubiki_web/api/workflow_named_input_contract_api_test.exs test/kyuubiki_web/jobs/progress_race_test.exs test/kyuubiki_web/jobs/watchdog_test.exs
```

Do not use an application database: API test setup resets stores. Remove only
the disposable database and its SQLite sidecars after the test VM has exited.

## Verification Results

Executed locally on macOS with the current Rust and Elixir workspace toolchains:

| Check | Result |
| --- | --- |
| Full `kyuubiki-engine` tests | 722 passed, 0 failed, 1 existing ignored |
| Full web tests with isolated SQLite | 963 tests, 0 failures, 8 existing skipped |
| Named-input Make target | 43 Elixir tests and 6 Rust test functions passed |
| Progress race plus watchdog tests | 7 tests passed |
| Native named-input validation profile | Executed and passed |
| Engine Clippy, all targets, warnings denied | Passed |
| Test-environment Elixir compilation, warnings as errors | Passed |
| Validation profile structural audit | 59 profiles passed |
| Make module audit | 8 included modules passed |
| Organization audit | 800-line source / 2000-line document limits passed |

Focused counts overlap the full suites; they must not be added as independent
coverage. The final full web log contains no warning or error entries, including
no watchdog restart. The original wrapped-thermal-summary API file remains
unchanged. Test data stayed in a disposable SQLite database, not the installed
runtime or developer application database.

The tensor structure and command audit passed with zero structural gaps. Its
overall status remains blocked: four maturity gaps, 16 evidence-grade gaps, and
11 P0 gaps remain. The new routing and deleted-job claims add only local
`verified` contract/recovery evidence, not broad readiness promotion.

## Scope

This is bounded local input-routing evidence, not numerical or distributed qualification.
It does not change solver formulas, qualify million-node scale, add unsupported
operators to a runtime, or establish universal input-schema equivalence. The
focused profile excludes database-dependent tests; the API and race tests are
run separately with isolated storage and as part of the full web suite.
