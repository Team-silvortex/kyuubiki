# Version Line: daji 3.x

The active product identity is defined by the
[version policy](../config/version-line-policy.json) and
[brand metadata](../assets/brand/brand.json).

## Formal Line Record

- codename: `daji`
- active line: `3.x`
- current development point: `daji 3.7.7`
- current documentation target: `daji 3.7.x` line
- current source packaging target: `daji 3.7.7`
- local macOS installed baseline: `daji 3.7.7`, verified November 4, 2026 in the [current snapshot](../releases/snapshots/3.7.7.json); previous baseline `3.7.1` is historical
- latest version-named Git checkpoint: `f28de0dd`, subject `daji 3.7.7`; not by itself an installed version
- build provenance: base `3fe22542` plus working-tree overlay, not a clean-commit release
- first version: `3.0.0`
- terminal version: `3.20.9`
- cadence: minor positions `0..20`, patch positions `0..9`
- public launch channel: Reddit, subject to the retained readiness gates
- next line: not yet declared

## Transition Rules

Moxi 2.x and Tamamono 1.x are historical lines. Moxi's planned final
stabilization window is retained in `archived_lines`; it no longer constrains
the active Daji line. This transition does not assert that every planned moxi
patch was shipped.

Current development documentation follows `docs/book-manifest.json` and is
aligned on `3.7.7`. Desktop/runtime manifests, first-party SDK package metadata,
brand mirrors, update channels, and language-pack targets also use `3.7.7`.
The macOS rebuild, installation, three-shell startup checks and bounded
Workbench observation are tracked separately in the release snapshot. Registry publication
and public distribution are separate actions; this local acceptance does not
establish all-platform or complete GUI qualification.
Protocol/schema versions and persistent storage identifiers do not follow
product SemVer mechanically.

The [version/evidence ledger](current-line.md#version-and-evidence-ledger) owns
the active distinction between package metadata, recorded installation and
later source work. A commit subject alone does not reconcile those identities.

`make check-version-line` verifies the active policy rather than assuming
that the major must be 2. Historical snapshots and evidence keep their
original identity; a source version bump never promotes release readiness.

See [current-line.md](current-line.md), [daji-3.0.0.md](daji-3.0.0.md),
and [moxi-closeout.md](moxi-closeout.md).
