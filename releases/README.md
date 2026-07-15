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
- `qualification-records/<version>.json`
  Staged qualification evidence records for a release. These bind candidate
  IDs to capture/check commands and evidence bundle paths before any operator
  is promoted beyond review.
- `qualification-evidence/<version>/<candidate>-release-evidence.json`
  Release-retained validation reports captured from the qualification commands.
  These files are checked back through the validation profile runner so the
  release record points at evidence that can be replayed without reopening the
  full workflow UI.
- `qualification-review-decisions/<version>/<candidate>-review-decision.json`
  Reviewer-authored decision records for qualification candidates. These bind
  the reviewer identity, decision, release version, evidence path, and review
  gate before a release record can move from pending sign-off to another review
  status.
- `update-catalog.json`
  Generated channel-to-version registry that gives desktop and installer
  surfaces one unified update view.

## Scaffolding

Create the next snapshot scaffold with:

```bash
node ./scripts/create-release-snapshot.mjs --self-test
node ./scripts/create-release-snapshot.mjs 1.6.1 --status staged --dry-run
node ./scripts/create-release-snapshot.mjs 1.6.1 --status staged
```

The script updates `releases/index.json` and creates
`releases/snapshots/<version>.json`.

New snapshot scaffolds now seed frontend verification with:

- `npm run typecheck`
- `npm run build`
- `npm run check:workflow-preflight`

That keeps workflow topology plus browser-backed layout/search validation
visible in the release record instead of leaving workflow-heavy frontend
quality as an unwritten expectation.

Snapshot scaffolds also seed repository verification with:

- `git diff --check`
- `make audit-project-organization`
- `make operator-package-preflight`
- `make architecture-check`

That keeps repository organization, installer test module boundaries, external
operator package manifests, SDK API version gates, host version gates, and
read-only dynamic-loading safety visible in release records. The architecture
check is the aggregate guard that also exercises docs manifest validation,
focused Operator TaskIR checks, and the Rust live operator task path.

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
