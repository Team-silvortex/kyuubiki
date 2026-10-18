# Architecture And Local Disk Organization

Date: October 18, 2026. Source metadata 3.6.0, base commit `3a5ef43f`
(`daji 3.6.5`). Existing uncommitted Agent network/preflight work is preserved.
This pass changes documentation and removes regenerable local build artifacts;
it does not change numerical/runtime behavior, versions or installed Apps.

## Findings And Organization

- The compact architecture map had grown to 433 lines, with repeated historical
  Solver experiments and old tensor counts mixed into current ownership. It now
  is now 194 lines focused on product roles, crate boundaries, execution/recovery flow and storage
  ownership. Original reports, tests, current-line history and tensor claims stay.
- The source-placement guide incorrectly called TaskIR `execute` reserved and
  implied all execution still needed external package attachment. Current native
  built-ins, capability-admitted Engine solver dispatch and external package
  execution are now distinguished with source links.
- Headless clients, the Rust-only Worker / Operator SDK and frontend PWDT have
  separate ownership. Independent desktop shells still share lifecycle libraries
  instead of being merged to save build-cache space.
- The map distinguishes 13 logical topology modules from 15 Rust workspace
  crates. The topology's acyclic ownership graph is not presented as the full
  Cargo dependency or deployed-process graph.
- Documentation no longer labels all ignored `tmp/` content disposable. Databases,
  execution receipts, journals, credentials and process records need owner-aware
  handling. The repository guide no longer links to an ignored temporary README
  as if it were a permanent documentation source.
- The HTML architecture chapter now exposes the same responsibility/storage
  rules. Documentation ownership identifies where navigation, policies and dated
  evidence belong, avoiding further duplicated experiment logs in overview pages.

## Disk Measurements

`du -k -d 1 .` measures allocated KiB for the repository walk; values are snapshots,
not a permanent quota or independent physical-byte guarantee on APFS.

| Location | Before KiB | After cleanup KiB |
| --- | ---: | ---: |
| Repository | 41,423,112 | 22,523,584 |
| `workers` | 35,037,612 | 16,396,328 |
| `sdks` | 544,292 | 304,196 |
| `target` | 4,522,820 | 4,504,672 |

The repository walk dropped from approximately 39.50 GiB to 21.48 GiB, a decrease
of 18.02 GiB. Filesystem free space reported by `df -h .` rose from approximately
14 GiB to 32 GiB during cleanup. Other host activity and filesystem accounting
can affect those rounded readings.

The existing native `dev-disk --test-binaries` preview found 51,005 removable
entries and 22.33 GiB of logical file lengths. Its `--apply` run removed that
same scoped count. The larger logical estimate is not claimed as physical space
freed: hard-linked Cargo outputs can overlap, and top-level tools are retained.

Removed categories were debug incremental caches, residual `*.rcgu.o` objects
and hashed Cargo executables with dep-info companions. The native tool also
supports frontend `.next/cache`; there was no significant cache there in this
inventory. The runtime incremental directory alone occupied about 10.85 GiB.

## Safety And Retention

Process inspection identified the active Cargo build as belonging to another
project. No Kyuubiki build/test/runtime process was found in that snapshot;
the native cleaner additionally acquired Cargo output locks before deletion.
Its existing safety policy refuses tracked-output paths and symlinked cache roots.
No processes were stopped and no other project paths were cleaned.

Retained dependency libraries, current shared desktop release cache, top-level
native tools, packaged payloads, SDK sources, regression fixtures, lockfiles,
original failure reports, qualification evidence, runtime data and credentials.
No global Cargo/npm cache, Docker storage, server, installed runtime or App was
modified. No backup copy of disposable output was made.

The next tests may relink deleted executables; this is the intended cleanup
tradeoff. The test profile already disables incremental output. For one-off dev
checks under disk pressure, `CARGO_INCREMENTAL=0` can be set for that command
without imposing a new default or creating another target directory.

## Verification Scope

The existing native tools were used directly, without rebuilding binaries just
to verify documentation. The following checks passed:

- Project organization audit: 800 source / 2000 documentation line limits,
  tracked debt zero.
- Module topology, module function matrix and module extension standard.
- Coverage tensor: zero structural gaps, two maturity gaps, 22 evidence-grade
  gaps and 17 P0 gaps. Release readiness remains blocked; this pass adds no
  evidence claim and closes no product qualification requirement.
- Modular Make entrypoints: eight included modules.
- HTML book and documentation inventory; relative Markdown file links in the
  changed architecture guides and this report.
- `git diff --check`; native `dev-disk --test-binaries` post-cleanup preview:
  zero candidates, 0.00 GiB logical bytes.

This is not a new solver, installed-GUI, multi-host Mesh or power-loss
qualification campaign. The full `make architecture-check` includes dependency
network audits and runtime tests; it was not rerun for these documentation-only
source changes. Existing executable artifacts were retained and used, but this
does not attest that every binary embeds the latest uncommitted source overlay.

See [the compact architecture map](../docs/current-architecture-map.md),
[storage policy](../docs/project-architecture-organization.md#generated-disk-use)
and [HTML book entry](../docs/book-ch03-architecture-boundaries.html#source-ownership).
