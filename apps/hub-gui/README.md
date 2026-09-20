# Hub GUI

Hub GUI is the desktop operator shell for workload entry, runtime posture, and
project/workflow navigation.

## Bundle Entry

Bundle tools separates creating or opening a file from advanced archive work.
Create opens a name/location dialog with a native folder chooser and destination
preview; Open uses the native `.kyuubiki` file chooser. Existing files are never
overwritten, cancellation keeps the current bundle, and failures retain the draft.
Advanced tools, operation details, and recent history are initially collapsed.

The fixed `project_bundle_pick_path` command only selects paths. File creation
still goes through `guarded_mutation_action`, with directory/name validation in
`kyuubiki-project-bundle`. PWDT keeps its existing raw-path creation route and
does not require a modal or native chooser. No frontend filesystem scope or
Node runtime dependency is added. The UI boot fallback cannot create files.

See the [HTML walkthrough](../../docs/tutorial-first-research.html#hub-bundle-file).
Browser regression coverage is in `tests/integration/hub-bundle-creation.shared.mjs`;
native filesystem cases are in `workers/rust/crates/project-bundle/tests/create.rs`.

## UI Source Layout

The assistant runtime context starts collapsed with a short running/total count.
Expanding it reveals a service list; configuration/paths and raw diagnostics have
their own disclosures. Native structured states drive both the count and guide
recommendations, while raw output remains selectable text. Refresh and language
changes preserve disclosure state. A failed refresh clears stale service states.
See `test/hub-assistant-runtime.test.mjs` and
`tests/integration/desktop-hub-assistant-context.test.mjs` for regression coverage.

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
