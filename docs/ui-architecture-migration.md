# UI Architecture Migration

This page tracks the `1.14.x` UI refactor path for moving the desktop UI away
from ad-hoc JavaScript modules into typed, layered contracts without breaking
the Tauri shells.

## Direction

- Source TypeScript lives in the owning source tree.
- Generated JavaScript is still copied into app-local `ui/` folders for Tauri
  packaging predictability.
- Shared desktop contracts move first because Hub, Installer, and Workbench all
  consume them.
- App-specific UI modules move after their shared inputs are typed.
- Runtime payloads stay `unknown` at the boundary and are normalized before
  rendering.
- Files stay under 600 lines; split by contract, model, renderer, or controller
  when needed.

## Layer Map

- `apps/desktop-shared/src/platform.ts`
  Shared platform detection, labels, and release-root helpers.
- `apps/desktop-shared/src/tauri-bridge.ts`
  Tauri invoke/listen helpers, global language preference, brand loading, and
  desktop state styling.
- `apps/desktop-shared/src/runtime-status-types.ts`
  Typed runtime status model structures consumed by renderer and future app
  controllers.
- `apps/desktop-shared/src/runtime-status-model.ts`
  Runtime/mesh status normalization, filters, topology groups, and detail
  selection.
- `apps/desktop-shared/src/runtime-status-summary.ts`
  Runtime status formatting and DOM rendering from the typed model contract.
- `apps/desktop-shared/ui/*.js`
  Generated distribution files synchronized into Hub, Installer, and Workbench.
- `apps/hub-gui/src/hub-app-config.ts`
  Hub-owned action, storage, density, and risk constants compiled into
  `apps/hub-gui/ui/hub-app-config.js`.
- `apps/hub-gui/src/hub-state.ts`
  Hub-owned state shape compiled into `apps/hub-gui/ui/hub-state.js`, with
  narrow `.d.ts` shims for JS modules not yet migrated.
- `apps/hub-gui/src/hub-storage.ts`
  Hub-owned local/session storage boundary for recents, workload library
  handles, assistant settings, trusted hosts, audits, logs, and density.
- `apps/hub-gui/src/hub-workload-library.ts`
  Hub-owned workload library normalization, catalog validation, provenance, and
  labeling contract.
- `apps/hub-gui/src/hub-action-contexts.ts`
  Typed action context builders that narrow the large Hub app context before
  project, runtime, workload, and desktop action handlers run.
- `apps/hub-gui/src/hub-action-runner.ts`
  Typed Hub action dispatcher that preserves confirmation, busy-state,
  timestamp probes, and project/runtime/workload/desktop handler order.
- `apps/hub-gui/src/hub-project-actions.ts`
  Typed project/document/bundle action handler.
- `apps/hub-gui/src/hub-runtime-actions.ts`
  Typed local/hot/observe runtime action handler.
- `apps/hub-gui/src/hub-workload-actions.ts`
  Typed workload library and workflow catalog action handler.
- `apps/hub-gui/src/hub-desktop-actions.ts`
  Typed desktop/package/status action handler.
- `apps/hub-gui/src/hub-copy-registry.ts`
  Typed Hub language-pack registry, import manifest, storage cache, and
  shape-preserving copy merge contract.
- `apps/hub-gui/src/hub-i18n-types.ts`
  Shared loose i18n registry shape used while feature copy files are migrated.
- `apps/hub-gui/src/hub-i18n-core.ts`
  Typed Hub i18n composition entry that wires base languages and feature copy
  patchers into one registry.
- `apps/hub-gui/src/hub-i18n-docs.ts`
  Typed docs/current-line feature i18n patch.
- `apps/hub-gui/src/hub-i18n-guides.ts`
  Typed guides/assistant docs feature i18n patch.
- `apps/hub-gui/src/hub-i18n-localization.ts`
  Typed localization override panel feature i18n patch.
- `apps/hub-gui/src/hub-i18n-assistant.ts`
  Typed local assistant and next-step guide feature i18n patch.
- `apps/hub-gui/src/hub-i18n-workloads.ts`
  Typed workload library, bundle tools, and direct-mesh regression feature i18n
  patch.
- `apps/hub-gui/src/hub-localization-panel.ts`
  Typed Hub localization panel renderer and import/export/clear bindings for
  language-pack overrides.
- `apps/hub-gui/src/hub-workflow-catalog.ts`
  Typed workflow catalog search, reference sample submit/poll flow, and DOM
  renderer.
- `apps/hub-gui/src/hub-workload-runtime.ts`
  Typed workload runtime actions for remote catalog sync, bundle download,
  local bundle registration, workload attach/open, and library import/export.
- `apps/hub-gui/src/hub-workload-list.ts`
  Typed workload list filters, search highlighting, workload action buttons,
  and DOM rendering contract.
- `apps/hub-gui/src/hub-library-controls.ts`
  Typed workload and workflow catalog filter/search controls with
  requestAnimationFrame render scheduling.
- `apps/hub-gui/src/hub-workload-panel.ts`
  Typed workload panel glue that connects runtime actions, workload list
  rendering, copy fallback, filters, and Workbench handoff.
- `apps/hub-gui/src/hub-streaming-runtime.ts`
  Typed UI chunk runtime for registering, hydrating, warming, releasing, and
  snapshotting streamable WebView UI regions.
- `apps/hub-gui/src/hub-streaming-setup.ts`
  Typed streamable region registration for sections, project panes, panel
  panes, and assistant overlay hydration hooks.

## Current Coverage

- Shared desktop platform contract is typed.
- Shared Tauri bridge is typed.
- Shared runtime status model is typed.
- Shared runtime status renderer is typed.
- Hub, Installer, and Workbench package manifests declare ESM through
  `"type": "module"`.
- Hub has an app-local TypeScript compile path for migrated Hub-owned modules.
- Hub app config is typed and generated from `apps/hub-gui/src`.
- Hub state shape is typed and generated from `apps/hub-gui/src`.
- Hub storage reads and writes are typed and generated from `apps/hub-gui/src`.
- Hub workload library and remote catalog validation are typed and generated
  from `apps/hub-gui/src`.
- Hub action context slicing is typed and generated from `apps/hub-gui/src`.
- Hub action dispatch is typed and generated from `apps/hub-gui/src`.
- Hub direct action handlers are typed and generated from `apps/hub-gui/src`.
- Hub language-pack override registry is typed and generated from
  `apps/hub-gui/src`.
- Hub i18n composition is typed and generated from `apps/hub-gui/src`.
- Hub docs and guides i18n patches are typed and generated from
  `apps/hub-gui/src`.
- Hub localization, assistant, and workload i18n patches are typed and
  generated from `apps/hub-gui/src`.
- Hub localization panel helpers are typed and generated from
  `apps/hub-gui/src`.
- Hub workflow catalog search, sample run, and renderer are typed and generated
  from `apps/hub-gui/src`.
- Hub workload runtime actions are typed and generated from
  `apps/hub-gui/src`.
- Hub workload list filtering, highlighting, and action rendering are typed and
  generated from `apps/hub-gui/src`.
- Hub workload and workflow catalog controls are typed and generated from
  `apps/hub-gui/src`.
- Hub workload panel glue is typed and generated from `apps/hub-gui/src`.
- Hub streaming UI chunk runtime is typed and generated from
  `apps/hub-gui/src`.
- Hub streaming setup is typed and generated from `apps/hub-gui/src`.
- Hub startup phases and deferred startup scheduling are typed and generated
  from `apps/hub-gui/src`.
- Hub localized shell language relabeling, section copy refresh, and
  cross-panel rerender entrypoint are typed and generated from
  `apps/hub-gui/src`.
- Hub assistant shell, bundle panel, library panel, and workspace group copy
  renderers are typed and generated from `apps/hub-gui/src`.

## Next Migration Targets

1. Hub streaming and startup modules:
   deferred hydration policy enhancements after the typed runtime/setup/startup
   path settles.
2. Hub localization and language-pack modules:
   base language registry files and remaining home/panel/guides copy renderer
   shims used by `hub-localized-shell.ts`.
3. Workload runtime and workflow catalog modules:
   split workflow search/run/render if the catalog grows past the current
   single-module boundary.
4. DOM composition modules:
   panel renderers, list renderers, and event binders after their state/action
   contracts are typed.

## Guardrails

- Do not import generated `ui/shared/*.js` back into source TypeScript.
- Do not edit synchronized generated files directly; edit `apps/desktop-shared/src`
  or the owning app source instead.
- Keep Tauri shell distribution app-local until packaging has a stronger shared
  asset story.
- Preserve existing DOM contracts used by wasm Python automation.
- Prefer narrow `unknown` inputs plus runtime normalization over broad global
  `any` payload assumptions.
- Run smoke tests for all affected desktop shells after shared UI changes.

## Verification

- `node apps/desktop-shared/scripts/sync-desktop-shared.mjs`
- `npm --prefix apps/hub-gui run test:smoke`
- `npm --prefix apps/installer-gui run test:smoke`
- `npm --prefix apps/workbench-gui run test:smoke`
- `npm --prefix apps/frontend run check:file-lines`
- `git diff --check`
