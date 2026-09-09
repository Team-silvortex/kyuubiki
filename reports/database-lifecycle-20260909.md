# Runtime database lifecycle verification

Date: 2026-09-09. Source base: `b35d6863` (`daji 3.1.8`) plus this working tree.
No product version, Git history, installed application or production database was changed.

## Result

Runtime database baseline, copy upgrade and independent verification: PASS.

This increment adds a real startup gate and version ledger to the existing Elixir/Ecto
storage owner, not another database implementation in Rust or Node. The ledger is data
version 1, independent of application release numbering. SQLite and PostgreSQL share
the baseline definition and refusal policy. SQLite additionally has a native-library
and Mix maintenance path for consistent snapshots, copy upgrades and separate restores.

## Verified behavior

- Empty startup installs all 13 managed runtime/central-store tables and one ledger row
  transactionally. Repeated startup does not duplicate or rewrite history.
- Legacy startup refuses before schema/data mutation. Maintenance adopts a candidate,
  preserves rows and scientific JSON/JSONB, and adds the four known absent job columns.
- Pinned backend-specific recipe checksums prevent accidentally changing a shipped
  baseline. Unknown versions, history gaps, altered checksums, reader floors, ledger
  shapes, missing current tables, incompatible columns and missing foreign keys fail closed.
- SQLite immediate transactions and PostgreSQL transaction-scoped advisory locks
  serialize cooperating migrators. A late DDL failure after ALTER rolls everything back.
- SQLite VACUUM INTO includes committed, uncheckpointed WAL rows. Plan/copy operations
  reject active WAL/journal input and require an approved standalone-file digest.
- Restore writes a distinct target with exactly the approved snapshot bytes. Upgrade
  publishes a verified current candidate without deleting or overwriting its source.
- Changed approval, malformed databases, broken references, symlinks, occupied targets,
  dangling destinations and existing output sidecars fail before publication.
- Receipts verify whole-file digest, byte count and the live data plan after relocation.
  Tampering and incomplete receipts are rejected. Unix outputs use mode 0600.
- Failed pre-publication operations clean owned staging. Post-publication cleanup failure
  returns the successful receipt with `cleanup_pending`, rather than losing the receipt.
- CLI tests cover backup, plan, upgrade, verify and restore, including missing approval,
  irrelevant flags and duplicate options. OptionParser originally discarded duplicate
  options before validation; the adapter now explicitly retains and rejects them.
- Dynamic Ecto Repo routing is tested: raw SQL resolves the selected dynamic Repo rather
  than accidentally connecting to the default named repository.

## Test evidence

macOS ARM64, Elixir 1.19.5 / OTP 28: storage group contains 41 tests, 36 passed and
5 PostgreSQL-only tests explicitly skipped. There are no failures.

Ubuntu x86_64, isolated Elixir 1.19 Docker image, Elixir 1.19.5 / OTP 28, and a disposable
PostgreSQL 16 container: 317 tests passed, 0 failures, 0 skipped. This includes the full
storage group (all 41 tests), jobs, Orchestra and API tests. The application itself uses
a newly created SQLite database, so startup and the existing restart/recovery test pass
through the new gate. The PostgreSQL cases use isolated schemas and real connections.

Development command groups:

```text
KYUUBIKI_STORAGE_BACKEND=memory MIX_ENV=test mix test test/kyuubiki_web/storage
KYUUBIKI_STORAGE_BACKEND=sqlite MIX_ENV=test mix test \
  test/kyuubiki_web/storage test/kyuubiki_web/jobs \
  test/kyuubiki_web/orchestra test/kyuubiki_web/api
```

Use a NEW `SQLITE_DATABASE_PATH` for the second group; PostgreSQL cases require an
isolated `KYUUBIKI_TEST_DATABASE_URL`. Never run these mutation tests against user data.

The remote Mix cache initially contained older-OTP BEAM files. A private test-only Mix
home with OTP-28-compatible Hex/rebar was used; the server's global toolchain was not
modified. Existing Docker images were reused and the SQLite NIF compiled from cached
source. No production container or database was reused for mutation tests.

An initial expanded run lacked schema fixtures; those unchanged repository fixtures
were copied and the complete group rerun. Another initial run incorrectly used memory
storage for the SQL-first suite: missing-job FK and duplicate memory-backend assumptions
failed, plus one shared-run JSON persistence digest error. That failed log is retained.
The affected workflow API file passes separately in a fresh memory profile (2 tests,
0 failures). This does NOT qualify the full mixed memory profile or establish the digest
error's root cause; follow-up should isolate backend/lifecycle fixture interference.
The final 317-test result is specifically the normal SQLite profile plus explicit PG cases.

Follow-up: [result key round-trip verification](persistence-key-roundtrip-20260909.md)
identified and fixed the digest mismatch as inconsistent native nil-key serialization.
Its corrected 327-test memory and SQLite selections pass; the SQL-only assertion and
duplicate memory-process test setup are handled explicitly rather than hiding failures.

## Real command artifacts

The remote native maintenance command chain produced and verified:

- `source.sqlite3`: synthetic legacy thermal study, job and result records.
- `snapshot.sqlite3` and `backup-receipt.json`: database-aware standalone snapshot.
- `plan.json`: reviewed snapshot digest and pending columns.
- `relocated.sqlite3`, `upgrade-receipt.json`, `verify.json`, `verify-relocated.json`:
  versioned candidate verified before and after relocation.
- `restored.sqlite3` and `restore-receipt.json`: byte-identical retained snapshot copy.
- `verify-backup.json`: independent backup verification.
- `refuse-overwrite.stderr`: expected nonzero exit when targeting an existing database.

Snapshot and restored-file SHA-256:
`2e2bb2dcbf28d062849b98599afc36fad741b7c1a8f34cbbe23208a7c4567cb4`.

Relocated candidate SHA-256, unchanged after the refused overwrite:
`410c1c2ac24d805c5ed5c3a8aa5010d4c196b2a2f4fe8b3b58959f2455ad97ff`.

Receipts report `directory_synced=true`; no ordinary cleanup residue was reported.
The lab's isolated `research-runs/database-lifecycle-20260909/evidence` directory retains
the small logs, plans, digests and receipts, not binary database fixtures. A development
retention cleanup on 2026-09-09 removed the completed run's duplicate source/build tree,
synthetic SQLite files/sidecars and successful isolated memory-profile stores. The small
`memory-profile-results.json.corrupt` failure artifact was retained for diagnosis, then
removed after the follow-up reduced it to source regressions; the failed-run log and
compatible Mix tool cache were kept. The hashes above describe the
verified historical run, not database files still retained. No cleanup backup was created.
The shared JSON contracts
are `schemas/database-lifecycle.schema.json` and `schemas/central-database-policy.schema.json`.
All seven lifecycle JSON outputs and the actual central-database policy output passed
their JSON Schema checks using the server's preinstalled validator.

Repository checks passed: Elixir format and warnings-as-errors compilation, 26-page
HTML book validation, documentation inventory, source/document size organization audit
(800/2000 limits, no tracked debt), tensor structure/command validation and tensor
self-test. `git diff --check` is clean. Global tensor `daji status=blocked` remains unchanged.

## Remaining boundaries

- PostgreSQL backup/restore orchestration is not implemented. Candidate adoption requires
  an operator-owned, properly restored isolated database, not the active Repo.
- No online delta synchronization, automatic configuration switch, live-writer fencing,
  automatic database downgrade or merging of post-snapshot writes is provided.
- External research artifacts, Kcore, recovery envelopes, secrets, drafts and package
  caches are not included. This is not a whole-system or encrypted backup.
- Gate validation covers known column/key contracts, not every SQL index, trigger,
  default expression, database privilege or scientific payload invariant.
- Hard process kills, ENOSPC, power loss, network filesystems, Windows ACLs and large
  production datasets remain unqualified. The 16 GiB limit is a safety budget, not a
  throughput, temporary-space reservation or million-node qualification.
- The mixed-profile regression noted above is resolved by the follow-up; this does not
  qualify concurrent file writers or installed whole-system JSON recovery.
- Existing unversioned databases now require explicit maintenance before runtime startup.
  Installed GUI/Installer integration and rollout messaging are the next operational work.

The scoped evidence is added to the coverage tensor. It must not promote whole-system
readiness or replace the project's remaining release gates.
