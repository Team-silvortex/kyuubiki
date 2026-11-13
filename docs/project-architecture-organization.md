# Project Architecture Organization

This document is the working organization map for the active `daji 3.x`
architecture line. It exists to keep feature work, protocol work, and cleanup
moving in the same direction after the `tamamono 1.x` bridge became historical
preparation context.

For a whole-system module map before diving into source ownership details, read
[module-architecture.md](module-architecture.md) first.

## Architecture Posture

Kyuubiki is now organized around a strict split between product shells,
control-plane services, runtime engines, and shared contracts.

- `Hub`
  is the operator entry shell. It launches, selects, observes, and routes users
  into runtime workloads.
- `Workbench`
  owns concrete project and workflow authoring. Its UI is built-in and stable
  so browser automation and WASM Python can rely on fixed surfaces.
- `Installer`
  owns deployment, repair, update, remote bootstrap, and machine layout policy.
- `Orchestra`
  is the Elixir control-plane workload. It schedules, verifies, persists, and
  exposes APIs, but should not become a monolithic platform by itself.
- `Agent`
  is the Rust runtime/data-plane worker. It executes solver and operator tasks
  through protocol payloads, not UI knowledge.
- `Headless SDKs`
  are Rust/Python/Elixir control clients. They must be able to use core
  capabilities without going through the frontend.
- `Worker / Operator SDK`
  is the Rust-only executable operator extension surface, not a Headless client.
  Frontend PWDT is a third, separate UI automation surface.
- `Schemas`
  are the shared contract layer used by GUI, control plane, SDKs, installer,
  and agents.

## Current Source-Of-Truth Files

- Compact current map, without experiment history:
  `docs/current-architecture-map.md`
- Runtime and product shell boundaries:
  `docs/app-runtime-boundaries.md`
- System module architecture:
  `docs/module-architecture.md`
- Agent and orchestrator boundary:
  `docs/agent-orchestrator-boundary.md`
- Headless agent contract:
  `docs/headless-agent-contract.md`
- Operator TaskIR digest rules:
  `docs/operator-task-ir-digest.md`
- Minimum industrial closure bridge:
  `docs/minimal-industrial-closure.md`
- Shared schema index:
  `schemas/README.md`
- Repository shape:
  `docs/repository-structure.md`
- Documentation ownership:
  `docs/runtime-doc-ownership.md`

## Operator TaskIR Stack

The current TaskIR path is:

1. Operator descriptors are authored by Elixir catalog code, Rust SDKs, or
   future external SDKs.
2. Descriptors lower into language-neutral `operator-task-ir`.
3. TaskIR carries a language-neutral `execution_program`.
4. Digest verification uses canonical JSON and SHA-256.
5. Control-plane APIs expose `prepare` and `execute` endpoints.
6. Rust protocol code exposes the same digest and execution-summary semantics.
7. Headless SDKs can prepare or execute TaskIR without frontend coupling.
8. Rust agents can now receive `run_operator_task_ir` over the solver RPC
   transport and return an agent-native execution summary.
9. Agent-side TaskIR verification lives behind a Rust runtime module, so RPC
   handlers stay transport-shaped instead of becoming execution engines.
10. Agent preflight returns an explicit execution plan with digest verification,
   execution-program summary, package fetch, integrity, dispatch, and result
   serialization stages.
11. Agent RPC params carry an optional `mode` field: `preflight` is the
   default; `execute` dispatches admitted native built-ins, supported Engine
   solver requests, or eligible external operator packages.
12. Agent preflight exposes an `operator_package_runtime` contract that points
   at `kyuubiki-engine.operator-sdk-host/v1` and its default trust boundary.
13. External package execution requires an attached runtime, package resolution,
   integrity and ABI checks. A detached runtime or successful preflight cannot
   be reported as completed execution. Native built-ins do not require an
   external package attachment; solver TaskIR has its own capability allowlist.

The implementation anchors are:

- `apps/web/lib/kyuubiki_web/orchestra/operator_task_ir.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_execution_program.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_execution_summary.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_envelope.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_executor.ex`
- `workers/rust/crates/protocol/src/operator_task_ir.rs`
- `workers/rust/crates/protocol/src/types/rpc.rs`
- `workers/rust/crates/cli/src/operator_task_runtime.rs`
- `workers/rust/crates/cli/src/operator_task_runtime/engine_solver.rs`
- `workers/rust/crates/cli/src/operator_task_runtime/external_execution.rs`
- `workers/rust/crates/cli/src/rpc.rs`
- `workers/rust/crates/headless-sdk/src/operator_task.rs`
- `schemas/operator-task-ir.schema.json`
- `schemas/operator-execution-program.schema.json`

## Directory Rules

- `apps/frontend`
  should keep Workbench UI and browser-side automation, but not runtime-only
  solver behavior.
- `apps/web`
  should keep orchestration, persistence, HTTP APIs, workflow catalog lowering,
  and control-plane validation.
- `apps/hub-gui`
  should keep desktop entry, runtime posture, docs shelf, and operator shell
  behavior.
- `apps/workbench-gui`
  should remain the native wrapper around Workbench, not a second Workbench
  implementation.
- `apps/installer-gui`
  should own machine setup, remote deployment, update, cleanup, integrity, and
  certificate flows.
- `workers/rust/crates/protocol`
  should hold language-neutral messages, digests, summaries, and RPC payloads.
- `workers/rust/crates/engine`
  should hold reusable operator/workflow execution helpers.
- `workers/rust/crates/solver`
  should hold numerical kernels.
- `workers/rust/crates/cli`
  should hold agent process and command-line surfaces.
- `workers/rust/crates/headless-sdk`
  should remain a Rust client SDK, not the engine implementation.
- `workers/rust/crates/desktop-runtime`
  should own shared native service lifecycle and process ownership, not the
  Agent's solver or the desktop UI. Share this library across the independent
  shells instead of growing three lifecycle implementations.
- `workers/rust/crates/script-runner`
  should own native repository maintenance and verification entrypoints, not
  numerical policy or a second implementation of Installer deployment logic.
- `schemas`
  should hold JSON contracts and examples, not app-specific implementation
  prose.
- `make`
  should hold modular Make target groups. The root `Makefile` should stay a
  small include-based entrypoint instead of growing target logic directly.
- `docs`
  should hold source-of-truth architecture documents. Hub docs may mirror them
  for operator reading, but should not silently become the deeper source.

## Numerical Source And Test Placement

Keep one shared coordinate-constraint arithmetic policy under
`workers/rust/crates/solver/src/linear_algebra.rs` and its private modules.
Directional projection, physical reaction recovery and increment-versus-state
semantics remain with their operator family. Do not move numerical admission
into Engine, Agent, GUI or SDK wrappers just to reach another control mode.
The [compact map](current-architecture-map.md#numerical-admission-ownership)
lists the currently connected equation families without duplicating their dated
test results here.

For a new constrained operator or a new solve path:

1. Identify the original equation, support identities, unknown representation
   and publication boundary. Do not substitute a reduced residual for the
   restored system, or a Newton/Newmark increment for the physical state.
2. Reuse the shared numerical policy where applicable; keep a genuinely different
   projected, complex or eigen contract explicit. Registering an operator does
   not automatically install a numerical gate or prove its physics.
3. Put local wrong-map, hidden-value, range and cancellation controls beside the
   kernel. Include all-fixed and zero-correction early exits, not only the main
   linear solve. Keep family fixture files focused rather than adding a global
   fixture registry or a wrapper module per helper.
4. Put serialized dispatch, workflow, failure propagation and replay checks under
   `workers/rust/crates/engine/tests`. Preserve the distinction between
   cooperative cancellation and durable recovery.
5. Extend the existing validation-profile and tensor-evidence shard for that
   responsibility. Dated reports retain measured scope, exclusions and costs;
   the overview stays a navigation map, not an accumulating experiment log.

## Workflow Recovery Source And Test Placement

The [recovery ownership map](current-architecture-map.md#workflow-recovery-ownership)
names existing Orchestra components. Do not add another service, generic retry
framework or per-feature facade when the current boundary already owns the work.

1. Keep coordinator calls/messages and state routing in the coordinator. Move
   new policy into its existing scan, ownership, dispatch, completion or failure
   owner rather than extending the coordinator toward the line limit.
2. Keep envelope/checkpoint validators focused on contract checks. Store helpers
   compose those checks with the owning lease and exact job/result snapshot;
   they must not publish progress and runtime evidence as independent writes.
3. Split tests by failure boundary: admission, dispatch, node progress,
   completion, cancellation, terminal reconciliation and legacy recovery.
   Shared fixtures belong in `apps/web/test/support`, not production modules.
4. Exercise stale reads, independent terminal winners, deleted records, failed
   commits and a healthy sibling. A completed scan and a completed job are
   different assertions. Test ordinary jobs without workflow records explicitly.
5. Run SQLite transaction/race cases and isolated memory/JSON persistence cases
   on their applicable backends. Do not report skips as passes or treat local
   process/transaction tests as physical power-loss or multi-host qualification.
6. Extend the existing Orchestra operational evidence shard and dated report.
   Keep source ownership in the map, protocol semantics in `docs/protocols.md`,
   retention rules in the HTML lifecycle chapter, and run totals in reports.

## Store Source And Test Placement

Keep the Store in Workbench. The [ownership map](current-architecture-map.md#workspace-store-ownership)
names its existing presentation, catalog, command, manifest and backend owners.

1. Keep filtering and reference normalization in the current pure helpers.
   GUI and PWDT must share commands and conflict rules, not duplicate writes.
2. Keep source/API adaptation behind `store-backend-service`; components render
   observed capabilities and failures, not invented login/install success.
3. Keep identity and commerce policy with the configured service. Package
   verification, installation and Agent execution remain distinct native gates.
   Do not implement a second account system or require vendor login for self-hosting.
4. Put pure command/catalog tests in the frontend workflow tests and browser
   journeys in `tests/integration`. Extend the existing GUI manifest and tensor
   evidence. Real identity/download/install journeys require separate evidence.
5. Keep login callbacks/sessions and authorized retrieval in `desktop-runtime`,
   installation/variant staging in `installer`, and deployment control in the
   Headless SDK and Agent. A project pin is not a running operator. Use the
   [current native source map](current-architecture-map.md#workspace-store-ownership)
   instead of introducing a second Store orchestrator.

The [HTML handoff](operator-store-integration.html) owns implemented contracts,
qualification boundaries and pending integrations; this guide does not duplicate it.

## Native Operator Isolation Placement

Keep `cli/src/operator_package_process.rs` as the supervision boundary and
`cli/src/operator_package_process/worker.rs` as its private child implementation.
CLI startup and package bindings use the parent module; they do not import the
child's ABI/loading internals directly. Engine stays reusable and does not own
Agent lifecycle or Store policy. The child reuses the existing CLI executable,
not a new top-level service, crate, or sidecar packaging contract.

`cli/src/operator_package_process/fetch.rs` reuses that supervisor for Installer
downloads. It must not load plugin code, activate generations or pass download
credentials to native execution workers. The parent retains generation ownership
and waits for termination before cleanup; Installer remains the authority for
download and package validation. Do not replace blocking HTTP with detached
threads that can outlive cancellation or publish packages after task termination.

`operator_package_generation_session` owns cache session coordination. Agents and
downloaders hold independent shared leases; cleanup requires an exclusive lease
and a stable cache-level catalog lock. Serialize creation, attachment and removal,
not the HTTP transfer itself. Retain busy sessions and rescan before explicit
generation preparation; do not add a background replay worker or PID-based kill.
Keep the reusable catalog lock file in place so concurrent openers cannot split
ownership across different lock inodes.

Its `initialization` submodule records creation intent before creating a session
directory. Keep partial-initialization recovery separate from ordinary stale
session removal: match the intent, inspect only known metadata and empty generation
directories, require an available lease, and never recursively delete uncertain
contents. A malformed intent protects its syntactically associated session from
the ordinary janitor as well. Test-only process barriers belong in the submodule's
tests, never production environment controls. This is not a new task journal,
automatic replay mechanism or a claim of physical power-loss durability.

The nested `initialization/staging` module publishes synced records without
replacing an existing name. Versioned staging names bind reconstructible identity
and expected-content SHA-256; cleanup validates both the name and actual prefix,
and can only unlink staging. It never authorizes session removal. Keep staging
and session cleanup counts separate, preserve conflicting/unknown files, and do
not replace no-replace publication with an overwriting rename or a copy fallback.

Keep bounded filesystem failure qualification in `initialization/io_failures`:
test-only sync injection, owned-child file-size limits, and non-root Unix permission
tests. Only `NotFound` establishes an absent target; other inspection failures
must stop before intent publication. Run the ignored permission test explicitly;
do not conflate process termination, real short writes, injected sync errors and
physical disk failure. Tests must restore owned directory permissions and limits,
not fill disks or change daily service configuration.

`operator_package_generation_session/retirement` owns completed-session cleanup.
Keep its records separate from incomplete initialization: hard-link the existing
validated marker without replacement before removing payload, then unlink the
lease, marker, empty root and finally the retirement record. Never reintroduce
recursive removal of the session root. A pending retirement blocks new downloader
attachment; a surviving lease still blocks reclamation. Conflicting initialization
and retirement records must protect the session from both janitors. Keep crash,
permission and safety qualification in the sibling `retirement_tests` modules;
record cleanup is not task replay, a permanent backup or a physical power-loss claim.

The session `lifecycle` helper shares entry checks for normal close, new generation
preparation, downloader attachment and owned-generation cleanup under catalog coordination. Reuse staging's
association parser instead of inventing a second filename interpretation. Reject
unsafe/unreadable record namespaces and unresolved records for the target session;
do not revoke an existing downloader lease or broaden a peer's record into a global
per-session failure. Publish scan diagnostics before returning a preparation error.
Retired-generation removal and abandoned-preparation drop must enter through
`with_generation_cleanup`: recheck the session identity and lifecycle records,
then retain the catalog lock through deletion. Failed cleanup keeps its owned
generation for a later retry or stale-session recovery; do not replay execution.
Keep these entry-point and namespace regressions in `admission_tests`, with
generation cleanup cases in its `generation_cleanup` child module. This is not
proof for generation-internal partial deletion, physical power loss or hostile
filesystem races; generation cleanup has its own bounded proof below.

`operator_package_generation/cleanup` owns resumable generation deletion. Under
the existing session cleanup guard, retain the validated marker as a no-replace
sibling hard link before deleting payload, then remove the marker, empty root
and journal. A missing root must not skip journal validation and unlink. Retain
runtime queue ownership on failure, and reject new downloader attachment to any
recorded generation without blocking a healthy sibling. Do not replace this with
whole-root recursive deletion or infer ownership from a missing marker. Keep
error/permission/crash/safety tests in `cleanup_tests`, and runtime retry-queue
tests in `operator_package_runtime/retirement_tests`.

`operator_package_generation/retry` owns failed-preparation cleanup, not runtime
activation. The session owns the identity-only queue; its lazy worker holds a
weak session reference between attempts, preventing an ownership cycle. Retry
every 1000 ms and before preparation, at most 16 entries per sweep; rotate failed
entries and deduplicate in-flight roots. Hold no queue-state mutex during I/O,
skip overlapping sweeps, and reuse session identity/lifecycle/catalog checks.
Worker creation failure keeps entries for the next preparation. Pending and
removed counts are live-session diagnostics, not persisted completion records.
An in-progress filesystem operation can extend the session lifetime; a sleeping
worker cannot. Closing the session releases the queue, leaving remaining data
to the existing stale-session protocol. Do not infer ownership by scanning live
directories or enqueue committed/active generations. This is not live orphan
adoption, a permanent backup, or a power-loss guarantee.

Use `cli/tests/operator_process_isolation.rs` for actual crash/hang/cancel and
parent-loss checks. Compile its synthetic library from `tests/fixtures` into an
owned temporary directory, never into distributable templates or retained
release products. Keep deployment, activation repair and Orchestra generation
rotation tests separate; a smaller module namespace does not merge their claims.

## Line-Count Rule

Source files should stay below 800 lines and documentation files should stay
below 2000 lines unless they are generated, binary, lockfiles, or
app-framework generated schemas. When a file crosses the line, split by
responsibility rather than by arbitrary chunks.

Current source-side posture:

- `./scripts/kyuubiki audit-project-organization` is the repository-wide guard
  for this rule and should report `tracked debt 0`. It checks tracked files
  and untracked files that are not ignored, so new source modules are covered
  before they are staged. Run it with `--self-test` when changing audit helper
  logic.
- Hub keeps the shell document small by loading large static markup from
  `apps/hub-gui/ui/hub-static-partials.js` before `app.js` starts. Any new
  static Hub section should follow that boundary instead of growing
  `apps/hub-gui/ui/index.html`.
- The old `scripts/kyuubiki-legacy-*.zsh` shell modules have been removed.
  Long-lived operational command behavior belongs in the native Rust script
  runner. JavaScript tooling is limited to frontend development and UI tests.
  Do not keep a second operational implementation as an unused parity copy
  after its native entrypoint and regression checks have replaced it.
  When removing such a copy, move coverage evidence to the native implementation
  and verify its normal and negative self-test lanes. A deleted filename must
  not remain a required source in the architecture or integrity contracts.
- Workflow templates should split entry metadata, graph assembly, graph nodes,
  and runtime helpers into separate modules once a file starts mixing those
  responsibilities.
- Tests should split by operator family, runtime surface, or fixture domain.
  Shared fixtures should remain narrow enough that a new domain can be moved
  out without changing public fixture APIs.
- Installer tests use `workers/rust/crates/installer/src/tests.rs` only as a
  module index; add new installer tests under the responsibility modules in
  `workers/rust/crates/installer/src/tests/`. The organization audit enforces
  this module-index boundary.
- Dependency-audit lockfiles are required project structure, not disposable
  generated output. The organization audit enforces that the shipped npm
  `package-lock.json` files and Rust/Tauri `Cargo.lock` files used by
  `make audit-dependencies` exist and are not ignored by git. Keep the shared
  lane contract in `config/dependency-audit-lockfiles.json` synchronized with
  shipped app, SDK, and runtime surfaces.
- The organization audit rejects existing tracked files matched by `.gitignore`.
  Adding an ignore rule alone does not untrack an existing cache. Remove the
  artifact, or add a narrow ignore exception for an intentionally owned file.

Allowed large-file categories:

- image assets and icon files
- package lockfiles
- Tauri generated schemas under `src-tauri/gen`
- generated update catalogs when regenerated from source data

## Generated Disk Use

Git ignore rules, runtime ownership and retention policy are separate concepts.
An ignored directory is not automatically safe to delete. In particular,
`tmp/data`, execution receipts, process records and credentials are not caches.

| Class | Paths | Default cleanup policy |
| --- | --- | --- |
| Rebuildable debug artifacts | `workers/rust/target`, `sdks/rust/target`, desktop debug targets | Native `dev-disk` removes incremental/object output and optionally hashed executables |
| Reusable dependencies | Cargo libraries/build dependencies, `apps/web/deps`, `apps/web/_build`, UI `node_modules` | Keep during ordinary cleanup to avoid network downloads and full recompilation |
| Shared packaging cache | `target/desktop-cache/<platform>/release` | Keep current reusable release cache; do not merge the three Apps |
| Frontend output | `apps/frontend/.next` | Native cleanup targets `.next/cache`, not the exported/current UI payload |
| Release products | `dist`, desktop bundles, runtime payloads | Separate review; do not delete the only usable package as cache |
| Mixed local state | `tmp`, `results`, `.kyuubiki`, ignored environment files | Inspect by owner; preserve live state, credentials and non-reproducible inputs |
| Retained evidence | `reports`, `evidence`, `releases` | Preserve test baselines, failure evidence and qualification references |

Use the existing native maintenance path:

```text
make disk-usage
make clean-dev-cache
```

The first command previews; the second explicitly opts into removal of hashed
debug Cargo executables as well as incremental/object/frontend caches. Underlying
`./scripts/kyuubiki dev-disk --apply` omits the executable cleanup for a smaller
tradeoff. Cargo output locks block cleanup during builds; ensure tests and
frontend build/dev servers are idle too. Tracked output and symlinked roots
reject cleanup. Top-level tools, dependency libraries, release products and
runtime data are retained. Subsequent tests may need to relink.

Check process ownership separately from Cargo locks. A finished or interrupted
test may leave an Agent or BEAM child alive after its parent exits. Confirm the
test command, parent, working directory and owned log before stopping exact PIDs;
age or a process name alone is not authorization. Recheck released test ports and
the installed service listeners. Do not delete the test's data until its owner
and retention needs are known. Stopping leftovers does not fix the harness leak.

Preview bytes are logical file sizes, not promised physical recovery: hard links,
APFS accounting and unrelated activity can change the observed `du`/`df` delta.
Never back up caches into the same repository before deleting them.

Keep the existing build roots rather than inventing a target directory per test
round: runtime work uses `workers/rust/target`, independent SDK examples use
`sdks/rust/target`, and native packaging shares `target/desktop-cache/<platform>`.
The runtime test profile already disables incremental output. For one-off dev
build/check campaigns under disk pressure, opt in to `CARGO_INCREMENTAL=0` for
that command; do not silently change the team's normal edit/build tradeoff.

Temporary run reports are not regression-test source. Keep test fixtures,
benchmark baselines, schemas, lockfiles, and release qualification evidence
unless their replacement is verified. Do not delete tests merely because they
describe an old bug. Keep only intentionally retained summaries in `reports/`
or `releases/`; do not back up generated output inside the repository.
For identical repeated run reports, retain one representative result rather
than every timestamped copy. Remove empty or incomplete intermediate reports
only after verifying that the completed report preserves their useful results.
Do not prune independent failure evidence or required qualification artifacts.

Repository cleanup does not uninstall desktop apps or manage installed runtime
data. Use Installer lifecycle/storage operations for those separate locations.
Container build contexts exclude temporary research outputs and local env files.

### Latest Local Maintenance: November 13, 2026

The native cleanup removed 84,199 regenerable entries after the Agent tests and
builds finished. The preview's 23.00 GiB was logical size, not recovered space.
The same `du -x -k -d 1 .` measurement before and after removal gave:

| Measurement | Before | After |
| --- | ---: | ---: |
| Repository allocated size | 46.14 GiB | 29.76 GiB |
| Data-volume available space (`df -k .`) | 6.98 GiB | 23.96 GiB |

Repository allocated size fell by 16.38 GiB. Volume availability includes
concurrent activity outside the project and is not an isolated cleanup metric.
The combined content digest of tracked and non-ignored untracked files matched
across cleanup. Dependencies, release products, credentials, runtime/research
data, existing Store/SDK/Agent work and installed service processes were kept.
No new version, commit, installation or remote-server cleanup was performed.

The accompanying organization change nests child-only operator code under its
supervisor and updates the architecture map and Rust navigation. Local macOS
verification passed 337 Agent unit tests, nine live operator integration cases,
strict CLI Clippy, workspace formatting, and the focused organization,
topology/matrix/tensor, extension, documentation and Store contract checks.
This was not the full `architecture-check`, a whole-workspace test run or new
Linux/Windows qualification. Tensor maturity/evidence gaps remain unchanged.

The [November 4 organization record](../reports/architecture-disk-organization-20261104.md),
[October 28 record](../reports/architecture-disk-organization-20261028.md),
[October 23 record](../reports/architecture-disk-organization-20261023.md) and
[October 18 record](../reports/architecture-disk-organization-20261018.md)
remain historical evidence, not the current disk inventory. Cleanup totals are
observations, not a standing storage budget: retained dependencies and release
caches explain most remaining build output, and test linking grows it again.

## Refactor Priority

1. Keep TaskIR, execution summary, and schema contracts aligned across Elixir
   and Rust.
2. Move repeated protocol validation into shared protocol modules before adding
   new SDK features.
3. Split files that cross the source or documentation line limit when touching
   their feature area.
4. Keep Hub, Workbench, Installer, Orchestra, Agent, and SDK responsibilities
   visibly separate.
5. Prefer adding examples and golden fixtures for protocol changes before
   expanding UI affordances.
6. Treat `docs/minimal-industrial-closure.md` as a retained minimum-closure
   contract, not the current release roadmap. New TaskIR, installer, agent, operator-reliability, persistence,
   security, UX, and benchmark work should either close one of its gates or
   explicitly remain outside the minimum industrial loop.

## Near-Term Cleanup Queue

- Keep `./scripts/kyuubiki audit-project-organization` green before adding
  new architecture or workflow files.
- Run `make architecture-check` after changing TaskIR, materialization plan
  contracts, orchestration boundary code, external operator package admission
  contracts, architecture docs, or project organization docs.
- Run `make check-ui-automation-contract` before changing product-owned
  Workbench shell, rail, library, runtime, viewport, or control-window anchors.
- Keep Hub static partial smoke coverage aligned with every new shell partial.
- Retire duplicate operational tools only after verifying the native command,
  its callers, and regression coverage. Update current docs and evidence paths
  in the same change; keep historical release snapshots unchanged.
- Keep Make target logic under `make/*.mk`, with the root `Makefile` limited to
  shared variables and includes.
- Move remaining operator TaskIR API examples into schema examples.
- Keep the Rust protocol TaskIR summary as the basis for future agent-native
  execution.
