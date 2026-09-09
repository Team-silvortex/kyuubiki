# Project, checkpoint and export chain regression

Date: 2026-09-09

Project/version chain: PASS within the tested scope.

## Scope and reproduced defects

This round moved beyond result-file persistence to the project-library chain:
create a project/model, save checkpoints, edit the working copy, delete a checkpoint,
export, and reconstruct the model/checkpoints into a separate project through existing APIs.
The tests call the real Plug router and storage backends, not mocked frontend effects.

The initial seven regression cases produced three failures on memory and five on SQLite:

| Defect | Reproduction | Correction |
| --- | --- | --- |
| SQL model name overwritten | Creating `Copper conductor` returned `Initial version` as the model name. | Keep the initial checkpoint label separate from the model name. |
| Dangling latest checkpoint | Deleting the newest or only version left `latest_version_id` pointing at the deleted row. | Recompute the latest ID/number; preserve the working payload. |
| Cross-model active version | Exporting two models after removing the active model's checkpoint selected the other model's version. | Resolve the active version within the active model only, or return null. |
| Concurrent duplicate numbers | Twelve successful SQLite checkpoint requests produced `[2,2,2,2,2,2,3,3,3,3,3,3]`. | Allocate and publish under one transaction and parent-model lock. |

Memory serializes its mutations through an Agent. SQLite uses an immediate transaction;
PostgreSQL uses `FOR UPDATE` on the parent model. Creation and deletion take the same lock
order. Creating a model and its initial version is also atomic. Latest-pointer repair queries
only the newest version's ID/number instead of loading all historical model payloads.

`version_id` remains the identity. `version_number` orders the remaining checkpoints;
deleting the tip can allow a number to be reused with a fresh ID. No migration, relabeling of
existing duplicate versions, automatic backup or rewriting of scientific payloads was added.

## Repeatable tests

Source suites:

- `apps/web/test/kyuubiki_web/api/project_version_chain_api_test.exs`: seven chain cases,
  including deletion/retry, independent working-copy edits, non-latest deletion, all versions
  removed, twelve parallel requests, project/result isolation and API model reconstruction.
- `apps/web/test/kyuubiki_web/library/checkpoint_transaction_test.exs`: two SQL fault-injection
  cases. Database triggers deliberately reject the latest-pointer update after a model/version
  mutation. Initial creation, checkpoint creation and checkpoint deletion must roll back;
  removing the injected fault permits a clean retry. These cases skip the non-SQL backend.

| Execution | Result |
| --- | --- |
| macOS ARM64, focused memory, seed 0 | 7 passed |
| macOS ARM64, focused SQLite, seed 9127 | 9 passed |
| Remote physical Ubuntu, Docker `elixir:1.19`, SQLite, seed 9127 | 9 passed |
| Remote physical Ubuntu, Docker `elixir:1.19`, PostgreSQL 16, seed 9127 | 9 passed |
| macOS ARM64, broader memory, seed 9127 | 336 selected: 328 passed, 8 skipped, 0 failed |
| macOS ARM64, broader SQLite, seeds 9127 and 0 | Each run: 336 selected, 331 passed, 5 skipped, 0 failed |

The broader selection includes all API, library, storage, job and Orchestra tests, plus
`workflow_diagnostics_bundle_runtime_test.exs`. Five skipped tests need a separate PostgreSQL
test endpoint; memory additionally skips the SQL foreign-key case and two SQL-trigger cases.
The remote nine-case PostgreSQL run is not a claim that those five separate suites ran.

The first broad memory run and its isolated rerun also exposed a test-budget failure in
the 14-node electrostatic/heat/thermo diagnostics chain: the persisted job had reached node
9 and was still progressing when the test exhausted twenty 10 ms polls. The shared test
helper now allows 200 polls and reports the final lightweight job status on exhaustion.
The isolated case and the broad memory selection then passed. This changes only test waiting,
not production watchdogs, solver timeouts, completion assertions or physical tolerances.

Local reproduction, using fresh isolated paths and the configured Elixir toolchain:

```text
cd apps/web
KYUUBIKI_STORAGE_BACKEND=memory KYUUBIKI_DATA_DIR=<isolated-memory-dir> MIX_ENV=test \
  mix test test/kyuubiki_web/api test/kyuubiki_web/library \
  test/kyuubiki_web/storage test/kyuubiki_web/jobs test/kyuubiki_web/orchestra \
  test/kyuubiki_web/workflow_diagnostics_bundle_runtime_test.exs --seed 9127

KYUUBIKI_STORAGE_BACKEND=sqlite KYUUBIKI_DATA_DIR=<isolated-json-dir> \
  SQLITE_DATABASE_PATH=<isolated-sqlite-file> MIX_ENV=test \
  mix test test/kyuubiki_web/api/project_version_chain_api_test.exs \
  test/kyuubiki_web/library/checkpoint_transaction_test.exs --seed 9127
```

## Remote scope and disk budget

Remote checks compiled eight current runtime sources in memory over an existing read-only
Linux build/dependency tree, then loaded the two new test files. All ten transferred source
and test SHA-256 digests matched the local files. This is a focused current-module check,
not a full-source Linux rebuild or installed desktop qualification.

The Elixir containers used two CPUs, 768 MiB memory, read-only source mounts and bounded
tmpfs. SQLite ran without networking. PostgreSQL used a separate disposable container with
no published port and no external network; the test process shared only its network namespace.
The database listened on loopback. Existing service databases and unrelated containers were
not used. No image download or complete source/build backup was needed.

After verification, the owned temporary databases, source overlay and disposable containers
are removed. The local temporary run directory was 7.3 MiB including its full tensor report.
Keep the source regressions and this report, not generated stores or dump copies.

Repository checks passed: changed Elixir formatting, compilation with warnings as errors,
the 26-page HTML book check, documentation inventory, 800-line source / 2000-line document
organization audit, tensor structure/command/self-test and `git diff --check`. The new tensor
claim is scoped to verified checkpoint/export behavior, not global release qualification;
the existing Daji release gate remains blocked.

## Limits

- Model reconstruction in this test is a client sequence over the existing CRUD APIs, not a
  new server-side import API, an installed GUI import test or a native archive round trip.
- The result-isolation fixture is synthetic and seeded through stores. No new solver accuracy,
  mesh-scale benchmark or physical-research validation is claimed.
- Export is still composed of multiple reads; consistent export during concurrent writers is
  not qualified. Stop writers when a stable transfer snapshot is required.
- Transaction exceptions still propagate to the caller. Rollback and retry safety are tested;
  standardized HTTP reporting for every database I/O failure remains separate work.
- Historical jobs/results, deleted-version provenance, malformed metadata inputs, already
  inconsistent stores, power loss and every cross-process deletion race are outside this round.
- Memory library state remains an in-process library, not a newly durable project database.
  No version bump, Git commit/push, app rebuild or installation was performed.
