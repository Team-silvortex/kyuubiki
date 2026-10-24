# Daji 3.7.1 Local Rebuild

Date: 2026-10-24

The existing Git checkpoint `c4cc2f96` is labeled `daji 3.7.1`, while package
metadata previously said 3.7.0 and the local installed applications/runtime
were 3.4.0. Current first-party metadata is reconciled to 3.7.1. This build
includes the uncommitted numerical, compact-layout and version-alignment
overlay; it is not a clean-commit artifact. No commit or push is performed.

## Build and Installation

The native runtime payload builder rebuilds the release CLI, Headless and
runtime controller, the Orchestra release and the static frontend, then seals
the macOS payload. Its 1,736-file manifest has SHA-256
`60bad8d56315ced19cb72bb1418e5699b84328ae431e74974c62a4970c840bcb`.
The installed manifest matches this digest.

Installer activates runtime 3.7.1, retaining 3.4.0 as the previous activation.
The versioned frontend, Orchestra and both local Agents start successfully.
The runtime controller reports all four running; the Orchestra health endpoint
reports `ok`, two ready endpoints and a healthy watchdog. The served frontend
brand metadata reports 3.7.1. The frontend is static and served natively,
not through a Node runtime server.

The three independent macOS arm64 shells are built with a shared Cargo target
cache and at most two build jobs. The app-only build takes 506.47 seconds:
Hub 289.73, Installer 153.89, Workbench 52.00, plus shared setup and staging.
No DMG or extra installed-app backup copy is retained.

The normal host installer replaces the three applications in the standard
Applications directory after the old Workbench exits normally. All three
installed Info.plist versions are 3.7.1. Each executable matches its build
output; exact SHA-256 values are retained in the
[release snapshot](../releases/snapshots/3.7.1.json). Strict recursive local
signature verification passes for all three ad-hoc-signed apps. This is not
Developer ID notarization or public distribution.

Existing runtime versions and state are not deleted or migrated. The current
acceptance concerns the new version's startup, not existing-project migration,
data continuity, rollback execution or crash recovery.

## Installed and Source Checks

All three installed applications return valid interactive startup receipts:
three pass, zero fail. The portable machine-readable report is retained in
[installed desktop smoke evidence](../releases/usability-evidence/3.7.1/macos-installed-desktop-smoke.json).

The actual installed Workbench is then opened normally. Its native shell and
embedded frontend both show 3.7.1. The embedded workspace connects to Orchestra
instead of remaining blank. Returning to Control shows four compact running
service cards in one row, with the selected detail below rather than as a tall
neighbor. Runtime metadata and viewer controls stack by content height. The
status/log buttons show native frontend logs, and the keyboard shortcut returns
to the status overview. The application is left open on that overview.

This is a bounded installed-WebView observation, not a comprehensive interaction,
all-language, all-resolution, modeling or workflow qualification.

The fresh density fixture suite passes three tests, with no skips. It covers
the three shells at multiple widths, long Chinese/German descriptions,
Workbench status/log and detail interactions, and informational grid alignment.
These browser fixtures remain distinct from installed-WebView observations.
Earlier broader source checks retain the scope in the
[density report](desktop-content-density-20261024.md); their counts are not
added to this run.

Version audit: 225 exact checks, zero mismatches. Locked offline manifest
metadata checks pass for the Rust workspace, Rust SDK, three desktop shells
and operator template. Desktop shared resources, 60 language-pack contracts,
documentation book/inventory and project organization checks pass. Language
key coverage is not independent translation review. The native build refreshes
Workbench's missing first-party protocol lock entry; third-party versions are
not intentionally upgraded.

The tensor remains structurally valid with 13 modules and 11 paradigms, two
maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall qualification
remains blocked. No Linux/Windows build, full numerical rerun, SDK registry
publication, remote deployment or public release is claimed by this rebuild.

## Reproduction

```text
kyuubiki-script-runner desktop-runtime-payload macos
kyuubiki-script-runner desktop-build-host --bundles app
kyuubiki-installer install-runtime-payload dist/macos
kyuubiki-runtime start-local
kyuubiki-script-runner desktop-install-host
kyuubiki-script-runner desktop-packaged-smoke macos --bundle-root <applications-root> --out tmp/repack-371-installed-smoke.json
kyuubiki-script-runner desktop-packaged-smoke macos --retain-report tmp/repack-371-installed-smoke.json
node --test tests/integration/desktop-gui-content-density.test.mjs
```

Use the current native tools with the correct local filesystem and loopback
access; a restricted network check can misleadingly report healthy services as
not ready. The immutable installed runtime version is not overwritten in place.
