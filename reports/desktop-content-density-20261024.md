# Desktop Content Density

Date: 2026-10-24

Runtime summaries and informational cards now size to their content instead
of stretching short content to match a long neighbor. The existing theme,
action IDs, data contracts and runtime services are unchanged. This source
change does not rebuild or reinstall the desktop applications.

## Layout Changes

The shared runtime plane groups its summary cards separately from mesh,
topology, filters and selected detail. The summary and mesh grids top-align
their cards, wrap long descriptions and allow narrow columns to shrink safely.
No description is removed or clamped to achieve the smaller height.

Workbench control uses one primary console and an independent auxiliary stack.
The runtime metadata and viewer controls no longer reserve equal-height rows
against the spanning console. Workflow hints lose their oversized minimum
height, short status output is compact, and logs keep their own readable
minimum height. The language selector has a bounded width. Localized panel
titles bind to semantic panel classes rather than positional selectors, so
reordering panels does not mislabel them.

Hub overview and mainline cards, Installer service/diagnostic/integrity/update
summaries, and frontend runtime and metric grids use content-aligned layout.
Explicit editor, visualization, fullscreen and immersive-modeling height rules
are not removed. Generated shared assets are synchronized from the canonical
TypeScript/CSS sources into all three desktop apps.

## Browser Controls

Before repair, all three new test groups fail. One fixture stretches a short
runtime card to 1,071.75 CSS pixels because of a long neighboring description.
A Workbench workflow-hint height check and a Hub informational-card control
also fail before the changes.

The dedicated suite exercises the shared status plane inside Hub, Installer
and Workbench at 1,440, 1,080, 768 and 390 CSS-pixel widths. Short runtime cards
must remain at most 180 pixels high and short mesh panels at most 200. The
long Chinese/German descriptions remain present and wrap inside the plane;
details occupy their own row after summaries.

Workbench control additionally checks metadata height, auxiliary stack gaps,
workflow-hint height, horizontal overflow, logs/status switching, four stopped
runtime entries, mouse and keyboard detail selection, and correct English and
Chinese panel titles. Isolated real-CSS fixtures check six informational grid
families, including the frontend runtime overview grid. Those isolated controls
are not a full frontend-page workflow test.

## Reproduction

The final-source Chromium fixture run passes 11 tests across the density,
workspace-priority and three-shell regression suites. Eight shell/bridge smoke
tests and 17 native shared-resource/build-helper tests also pass. Canonical
shared asset verification, documentation inventory, project organization,
Rust formatting and diff whitespace checks pass. The local UI claim is recorded
in the coverage tensor; its two maturity, 23 evidence-grade and 18 P0 gaps
remain unchanged, and overall daji qualification remains blocked.

The normal stopped-runtime Workbench control preview is captured after finite
card animations complete. It is a controlled browser fixture, not a screenshot
of a newly installed desktop build. No installed-application files changed.

```text
node --test tests/integration/desktop-gui-content-density.test.mjs
node --test --test-concurrency=1 tests/integration/workbench-shell-regression.test.mjs tests/integration/desktop-shell-regression.test.mjs tests/integration/desktop-gui-layout-priority.test.mjs
```

The density suite is registered in both the desktop validation contract and
the native desktop integration-test list. The standalone frontend layout
audit initially reports an environment-restricted browser-launch skip; that
skip is not counted as a passed full-page audit. The browser tests above run
with permission to launch an isolated browser and local fixture previews.

This is bounded browser-fixture content-density and interaction coverage, not full frontend, installed WebView, all-language or all-platform qualification.
