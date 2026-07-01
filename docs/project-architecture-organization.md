# Project Architecture Organization

This document is the working organization map for the `tamamono 1.13.x`
architecture line. It exists to keep feature work, protocol work, and cleanup
moving in the same direction.

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
- Agent and orchestrator boundary:
  `docs/agent-orchestrator-boundary.md`
- Headless agent contract:
  `docs/headless-agent-contract.md`
- Operator TaskIR digest rules:
  `docs/operator-task-ir-digest.md`
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
9. Full agent-side execution still needs the operator package runtime to attach
   package fetch, package integrity, dispatch, and result serialization.

The implementation anchors are:

- `apps/web/lib/kyuubiki_web/orchestra/operator_task_ir.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_execution_program.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_execution_summary.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_envelope.ex`
- `apps/web/lib/kyuubiki_web/orchestra/operator_task_executor.ex`
- `workers/rust/crates/protocol/src/operator_task_ir.rs`
- `workers/rust/crates/protocol/src/types/rpc.rs`
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
- `docs`
  should hold source-of-truth architecture documents. Hub docs may mirror them
  for operator reading, but should not silently become the deeper source.

## 600-Line Rule

Source files should stay below 600 lines unless they are generated, binary,
lockfiles, or app-framework generated schemas. When a source file crosses the
line, split by responsibility rather than by arbitrary chunks.

Current source-side split queue:

- `apps/web/lib/kyuubiki_web/workflow_template_bridge_contract_graphs.ex`
  should split template data from graph assembly helpers.
- `apps/web/lib/kyuubiki_web/playground/agent_pool.ex`
  should split pool selection, manifest loading, and registry fallback.
- `apps/web/lib/kyuubiki_web/playground/agent_registry.ex`
  should split registry state, lease/authority handling, and serialization.
- `apps/web/lib/kyuubiki_web/workflow_template_electromagnetic_guard_thermo_entries.ex`
  should split electromagnetic entries from thermo guard entries.
- `apps/web/test/kyuubiki_web/workflow_operator_runtime_test.exs`
  should split by operator family.
- `apps/web/test/kyuubiki_web/api/workflow_catalog_api_test.exs`
  should split catalog listing, job submission, and template-detail assertions.
- `apps/web/test/support/workflow_api_fixtures.exs`
  should split catalog fixtures, runtime fixtures, and assertion helpers.
- `apps/web/test/kyuubiki_web/workflow_template_catalog_test.exs`
  should split template-family coverage.
- `apps/hub-gui/ui/index.html`
  should continue moving app logic into typed modules before adding new Hub
  sections.
- `scripts/kyuubiki-legacy.zsh`
  should keep shrinking as native Rust script-runner commands replace shell
  behavior.

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

## Near-Term Cleanup Queue

- Keep `node ./scripts/audit-project-organization.mjs` green before adding
  new architecture or workflow files.
- Run `make architecture-check` after changing TaskIR, orchestration boundary
  code, architecture docs, or project organization docs.
- Split `agent_registry.ex` before adding more mesh authority behavior.
- Split `workflow_template_bridge_contract_graphs.ex` before adding more bridge
  templates.
- Move remaining operator TaskIR API examples into schema examples.
- Keep the Rust protocol TaskIR summary as the basis for future agent-native
  execution.
