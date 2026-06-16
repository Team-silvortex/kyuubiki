# tamamono 1.8 Prep

Use this page as the practical handoff for preparing the repository to move
from `tamamono 1.7.x` into `tamamono 1.8.x`.

This is intentionally not the product roadmap. It is the release-line hygiene
and coordination checklist that keeps the visible version contract coherent.

## Goal

Before the repo claims `1.8.0`, we want three things to be true:

- the runtime and package manifests agree on one shipping version
- the desktop shells, docs, and update surfaces show the same line
- release metadata can be advanced without hunting through scattered strings

## First command

Run the version audit before touching any versioned surface:

```bash
node ./scripts/audit-version-line.mjs --expected 1.7.0 --next 1.8.0
```

That audit gives us two views:

- exact contract checks
  package manifests, Cargo manifests, Tauri manifests, brand files, and
  release contracts that should already agree on the current shipping version
- next-version candidates
  textual and UI-facing references that will likely need review during the
  `1.8.0` transition

## 1.8 surfaces to review

Treat these as one release unit, not as independent chores:

- repo-level version line docs
  `README.md`, `docs/current-line.md`, `docs/version-line.md`
- desktop shells
  Hub, Workbench, and Installer package manifests, Tauri manifests, and
  `ui/assets/brand.json`
- frontend-visible version badges and workflow defaults
  workbench rail labels, local workflow version strings, docs pages, and
  language-pack defaults
- update and integrity contracts
  `deploy/update-channels.json`,
  `deploy/installation-integrity-contract.json`,
  `releases/index.json`,
  `releases/update-catalog.json`
- generated operator-facing HTML
  `docs/update-catalog.html`,
  `docs/installation-integrity-contract.html`,
  `apps/hub-gui/ui/docs/*.html`
- release snapshot metadata
  `releases/snapshots/<version>.json`

## Recommended sequence

1. Keep the shipping line at `1.7.0` until the 1.8 contract is ready.
2. Run the audit and clear any 1.7 drift that already exists.
3. Create the `1.8.0` snapshot scaffold:

```bash
node ./scripts/create-release-snapshot.mjs 1.8.0 --status staged --dry-run
```

4. Advance the shipping contract only when the repository-wide visible version
   is ready to move together.
5. Rebuild generated release docs after contract changes:

```bash
node ./scripts/build-update-catalog.mjs
node ./scripts/build-installation-integrity-docs.mjs
```

6. Run the audit again with the new expected version.

## What this prep step improves

The important part is not just the next version bump. The important part is
that future `1.x` transitions stop depending on memory and manual grep passes.

`scripts/audit-version-line.mjs` is now the first checkpoint before:

- promoting a staged snapshot to current
- rebuilding desktop-facing release docs
- packaging Hub, Workbench, or Installer bundles
- claiming a new shipping line in README or docs
