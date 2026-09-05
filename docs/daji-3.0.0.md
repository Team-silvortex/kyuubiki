# Daji 3.0.0

Release-line transition: 2026-09-05.

## Aligned Surfaces

- Hub, Workbench, and Installer remain separate cross-platform applications.
- Orchestra, the Rust workspace, Worker SDK, and official Rust/Python/Elixir
  Headless SDK package versions align to 3.0.0.
- Product branding, language-pack targets, installation integrity, update
  channels, release snapshot, HTML book, and Hub documentation share Daji.
- The existing stable channel now names Daji, with `daji:stable`,
  `daji:latest`, `daji:3`, `daji:3.0`, and `daji:3.0.0` aliases.
  These are catalog identities, not proof that a remote artifact is published.
- Existing protocol versions, bundle IDs, managed roots, credentials, and
  stored projects are not renamed. SDK package versions do not alter TaskIR,
  HTTP, workflow, or KCore contracts.

## Build And Upgrade

`make check-version-line` checks package and lockfile metadata as well as
current documentation. `make desktop-build-host BUNDLES=app` builds the three
macOS applications without producing redundant disk images.
Linux and Windows retain their native build/qualification lanes.

Existing installed applications are unchanged until an explicit install or
update is performed. Cross-major upgrade/rollback and public signing remain
separate, platform-specific qualification work.

## Evidence And Limits

The [3.0.0 snapshot](../releases/snapshots/3.0.0.json) records this transition
and its verification scope. Historical Moxi snapshots and retained numerical,
scale, recovery, and usability reports remain at their original paths.

The coverage tensor is still the work map. Version alignment does not waive
its open gates or assert general availability, independent solver
certification, or complete GUI/PWDT parity.

Start with [the book](book.html) and [the current line](current-line.md).

## Transition Verification

- 222 version/codename contracts pass with no mismatches.
- 359 native script-runner tests pass; Clippy reports no warnings.
- The frontend production build, typecheck, and product-identity test pass.
- All 60 language packs, book links, documentation inventory, and the
  source/document line limits pass.
- All three macOS source-built application bundles report 3.0.0, pass
  signature-integrity checks, and pass interactive-startup smoke tests.
- Applications were not reinstalled, notarized, or uploaded. Linux and Windows
  metadata is aligned, but their new native packages were not built here.
- The coverage tensor's structural check passes; its Daji readiness status
  remains blocked. This milestone does not override that evidence boundary.
