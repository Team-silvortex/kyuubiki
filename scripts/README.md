# Scripts

This directory contains host-native operational entry points.

- `kyuubiki`
  Unified launcher for local, cloud, and distributed development flows.
- `kyuubiki-lab`
  Thin operational wrapper for the shared Ubuntu lab machine that now hosts
  the standard download/deploy server plus the shared solver-agent test node.
- `create-release-snapshot.mjs`
  Scaffold a new lightweight release snapshot manifest and update the release
  index. When a snapshot is marked `current`, it also advances the shared
  shipping-version contracts.
- `build-update-catalog.mjs`
  Generate the shared update catalog JSON plus HTML docs from release snapshots
  and the human-owned channel contract.
- `build-installation-integrity-docs.mjs`
  Generate the installation integrity HTML docs for both the repository-level
  book and the Hub-facing mirror shelf.
- `audit-version-line.mjs`
  Audit repository-wide version contracts and inventory visible version
  references before advancing a shipping line such as `tamamono 1.7.0`.
- `check-doc-book.mjs`
  Verify the centralized docs book and Hub mirrors for version alignment,
  broken local links, required chapter markers, and old legacy wording.
- `sync-doc-book-version.mjs`
  Update the hand-maintained book entry pages to the current shipping version
  without touching the generated installation or update-catalog pages.
- `release-metadata.mjs`
  Shared release-path, JSON, artifact, and shipping-version helpers used by the
  release and installation-doc generators.

Use this directory for operator-facing workflow wrappers, not for source
libraries or generated output.

Typical responsibilities:

- start/stop/restart orchestration
- hot-reload/watch orchestration for local development
- mode switching (`local`, `cloud`, `distributed`)
- verification/test wrappers
- component-scoped build entry points
- runtime and desktop packaging entry points
- installer entry points
- release snapshot scaffolding
- unified update-catalog generation
- release metadata normalization across `releases/` and `deploy/`

Useful smoke wrappers:

- `./scripts/kyuubiki smoke`
  Current Elixir -> Rust integration smoke flow.
- `./scripts/kyuubiki sdk-smoke`
  Python / Elixir / Rust headless SDK smoke suite.
- `./scripts/kyuubiki frontend-test`
  Frontend typecheck plus production build verification.

Examples now include:

- `hot-local`
- `hot-cloud`
- `hot-distributed`
- `hot-web`
- `hot-agent`
- `hot-hub-gui`
- `hot-installer-gui`
- `hot-workbench-gui`
- `build-frontend`
- `build-orchestrator`
- `build-agent`
- `build-hub-gui`
- `build-installer-gui`
- `build-workbench-gui`
- `package-runtime`
- `package-desktop`
- `desktop-status`
- `desktop-stage`
- `desktop-build-host`
- `desktop-release`
- `desktop-verify`
- `sync-desktop-shared`
- `test-hub-gui`
- `test-installer-gui`
- `test-workbench-gui`

Keep these scripts thin. Product logic should live in the application/runtime
code, not in shell branching.

Hot-reload note:

- Next.js and Tauri already provide their own dev/HMR loops.
- `./scripts/kyuubiki hot-*` adds the missing restart-on-change layer for the
  non-Phoenix Elixir control plane and Rust solver agents so the whole stack
  can iterate under one operator command.
