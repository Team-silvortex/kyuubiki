# Release Snapshots

This directory stores lightweight version snapshots for shipped or staged
product points.

The goal is not to archive a full source tree for every small version.
Instead, each snapshot records:

- version and line metadata
- source commit
- notable product/runtime capabilities
- released desktop bundle paths
- verification status at the time of the snapshot

## Layout

- `index.json`
  Machine-readable list of known snapshots.
- `snapshots/<version>.json`
  One structured snapshot manifest per version.
- `update-catalog.json`
  Generated channel-to-version registry that gives desktop and installer
  surfaces one unified update view.

## Scaffolding

Create the next snapshot scaffold with:

```bash
node ./scripts/create-release-snapshot.mjs 1.6.1 --status staged --dry-run
node ./scripts/create-release-snapshot.mjs 1.6.1 --status staged
```

The script updates `releases/index.json` and creates
`releases/snapshots/<version>.json`.

If the snapshot is created with `--status current`, the same command also
advances:

- `deploy/update-channels.json`
- `deploy/installation-integrity-contract.json`

Regenerate the unified update catalog after channel or snapshot changes with:

```bash
node ./scripts/build-update-catalog.mjs
```

## Current policy

- add one snapshot per meaningful shipped or staged product point
- prefer updating the snapshot structure carefully rather than creating many
  one-off formats
- keep artifact paths and verification signals concrete
