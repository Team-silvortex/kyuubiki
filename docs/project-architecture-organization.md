# Project Architecture Organization

This document is the working organization map for the `tamamono 1.15.x`
architecture line. It exists to keep feature work, protocol work, and cleanup
moving in the same direction.

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
- `SDKs`
  are headless protocol clients. They must be able to use core capabilities
  without going through the frontend.
- `Schemas`
  are the shared contract layer used by GUI, control plane, SDKs, installer,
  and agents.

## Current Source-Of-Truth Files

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

## New Operator TaskIR Stack

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
11. Agent RPC params now carry an optional `mode` field: `preflight` is the
   default and `execute` is the reserved path for package dispatch.
12. Agent preflight exposes an `operator_package_runtime` contract that points
   at `kyuubiki-engine.operator-sdk-host/v1` and its default trust boundary.
13. Full agent-side execution still needs the operator package runtime to attach
   package fetch, package integrity, dispatch, and result serialization.

The implementation anchors are:

- `apps/web/lib/kyuubiki_web/orchestra/operator_task_ir.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_execution_program.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_execution_summary.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_envelope.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_executor.ex`
- `workers/rust/crates/protocol/src/operator_task_ir.rs`
- `workers/rust/crates/protocol/src/types/rpc.rs`
- `workers/rust/crates/cli/src/operator_task_runtime.rs`
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
- `schemas`
  should hold JSON contracts and examples, not app-specific implementation
  prose.
- `make`
  should hold modular Make target groups. The root `Makefile` should stay a
  small include-based entrypoint instead of growing target logic directly.
- `docs`
  should hold source-of-truth architecture documents. Hub docs may mirror them
  for operator reading, but should not silently become the deeper source.

## 600-Line Rule

Source files should stay below 600 lines unless they are generated, binary,
lockfiles, or app-framework generated schemas. When a source file crosses the
line, split by responsibility rather than by arbitrary chunks.

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
  Long-lived command behavior belongs in the native Rust script runner or a
  narrow Node utility when JavaScript ecosystem integration is required.
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

Allowed large-file categories:

- image assets and icon files
- package lockfiles
- Tauri generated schemas under `src-tauri/gen`
- generated update catalogs when regenerated from source data

## Generated Disk Use

These paths are disposable local output and should not guide architecture
decisions:

- `workers/rust/target`
- `sdks/rust/target`
- `apps/frontend/.next`
- `apps/web/_build`
- `apps/web/deps`
- `tmp`

They are already covered by `.gitignore`. Clean them when local disk pressure
matters; do not move source boundaries just to reduce generated output.

## Refactor Priority

1. Keep TaskIR, execution summary, and schema contracts aligned across Elixir
   and Rust.
2. Move repeated protocol validation into shared protocol modules before adding
   new SDK features.
3. Split files that cross 600 lines only when touching their feature area.
4. Keep Hub, Workbench, Installer, Orchestra, Agent, and SDK responsibilities
   visibly separate.
5. Prefer adding examples and golden fixtures for protocol changes before
   expanding UI affordances.
6. Treat `docs/minimal-industrial-closure.md` as the `1.16` to `1.20`
   bridge. New TaskIR, installer, agent, operator-reliability, persistence,
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
- Keep legacy shell modules below the shared line ceiling until native commands
  can retire them.
- Keep Make target logic under `make/*.mk`, with the root `Makefile` limited to
  shared variables and includes.
- Move remaining operator TaskIR API examples into schema examples.
- Keep the Rust protocol TaskIR summary as the basis for future agent-native
  execution.
