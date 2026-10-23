# Architecture And Local Disk Organization

Date: October 23, 2026. Source metadata remains 3.6.0; base commit `0a65f2e8`
contains the separate CI repair after `daji 3.6.9`. The 58 existing uncommitted
source/configuration/evidence files are preserved. This pass pauses operator
iteration and changes documentation and regenerable local build output only.

## Architecture Review

The existing 13 logical topology modules and 15 Rust workspace crates remain
distinct views. No extra service, wrapper layer or merged desktop application
was introduced to reduce cache size. The organization audit found no tracked
debt and no source/documentation line-limit violation before these edits.

The compact architecture map now identifies one numerical-admission owner:
Solver. It separates coordinate/prescribed equations, projected directional
supports, Newmark increments and Newton corrections/nonlinear states. Engine
dispatch/workflow integration tests verify the consumer boundary; Agent, GUI
and SDK layers must not duplicate tolerance or hidden numerical-retry policy.
Capability allowlists, protocol admission, physical qualification and durable
recovery remain different contracts.

The source-placement guide now explains how new constrained solve paths join
the existing kernel tests, engine integrations, validation-profile and evidence
shards. It does not create another framework or imply automatic coverage of all
operators. Current maps own responsibilities; dated reports retain their results.

The HTML architecture chapter exposes the same numerical boundaries. Its old
calibration, modal experiments and product-recovery narratives are collapsed
into three labeled historical groups. Existing section IDs, report links and
failure/limitation text remain. Historical commit/count labels are no longer
presented as the current snapshot. No test or evidence source was deleted.

## Measured Local Cleanup

Repository-allocated KiB from `du -k -d 1 .`, before documentation edits:

| Location | Before KiB | After cleanup KiB |
| --- | ---: | ---: |
| Repository | 31,552,628 | 23,046,888 |
| `workers` | 25,980,308 | 17,474,568 |
| `target` | 4,504,672 | 4,504,672 |
| `sdks` | 304,196 | 304,196 |
| `apps` | 563,016 | 563,016 |
| `dist` | 53,168 | 53,168 |
| `tmp` | 46,976 | 46,976 |

The repository walk dropped from approximately 30.09 GiB to 21.98 GiB, a reduction
of 8.11 GiB. Rounded filesystem free space rose from about 29 GiB to 37 GiB.
APFS allocation, hard links and unrelated activity mean these readings are
snapshots, not an exact independent physical-byte guarantee.

The existing native `dev-disk --test-binaries` preview identified 26,923 entries
and 9.97 GiB of logical file lengths. Its `--apply` run removed the same scoped
count. The logical total is not claimed as the physical space freed. A subsequent
preview reports zero candidates and 0.00 GiB. No backup copy was made.

Removed categories are debug incremental caches, residual `*.rcgu.o` objects
and hashed Cargo executables with dep-info companions. No frontend cache existed
in this inventory. Current shared desktop packaging cache, dependency libraries,
top-level native tools and exported/runtime payloads were kept. Subsequent tests
can relink their executables; package caches were not emptied just to make the
headline smaller.

## Safety And Scope

The pre-cleanup process snapshot showed no active Kyuubiki/Cargo/Rust test or
build. The native cleaner also held Cargo output locks; its existing policy
refuses tracked outputs and symlinked cache roots. No process was stopped.

The 58 pre-existing changed/untracked files had identical SHA-256 digests and
identical Git status immediately before and after deletion. Credentials,
environment files, runtime journals/databases, research inputs, reports,
regression fixtures, lockfiles and installed Apps were not cleaned. No remote
server or global Cargo/npm/Docker storage was modified. Nothing was staged,
committed or version-bumped.

This pass does not rerun numerical qualification or rebuild the desktop Apps.
The earlier [Newton](newton-original-equation-admission-20261023.md) and
[Newmark](newmark-original-equation-admission-20261023.md) reports retain their
own test scope; those results are not counted as executions of this cleanup.

## Verification

The retained native maintenance binary, without a fresh Rust build, passes:

- Project organization audit and its self-test: source/configuration 800 lines,
  documentation 2,000 lines, tracked debt zero, including untracked sources.
- Module topology, function matrix and extension standard, including their
  self-tests; the matrix still has 13 logical modules and 11 paradigms.
- Modular Make entrypoint check: eight included modules.
- Coverage-tensor structural/command checks: zero structural gaps, two maturity
  gaps, 23 evidence-grade gaps and 18 P0 gaps. Readiness remains `blocked`; this
  cleanup adds no numerical or product qualification claim.
- Centralized HTML book and documentation inventory checks, plus 35 relative
  Markdown file links in the changed map/guide/report.
- Original HTML IDs and hrefs retained. Headless Chromium at 1280x900 and 390x844
  verifies three initially closed historical groups, pointer and keyboard
  toggles, visible content after expansion, and no horizontal document overflow.
  Navigation to the existing modal fragment opens its ancestor and reveals it.
- Whitespace checks and a post-cleanup zero-candidate native cache preview.

The browser initially hit the macOS process sandbox; the final local-file-only
verification passed with approval. No backend or network server was started and
no browser dependency was installed. This is book-page verification, not an
installed WebView or full GUI acceptance campaign.

The umbrella `make architecture-check` also runs runtime/package tests, live
protocol exercises and dependency network audits. That full campaign was not
run for documentation/cache-only changes, and deleted test binaries were not
relinked solely to re-attest unchanged runtime behavior. Existing executable
tools were retained and used; no new desktop/runtime package is claimed.

See the [current architecture map](../docs/current-architecture-map.md),
[source and disk policy](../docs/project-architecture-organization.md) and
[HTML book chapter](../docs/book-ch03-architecture-boundaries.html).
