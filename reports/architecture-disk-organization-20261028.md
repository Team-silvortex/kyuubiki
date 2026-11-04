# Architecture And Local Disk Organization

Date: October 28, 2026. This pass pauses reliability implementation for source
ownership and disk maintenance. Existing release declarations are unchanged;
this is not a new release or installed acceptance run.

## Architecture Review

The 13 logical topology modules, 15 Rust workspace crates and deployed processes
remain different views. Hub, Workbench and Installer stay independent Apps;
Agent owns execution identity, Engine owns dispatch and Solver owns numerical
algorithms. Headless SDKs, the Rust Worker / Operator SDK and frontend PWDT remain
separate extension/control surfaces. No new module, wrapper or service is added.

The current architecture map now groups the existing workflow recovery owners:
coordinator routing, lease ownership, runner monitoring, scan/reconciliation,
replay admission, completion publication, failure/cancellation and paired storage
commits. The source placement guide adds corresponding test-placement rules.
These document the present implementation, including uncommitted recovery fixes;
they do not turn a helper into a new top-level topology module.

The harmonic arithmetic narrative had again grown into an experiment log in both
the compact map and HTML book chapter. It is now a responsibility/source/evidence
index in the map and a short boundary summary in HTML. The existing source and
report links are retained. Experiments, limits, failures and run totals stay in
their original reports; no evidence or regression test is deleted.

The contribution guide is corrected to match `apps/web/config/runtime.exs`:
SQLite is the default, tests use isolated SQLite unless an explicit database
path is supplied, and memory/JSON is opt-in. The native launcher example no
longer invokes it through `zsh`.

## Measured Cleanup

Repository-allocated KiB from the same `du -x -k -d 1 .` walk, measured before
cleanup and immediately afterwards, before documentation edits:

| Location | Before KiB | After KiB |
| --- | ---: | ---: |
| Repository | 39,268,492 | 26,201,764 |
| `workers` | 32,247,784 | 19,783,956 |
| `sdks` | 691,736 | 374,440 |
| `apps` | 852,448 | 566,844 |
| `target` | 5,234,536 | 5,234,536 |
| `dist` | 56,412 | 56,412 |
| `tmp` | 83,844 | 83,844 |

The repository walk decreased from 37.45 GiB to 24.99 GiB, a 12.46 GiB reduction.
The native `dev-disk --test-binaries` preview and apply each reported 17,412
entries and 13.35 GiB of logical file lengths. Logical lengths are not physical
space recovered because hard links can overlap. The post-cleanup preview reports
zero candidates and 0.00 GiB.

Filesystem available space was 5,560,000 KiB before and 42,800,000 KiB after the
cleanup. That filesystem-wide increase is larger than the project reduction;
APFS accounting or unrelated activity may contribute. It is not attributed
entirely to this operation. No snapshot purge or global cache cleanup was run.

Removed categories are debug incremental output, residual Rust object files,
hashed Cargo executables with dep-info companions, and frontend build cache.
The native maintenance executable was reused without rebuilding it. The cleaner
held Cargo output locks and checked tracked-file and symlink protections.
No backup copy was made.

Most remaining space is reusable build output: runtime debug dependencies occupy
about 15.98 GiB, shared desktop cache 4.99 GiB, and runtime release output 1.91 GiB.
These are directory allocation observations, not proof of independent physical
bytes. Dependency libraries, current top-level tools, frontend export, release
products and the Elixir/UI dependencies remain, avoiding a forced cold rebuild.
Future tests can relink their removed executables and grow debug output again.

## Preservation And Scope

The process check found installed Agent, Orchestra, frontend runtime and Installer
processes, but no active project Cargo/compiler/test process or frontend dev
server. Installed processes were left running. No installed App/runtime, server,
global Cargo/npm cache or Docker storage was modified.

All 10 pre-existing changed/untracked files had identical SHA-256 digests before
and after cache deletion. This includes the coordinator, scan, terminal/legacy
tests and their current documentation/evidence. Those files remain untouched by
this organization pass. Credentials, local environment files, live databases,
journals, receipts, research inputs, lockfiles and retained reports are preserved.
The mixed `tmp` tree is deliberately retained rather than blanket-pruned for a
small cosmetic saving. Nothing was staged, committed, pushed or version-bumped.

## Verification

The retained native maintenance tool passes organization audit plus self-test,
module topology, function matrix and extension-standard checks plus self-tests,
modular Make checks (eight included modules), and tensor checks plus self-test.
The tensor still has zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps. Readiness remains `blocked`; cleanup is not qualification.

Documentation book/inventory checks pass, as do 114 relative file/fragment links
across the five changed documents. All 49 original map links, 37 original HTML
links and existing HTML IDs are retained. The map is reduced from 311 to 271
lines and the HTML chapter from 510 to 494, despite adding recovery ownership.
Chromium at 1280x900 and 390x844 verifies no horizontal overflow, unique IDs,
the recovery fragment, and pointer/keyboard toggles for all three historical
groups. Screenshots were inspected and removed with their owned temporary
directory. The browser required permission to start on macOS; it loaded only
local documentation, with page HTTP requests blocked and no new dependency.

This pass changes documentation and regenerable output only; it does not rerun
numerical qualification, rebuild Apps or claim a new full runtime test result.
The preceding recovery pass retains its own
[4,083-case Web regression evidence](workflow-terminal-diagnostics-20261028.md#legacy-recovery-commit-follow-up).

The umbrella `make architecture-check` also triggers runtime/package campaigns,
live protocol exercises and dependency audits. That complete campaign is not run
merely to relink the binaries just removed. The directly executed bounded
architecture checks above are the scope of this maintenance pass.

See the [current map](../docs/current-architecture-map.md),
[source and disk policy](../docs/project-architecture-organization.md), and
[HTML architecture chapter](../docs/book-ch03-architecture-boundaries.html).
