# Project-bound native execution and export chain

Date: 2026-09-09. Development line: daji 3.2.0, baseline commit `c66afefa`.

Project execution chain: PASS within the tested scope.

## Reproduction and corrections

The preceding checkpoint/export tests seeded synthetic result records. This round joins
the library to real execution: create a model, select a saved version, change the working
copy, solve directly and through a catalog workflow, read persisted results, export the
project, reconstruct model data in another project, rerun, and remove owned records.

An initial probe used the installed 3.2.0 frontend HTTP proxy, Orchestra and Rust Agents.
The same selected model and version produced two completed jobs, but only the direct solve
appeared in its project bundle. The catalog job had a new unrelated project ID and no
version ID; the bundle contained one job/result instead of two. Probe records were removed.

| Finding | Fix and regression |
| --- | --- |
| `Analysis.submit_catalog_workflow/2` dropped `project_id` and `model_version_id` when delegating to raw graph submission. | Forward both fields through the existing whitelist; keep the catalog graph authoritative. Six new contract tests failed before this fix and passed afterward. |
| A missing/deleted catalog checkpoint was ignored, allowing execution with incorrect provenance. | The delegated context resolver now rejects it before creating a job. A corrected valid-version submission succeeds without phantom jobs. |
| Deleted or unknown jobs returned 422 from detail/status reads but 404 from other job operations. | Normalize both reads to 404 and `job_not_found`. Two new boundary tests failed before this correction; update the older CRUD test's explicit 422 expectation. |
| A broad memory run exposed a race in the operator-heartbeat test. | Its deliberate 1.7-second silence exceeded the configured 1.5-second stale limit, so a pre-existing periodic tick could kill the job before the manual heartbeat. Restart only the test application's watchdog with an isolated scan schedule and restore it on exit. Production deadlines, heartbeat throttling and watchdog behavior are unchanged. |

The initial native failure assertion was corrected, not the production recovery behavior:
a failed workflow deliberately retains diagnostics in the result store. `has_result = true`
does not certify a successful solve. The regression now requires failed recovery state,
failure history, an uncompleted solve node, no successful JSON artifact, and a successful
corrected submission using the same Agent. The bundle must retain both failed and completed
records with their respective states.

## Tests and evidence

- `apps/web/test/kyuubiki_web/api/project_workflow_context_api_test.exs`: eight always-on API
  contract cases, including project-only jobs, version-owner precedence, atom/JSON keys,
  invalid checkpoint rejection/retry, raw-graph parity, export isolation and deletion.
  Transport responses are fixtures; these cases do not certify physics.
- `apps/web/test/kyuubiki_web/api/project_execution_chain_live_test.exs`: three opt-in chain
  cases using an explicitly selected independent native Rust Agent. Each complete run makes
  14 real solver requests: 13 successful solves and one intentionally singular solve.
- `apps/web/test/support/workflow_project_execution_api.exs`: shared API requests and a
  small SI-unit rod model, kept separate from the test cases.
- Existing checkpoint transaction, library, storage, jobs, Orchestra and API suites were
  included in the wider regression selection. No error tolerance or production timeout was
  relaxed to make the tests pass.

The numerical oracle is a uniform 2 m axial rod, cross-section 0.002 m2 and modulus 200 GPa.
Both direct and catalog paths must match `u = F L / (E A)` and `sigma = F / A` for loads
0, 1500, 6000 and 40000 N. A separate 2000 N checkpoint must still yield displacement
`1e-5 m` and stress `1e6 Pa` after the working copy changes to 6000 N. The relative tolerance
is `1e-10`, with absolute floors of `1e-14 m` and `1e-8 Pa` for zero/near-zero values.
This is an independent closed-form check of a small model, not a broad FEM qualification.

| Execution | Result |
| --- | --- |
| macOS ARM64, memory, focused new cases, seed 9127 | 11 passed |
| macOS ARM64, SQLite, broader selection, seeds 9127 and 0 | Each run: 347 selected, 342 passed, 5 skipped |
| macOS ARM64, memory, broader selection after test-timer isolation, seed 9127 | 347 selected: 339 passed, 8 skipped |
| Remote physical Ubuntu x86_64, Docker Elixir 1.19 + native Rust Agent, SQLite | 20 passed, no skips |
| Remote physical Ubuntu x86_64, Docker Elixir 1.19 + native Rust Agent, PostgreSQL 16 | 20 passed, no skips |

The broader selection comprises all `api`, `library`, `storage`, `jobs` and `orchestra`
tests plus `workflow_diagnostics_bundle_runtime_test.exs`. Five cases require a separately
configured PostgreSQL test endpoint; memory additionally skips three SQL-only cases.
The remote 20 are the 11 new cases plus seven existing checkpoint-chain and two SQL
transaction-fault tests, not those five separate PostgreSQL suites.

Remote execution used nine current runtime modules compiled in memory over an existing
read-only Linux build/dependency cache. All 16 transferred source, helper, test and runner
digests matched. The Agent used the existing Linux binary from the preceding postprocess
research package. This is a focused current-module check, not a full Linux rebuild.
The containers had no external network or published ports, shared only their own loopback
namespace, used read-only source mounts and bounded tmpfs, and did not access existing
service databases. No image download or source/build backup was needed.

To repeat from a full checkout, first start a dedicated native Agent on an unused loopback
port. Do not reuse an Agent assigned to an installed or shared Orchestra. Then, from
`apps/web`, run with fresh isolated paths:

```text
KYUUBIKI_PROJECT_CHAIN_AGENT_PORT=<dedicated-port> \
KYUUBIKI_AGENT_ENDPOINTS=project-chain-agent@127.0.0.1:<dedicated-port> \
KYUUBIKI_STORAGE_BACKEND=sqlite KYUUBIKI_DATA_DIR=<isolated-json-dir> \
SQLITE_DATABASE_PATH=<isolated-sqlite-file> MIX_ENV=test \
  mix test test/kyuubiki_web/api/project_workflow_context_api_test.exs \
  test/kyuubiki_web/api/project_execution_chain_live_test.exs --seed 9127
```

The native suite skips explicitly if `KYUUBIKI_PROJECT_CHAIN_AGENT_PORT` is absent; an
invalid port or an unavailable explicitly selected Agent fails instead of using a fallback.
Use `KYUUBIKI_STORAGE_BACKEND=memory` for the memory path, or `postgres` and an isolated
`DATABASE_URL` for PostgreSQL. The API test case resets its test application's stores.

Repository checks passed: changed Elixir formatting, compilation with warnings as errors,
the 26-page HTML book and documentation inventory, the 800-line source / 2000-line document
organization audit, tensor self-test/structure/command, and `git diff --check`. The tensor
continues to report the Daji release gate as blocked; this scoped evidence does not replace
the remaining release qualifications.

## Limits and next gaps

- The fixed-source regressions exercise the Plug router, actual storage, scheduling and
  native Agent RPC. Only the initial defect probe used the installed frontend HTTP proxy;
  this round does not qualify the installed GUI, touch interaction or PWDT action chain.
- GUI and Rust headless SDK high-level workflow submit wrappers do not yet expose the
  project/version context used by these raw API tests. Their project-bound submission
  coverage remains a follow-up, not an implied success here.
- Checkpoint IDs are lineage metadata, not automatic payload resolution or a cryptographic
  input-equivalence check. These tests explicitly fetch and submit the selected payload.
- Reconstruction is client-driven model CRUD with fresh IDs. No new archive import endpoint
  or automatic historical job/result re-identification is provided. Old wrongly associated
  jobs are not guessed or silently rewritten.
- Export still uses multiple reads; concurrent-writer snapshot consistency, power loss,
  deletion during execution, cross-version upgrades, Windows and large meshes are outside
  this round. Stop writers when exporting a stable transfer snapshot.
- Scoped evidence is added to the existing persistence/provenance tensor file. The global
  release gate is not promoted. No version bump, commit/push, app rebuild or installation
  was performed; installed applications still need a later rebuilt runtime to include fixes.

Owned temporary databases, test containers and the small remote source overlay are removed
after verification. Keep these modular tests and this report, not transient state copies.
