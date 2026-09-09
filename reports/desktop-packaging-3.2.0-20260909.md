# Daji 3.2.0 Desktop Rebuild

Date: 2026-09-09. Platform: macOS ARM64. Source baseline:
`388e061e48c0a7b4d63366471c8095cf11b86c8e`, plus this rebuild's version metadata
and documentation alignment. No Git commit, push, or registry publication was
performed by the rebuild.

## Installed Result

Hub, Installer, and Workbench were rebuilt as separate `3.2.0` app bundles and
installed using the native `desktop-install-host` command. All installed plist
versions and build numbers are `3.2.0`; all three passed strict recursive
signature-integrity checks. These are ad-hoc-signed local builds, not notarized
public-distribution packages. Hub was reopened and its actual WebView title and
visible version badge were checked: `Kyuubiki Hub` / `daji 3.2.0`.

The installer-managed `3.2.0` runtime was rebuilt, sealed over 1,646 files,
verified, installed, and activated. It includes the native Rust CLI/Agents,
headless binary, native runtime/frontend server, self-contained Orchestra
release, and production static frontend. Node is used by the UI build toolchain,
not by the installed runtime services.

## Verification

| Check | Result |
| --- | --- |
| Exact version contracts | 224 checked; 0 mismatches |
| Language-pack metadata/validation | 60 packs; core coverage Workbench 960/960, Hub 510/510 |
| Documentation book | 26 HTML files plus manifest/bootstrap checked |
| Documentation inventory | Passed |
| Desktop shared assets | Passed |
| Project organization | Passed; source limit 800, document limit 2000; no tracked debt |
| Frontend production build | Static export and TypeScript validation passed |
| Installed desktop interactive startup | 3 passed; 0 failed |
| Installed runtime | Two Agents ready, Orchestra healthy, frontend serving `3.2.0` brand metadata |
| Native frontend API proxy | Health and project API requests reached installed Orchestra |
| Packaged persistence chain | Nine scoped checks passed; temporary projects deleted |

The persistence probe exercised project/model creation, initial-name
preservation, checkpoint metadata promotion, working edits, latest-checkpoint
deletion, reference repair, working-state preservation, bundle export, and
deleted-version 404 handling through the actual frontend HTTP proxy and SQLite
backend. It is not a numerical solver validation or a full GUI journey suite.

The first probe expected a model name to remain unchanged after explicitly
supplying new name metadata when saving a version. Inspection confirmed that
both backends promote supplied version metadata to the working model. The probe
was corrected to verify that existing behavior and the initial-name guarantee;
no production behavior was changed to make the check pass. Both temporary test
projects were removed, including the first probe's project.

Retained startup receipts:
[macOS installed desktop smoke](../releases/usability-evidence/3.2.0/macos-installed-desktop-smoke.json).
Release metadata: [3.2.0 snapshot](../releases/snapshots/3.2.0.json).

## Build And Storage

Desktop builds reused the shared macOS Cargo cache. Hub took 246.27 seconds,
Installer 235.92 seconds, and Workbench 80.37 seconds; total desktop orchestration
was 582.98 seconds. Runtime and frontend assembly preceded those desktop builds.

Only `.app` bundles were requested. No DMGs, source archives, or additional app
backup copies were created. The native installer removed its temporary previous
app copies after successful replacement; the desktop build staging area is
empty. Existing runtime rollback storage was not purged as part of this rebuild.

The old running `3.0.0` services belonged to an earlier temporary packaging test
state. After confirming no active or queued jobs, the native runtime controller
stopped them using that state root. The new services run under the standard
installer-managed, version-scoped state directory, without that temporary
override. Earlier test databases were not silently imported or deleted.

Rebuild entrypoints:

```text
kyuubiki-script-runner desktop-runtime-payload macos
kyuubiki-script-runner desktop-build-host --bundles app
kyuubiki-installer install-runtime-payload dist/macos
kyuubiki-script-runner desktop-install-host
kyuubiki-script-runner desktop-packaged-smoke macos --bundle-root /Applications
```

Linux and Windows package metadata was aligned, but their native binaries were
not rebuilt in this macOS installation task. Historical solver/research reports
remain scoped historical evidence, not a fresh full-system qualification.
