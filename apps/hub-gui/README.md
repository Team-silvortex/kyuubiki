# Hub GUI

Hub GUI is the desktop operator shell for workload entry, runtime posture, and
project/workflow navigation.

## UI Source Layout

- `src/`
  TypeScript source for Hub-owned UI contracts as they are migrated.
  Current migrated sources include `hub-app-config.ts`, `hub-state.ts`,
  `hub-storage.ts`, `hub-workload-library.ts`, `hub-action-contexts.ts`, and
  `hub-action-runner.ts`, `hub-copy-registry.ts`, `hub-i18n-core.ts`,
  `hub-i18n-docs.ts`, `hub-i18n-guides.ts`,
  `hub-i18n-localization.ts`, `hub-i18n-assistant.ts`,
  `hub-i18n-workloads.ts`, `hub-localization-panel.ts`,
  `hub-workflow-catalog.ts`, `hub-workload-runtime.ts`,
  `hub-workload-list.ts`, `hub-workload-panel.ts`,
  `hub-library-controls.ts`, `hub-streaming-runtime.ts`,
  `hub-streaming-setup.ts`, `hub-startup-phases.ts`,
  `hub-localized-shell.ts`, `hub-assistant-shell.ts`,
  `hub-bundles-copy.ts`, `hub-library-copy.ts`,
  `hub-workspace-groups.ts`, plus direct project/runtime/workload/desktop
  action handlers.
- `ui/`
  Tauri-facing JavaScript, HTML, CSS, and generated output consumed by the
  desktop shell.
- `ui/shared/`
  Generated shared desktop files synchronized from `apps/desktop-shared`.
- `scripts/compile-ui.mjs`
  Compiles Hub-owned TypeScript into `ui/` before smoke tests or packaging.

Do not edit generated TypeScript outputs in `ui/` when a matching source file
exists under `src/`. Update the source and run:

```sh
npm --prefix apps/hub-gui run compile:ui
```

Some `.d.ts` files under `src/` intentionally bridge legacy `ui/` JavaScript
modules while their callers move to TypeScript. Remove those shims as the
matching modules are migrated.
