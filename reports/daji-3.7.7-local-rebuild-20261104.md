# Daji 3.7.7 Progress And Local Installation

Date: 2026-11-04

Version-named Git checkpoint `f28de0dd` already says `daji 3.7.7`, while the
package/book and installed baseline remained 3.7.1. This alignment uses base
`3fe22542` plus the existing working-tree overlay. No additional patch number,
commit, push, public upload or SDK registry publication is implied.

## Project Progress

README, the HTML book/status chapter, Hub document shelf, version ledger,
weakness roadmap, package/SDK metadata, language targets, update catalog and
tensor checkpoint now agree on 3.7.7. Historical reports retain their versions.

The current summary distinguishes these workstreams:

- Engine/Solver: checked numerical ranges, original equations and bounded results;
  this rebuild does not rerun or newly qualify the complete physics catalog.
- Workflow recovery: paired job/runtime commits, stale-evidence fencing and
  durable finalization; process termination is not physical power-loss evidence.
- Agent access: native TLS/token transport, Rust SDK configuration and Workbench
  session credentials; authenticated multi-host and cross-language acceptance
  remain separate requirements.
- Store/PWDT: workspace catalog UI and exact recipe source/version matching;
  production account, subscription and publishing integration remains open.
- Model API: model discovery/manual ID, bounded native gateway, reviewed plans,
  endpoint-change fencing and memory-only keys. New copy is EN/ZH/JA with fallback
  elsewhere, not a claim of complete translation into every supported locale.
- Distribution: three independent macOS arm64 shells plus native runtime,
  with installation results separated from source tests and remote CI.

The tensor readback retains 13 modules, 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall qualification
remains blocked; metadata synchronization does not promote evidence grades.

## Runtime Installation

The native builder rebuilt the release Agent/Headless/runtime binaries,
the self-contained Orchestra release and the production static frontend.
An initial Elixir build was stopped by the local communication permission
boundary; the approved retry completed without changing the code to bypass it.

Installer sealed and activated runtime 3.7.7. Its 1,741-file manifest SHA-256 is
`8523db733a90747c84035ec76bb97a8a89cd5966ab54a6b6e8791c452b5df886`;
the installed manifest matches. The old services had zero active/queued work
and were stopped through the native lifecycle controller before activation.
Version 3.7.1 remains the managed previous activation. Existing state is not
copied, migrated or deleted, and no extra archive or DMG is produced.

New frontend, Orchestra and both local Agents start successfully. Health reports
two ready endpoints, zero active jobs and a healthy watchdog. The served brand
is 3.7.7. Model gateway capabilities return HTTP 200 and advertise no credential
storage or automatic retry. The installed store returns 212 built-in entries:
140 operators, 71 workflow templates and one frontend DSL template.
Those are catalog entries, not 212 newly qualified executable scenarios.

The deployment remains loopback-only. TLS/token support is included without
opening remote access or creating persistent credentials. No real API key is
copied into the repository, package, logs or installed preferences.

## Verification Scope

The three independent app-only builds completed in 351.36 seconds using the
shared Cargo cache and at most two build jobs per build. Hub took 162.42 seconds,
Installer 129.42 and Workbench 58.24, plus setup/staging. This is one build
observation, not a controlled compilation benchmark.

The native host installer replaced the standard Applications bundles. All three
Info.plist versions are 3.7.7, strict recursive local signature checks pass,
and installed executable hashes match the build outputs. Hashes are retained
in the release snapshot. All three actual installed shells return valid
interactive startup receipts: three pass, zero fail. The portable report is
[retained here](../releases/usability-evidence/3.7.7/macos-installed-desktop-smoke.json).

The installed Workbench was opened normally. Both native shell and embedded UI
show 3.7.7, with Orchestra connected. Its model connection panel has the new
URL/key/trust/model controls and an empty key field. The real store refresh
returns 212 entries; expanding the workspace, filtering to the DSL template
and opening source/version/package-reference details work. System configuration
opens the Agent TLS/token panel with certificate and session-token fields.
No real key was entered, package added or research task dispatched in this
installed UI observation. Workbench is left open on the security panel.

Fresh browser regressions pass seven tests with zero failures/skips: Agent
configuration at 1440/390px, model connection at both widths, exact PWDT
store selection and store browse/filter/detail/manifest at both widths.
They use isolated controlled services and synthetic secrets, not a production
store backend or the installed WebView.

Fresh frontend unit tests pass 1,982 cases with zero failures/skips. Fresh
native-runtime tests pass 150 cases with zero failures and three ignored opt-in
tests; the real-provider test remains opt-in. These module suites are not the
complete repository test suite. Fresh production frontend build/type checks pass.
Browser storage-security checks also pass. Version alignment passes
230 exact checks; six locked/offline Rust manifest checks pass. Shared assets,
60 language-pack contracts, HTML book/inventory and organization audits pass.
Source/doc limits remain 800/2000 lines with no tracked debt. Language-key
counts (Workbench 960/960, Hub 510/510) are not linguistic quality review.

Earlier same-day source verification retains its own scope: 1,982 frontend
unit passes, 150 native runtime passes with three ignored tests, and a separately
opted-in real OpenAI gateway request. The live check used a synthetic prompt,
accepted a bounded plan with no actions, and used 67 input/62 output tokens.
These overlapping checks are not additive or a new scientific qualification.

Desktop packaging and installed-WebView results are also recorded in the
[current snapshot](../releases/snapshots/3.7.7.json). No Linux/Windows build,
complete numerical rerun, remote Mesh fault campaign, physical power-loss test,
data migration, public distribution or notarization is claimed here.
