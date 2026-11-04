# Architecture And Local Disk Organization

Date: November 4, 2026. This is a repository maintenance pass, not a release,
new numerical qualification or installed-application upgrade.

## Architecture Organization

The existing 13 topology modules, 15 Rust workspace crates and three desktop
Apps remain separate views. No production module is merged, moved or deleted.
The organization audit found no source/doc line-limit debt, so this pass does
not split files just to create more layers.

The compact architecture map now indexes the existing Store owners: Workbench
presentation, pure catalog helpers, shared GUI/PWDT commands and manifest,
backend adaptation, and catalog/package services. Project references are not
downloads, installation receipts or execution permission. The HTML handoff owns
the proposed backend contract; shared identity remains Team Silvortex's concern.
Self-hosting does not require a vendor account. This is not a claim that hosted
login, download entitlements or publisher submission are implemented.

The map also catches up with the working-tree LAN policy: source-IP permission
allows discovery by default, trusted-LAN control is explicit, and drain/resume
remain local. Protocol owns the common contract, Agent enforces it, and control
clients consume it. These gates are not TLS or cryptographic identity.

Directory and documentation ownership guides now include configuration,
language packs, retained evidence, release indexes and Store integration.
The book remains the HTML entry point. Existing numerical/recovery reports,
tests, contracts and the new GUI pass/failure reports remain intact.

## Measured Disk Cleanup

Allocated KiB from the same `du -x -k -d 1 .` walk, immediately before/after
cache deletion and before this pass's documentation edits:

| Location | Before KiB | After KiB |
| --- | ---: | ---: |
| Repository | 31,691,356 | 26,216,952 |
| `workers` | 25,269,780 | 19,795,376 |
| Shared desktop `target` | 5,234,536 | 5,234,536 |
| `apps` | 567,012 | 567,012 |
| `sdks` | 374,440 | 374,440 |
| `tmp` | 84,164 | 84,164 |
| `dist` | 56,412 | 56,412 |

Repository allocation fell from 30.22 GiB to 25.00 GiB, a 5.22 GiB reduction.
The existing native `dev-disk --apply --test-binaries` removed 5,442 entries,
reporting 5.60 GiB of logical lengths. Logical lengths are not independent
physical bytes because hard links can overlap. The follow-up preview reports
zero candidates. Filesystem free space changed from 22,840,000 to 28,160,000 KiB;
APFS accounting and unrelated activity prevent treating that as a second exact
project-cleanup measure.

Removed scope: debug incremental/object output and hashed Cargo executables
identified by the existing native policy. Cargo output locks, tracked-file and
symlink checks remain enabled. No cache backup is created. Dependency libraries,
top-level executables, shared desktop release cache, packages and UI/Elixir
dependencies remain, avoiding a forced full rebuild/download. Subsequent tests
will relink their executables and may grow the cache again.

All 97 pre-existing modified/untracked files had identical SHA-256 values after
cache deletion, before intentional documentation edits. No source, local
environment, credential, live database, journal or receipt was deleted.

## Orphaned Test Processes

Process inspection found six Agent fixtures and three Web fixture processes
running for nearly five days with their original test parent gone. Ownership
was checked using exact commands, parent IDs, repository working directories,
test-specific Agent IDs and temporary Agent logs. The Web commands named
`apps/web/test/support/headless_live_server.exs` rather than an installed release.

Only these nine exact processes received graceful termination. They exited and
released their 12 loopback listeners. Installed Agent, Orchestra and frontend
listeners at ports 5001, 5002, 4000 and 3000 remained on their original processes.
Other projects' compiler/test processes were left alone. Unknown fixture data
and logs were not bulk-deleted, and no global cache or Docker prune was used.

This clears the observed leftovers, not the cause of the test-harness leak.
The original interruption was not reconstructed. Follow up in the existing
[Agent fixture](../workers/rust/crates/cli/tests/support/agent_lifecycle.rs) and
[Web fixture launcher](../workers/rust/crates/cli/tests/support/headless_live_server.rs):
test parent loss and failed setup should not leave descendants indefinitely.
Do not add process-name-based killing to production or infer native lifecycle
qualification from this cleanup.

## Verification Scope

The native organization audit passed with zero tracked source/doc line-limit
debt. Topology, module/function matrix, coverage-tensor structure, extension
standard and Make-module checks passed, including their available self-tests.
The tensor still reports two maturity gaps, 23 evidence-grade gaps and 18 P0
gaps; Daji readiness remains blocked. Organization is not qualification.

The documentation inventory initially caught the missing Store handoff entry.
Adding it to `docs/README.md` repaired both the catalog and task navigation.
The inventory and HTML-book checks then passed. A read-only local-link check
resolved 283 file/anchor references in the seven maintained documents.
The GUI-report checker and its self-test passed against the current report.

This pass does not rerun the full browser, HTTP or numerical suites merely to
regenerate the binaries just deleted. The preceding GUI report retains its
actual 660-test pass and initial failure; their evidence grades are not changed
by this maintenance operation. Final cache preview found zero candidates.

No staging, commit, push, version bump, app reinstall, remote-server cleanup or
installed-service restart is part of this pass.

See the [compact map](../docs/current-architecture-map.md),
[source/disk policy](../docs/project-architecture-organization.md),
[HTML architecture chapter](../docs/book-ch03-architecture-boundaries.html), and
[previous cleanup](architecture-disk-organization-20261028.md).
