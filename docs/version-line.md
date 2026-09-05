# Version Line: daji 3.x

The active product identity is defined by the
[version policy](../config/version-line-policy.json) and
[brand metadata](../assets/brand/brand.json).

## Formal Line Record

- codename: `daji`
- active line: `3.x`
- current development point: `daji 3.0.0`
- current documentation target: `daji 3.0.x` line
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

All shipping packages, first-party SDKs, brand mirrors, update channels,
language-pack targets, and current documentation align on `3.0.0`.
Protocol/schema versions and persistent storage identifiers do not follow
product SemVer mechanically.

`make check-version-line` verifies the active policy rather than assuming
that the major must be 2. Historical snapshots and evidence keep their
original identity; a source version bump never promotes release readiness.

See [current-line.md](current-line.md), [daji-3.0.0.md](daji-3.0.0.md),
and [moxi-closeout.md](moxi-closeout.md).
