# Daji 3.7.8 Local Rebuild And Installation

Date: 2026-11-04.

The version-named Git checkpoint `fb93e316` already said `daji 3.7.8`, but
package metadata still targeted 3.7.7. This round aligns source metadata and
rebuilds from that commit plus the alignment overlay. It does not create a
new commit, push, tag, public download or SDK registry release.

## Contents And Version Alignment

The three independent desktop shells, Rust workspace, official SDK package
metadata, Orchestra release, language-pack targets, brand mirrors, update
contracts and current book now target 3.7.8. Historical evidence keeps its
original scope; the 3.7.7 snapshot is only marked archived.

The new Workbench includes grouped service/model configuration, connection
collapse after successful discovery, catalog search/manual ID modes and
collapsed advanced details. Stale model catalogs after failed checks and
language-switching notices were fixed before this rebuild. See the separate
[research and GUI report](model-research-boundary-and-connection-20261104.md)
for prior source regressions and the real 26-workflow synthetic research run.
Those earlier results are not rerun or counted as new physics qualification here.

## Build And Acceptance

The native builder produced the release Agent, Headless executable, runtime
controller, self-contained Orchestra and production static frontend. The
sealed payload contains 1,741 files; every listed hash and executable flag
was checked. Its manifest SHA-256 is
`851b5f325745044b8defe4638f6998bbc06464871fd6ff7254bf0a5b5e898f95`.
There is no packaged Node runtime or frontend node_modules tree.

Three app-only macOS arm64 bundles were built through the shared Cargo cache.
Hub took 58.84 seconds, Installer 57.44 and Workbench 24.69; total wall time,
including preparation/staging, was 146.24 seconds. These are observations,
not a controlled build-performance benchmark. No DMG or full backup archive
was created. The resulting app bundles occupy approximately 17/17/16 MiB,
and the staged native runtime occupies 58 MiB.

All three source bundles and all three installed Applications copies passed
interactive startup probes. Installed versions are 3.7.8, strict recursive
local signature checks pass, and installed executable hashes match the build.
The [installed startup report](../releases/usability-evidence/3.7.8/macos-installed-desktop-smoke.json)
and [snapshot](../releases/snapshots/3.7.8.json) retain the evidence.
Bundles are ad-hoc signed, not notarized for public distribution.

The actual production export was served by the packaged native runtime on an
isolated loopback port. Browser checks passed at 1440x1000 and 390x844: 3.7.8
brand, PWDT language switching, two grouped model sections, empty password
input, responsive layout and no page errors. No real key or inference was
used. The temporary server and browser were stopped after testing. This is
a production-browser check, not a claim of full installed-WebView interaction
coverage beyond the separate native startup probes.

Fresh metadata checks passed: 230 exact version contracts, 60 language packs,
shared desktop assets, macOS stage/icons, HTML book and documentation inventory,
project organization and whitespace validation. The production frontend also
passed compilation and type validation. Full solver/platform suites were not
rerun for this packaging-only round.

## Installation And Retention

The user approved replacing the three Applications bundles and switching the
local runtime. Health checks showed zero active/queued jobs before stopping
the owned 3.7.7 processes through their native lifecycle controller.

Installer activated 3.7.8 with 3.7.7 as the previous managed version. The
installed manifest matches the sealed payload. Existing versioned state was
left in place, not copied, migrated or deleted. No extra persistent API key
or remote access was configured.

The new frontend, Orchestra and two Agents start normally. Health reports
two ready endpoints, zero active/queued jobs and a healthy watchdog. The
normal frontend port serves brand 3.7.8, and model gateway capabilities return
HTTP 200 with no credential storage or automatic retries. Workbench was opened
from Applications after installation.

Logs remain under ignored `tmp/daji-3.7.8-*`; no large test-result archive was
added to Git. This round does not qualify data migration, rollback execution,
power loss, all material solvers, remote Mesh, Linux or Windows.
