# Daji 3.4.0 Local Rebuild and CI Repair

Date: September 27, 2026. Host: macOS/aarch64. Base source revision:
`b7eadf93007aeeee59f43d603b1094389341d2b0`, plus the current working-tree
changes. This is not evidence of a clean committed build or public publication.

## Failure Evidence and Repairs

The failed GitHub CI baseline is run `36091493644` on `main`.

- The diagnostic-bundle fixture passed in isolation but failed when workspace
  feature unification enabled JSON insertion ordering. Source identifiers now
  have an explicit sorted contract independent of map representation.
- The live Headless workflow test asserted an obsolete per-node message after
  the job had reached its terminal state. It now checks the current terminal
  receipt while retaining result and completion assertions.
- The metric-contract scanner did not include the extracted alias module and
  assumed pre-refactor quality-scorer source layouts. Alias alternatives are
  scanned, and quality metadata/output shapes are exercised through the actual
  built-in Worker registry. Existing negative controls remain active.
- Sixteen PWDT failures in that CI run shared a Chromium/libdbus startup crash
  before any page opened. The browser harness permits one retry for that exact
  launch signature and preserves both failures. Application assertions and
  explicit browser executable overrides are not retried or substituted.
- Local whole-workspace testing exposed a live operator-task fixture sharing
  the default SQLite database. It now owns an isolated temporary directory,
  preserves startup logs, and joins/kills its owned child before removing data.
- A later Agent live-test run reported a missing file. While investigating,
  directory allocation was found to depend solely on concurrent clock samples.
  A monotonic sequence and exclusive directory creation now prevent aliasing;
  equal-timestamp allocation has a regression test. This addresses the unsafe
  allocation mechanism without claiming a captured filesystem race trace.

No additional test was skipped, no assertion threshold was relaxed, and no CI job was
disabled to obtain these results. Existing ignored/excluded qualification
tests retain their original requirements.

## Local Validation

| Check | Result | Scope |
| --- | --- | --- |
| Rust workspace, `cargo test --locked --no-fail-fast` | 3283 passed, 0 failed, 33 ignored | All default workspace test targets |
| Engine without workspace feature unification | 635 passed, 0 failed, 1 ignored | `kyuubiki-engine --lib` |
| Elixir, `mix test` | 873 tests, 0 failures, 8 skipped | Isolated SQLite database |
| GUI workflow suite | 256 passed, 0 failed | Real browser, with mock backends where the fixture declares them |
| Browser launch unit controls | 4 passed | Includes explicit executable handling added after the aggregate suite started |
| Metric scanner regressions | 2 passed | Repository contract and negative controls |
| Agent lifecycle, orphaning, cancellation, shutdown, and operator-task follow-up | 76 passed | After test-only lint cleanup |
| Exact version contracts | 224 checks, 0 mismatches | Current product/package metadata, not historical evidence |
| Lockfile and shared-asset consistency | Passed | Four npm locks differ only in application versions; frontend `npm ci --dry-run` passed; shared desktop assets match |
| Book, language packs, PWDT contracts, component integrity | Passed | Structural/contract checks, not complete translation certification |
| Operator reliability contract | Passed | Historical qualification retains its original scope |
| Organization audit | Passed | Source 800 lines, docs 2000 lines, zero tracked debt |

The aggregate GUI run includes the first three browser-launch unit tests. The
four-test standalone result is an additional targeted run, not four disjoint
tests to add to the aggregate count. Installed Tauri startup is tested separately
below; browser mock coverage is not relabeled as installed service qualification.

## Lint Follow-up

The six remaining `needless_borrow` findings were corrected: five in
`workers/rust/crates/desktop-runtime/src/runtime_control.rs` and one in
`workers/rust/crates/cli/src/agent_fault_injection.rs`. Expanding the check to
the desktop runtime's own test and example targets found two further idiom
warnings, also corrected: `ok_expect` in the test listener and
`nonminimal_bool` in the checkpoint-response fault example. No lint was
suppressed or downgraded.

The following strict check passed from `workers/rust`:

```text
cargo clippy --locked -p kyuubiki-cli -p kyuubiki-script-runner \
  -p kyuubiki-engine -p kyuubiki-desktop-runtime --all-targets -- -D warnings
```

The follow-up desktop-runtime all-targets regression passed 79 tests, with
0 failures and 1 existing ignored test. CLI binary tests plus the Agent
lifecycle, orphaning, shutdown, and solver-cancellation live suites passed
226 tests with 0 failures and 0 ignored tests. These are separate follow-up
runs, not disjoint additions to the earlier aggregate totals.

These eight source-only lint repairs do not rebuild or replace the already
verified immutable 3.4.0 runtime payload. The installed-package results below
remain evidence for the preceding build, not for a newly installed binary.

## Installed Package Validation

- Hub, Installer, and Workbench were built independently as 3.4.0 `.app` bundles,
  verified with strict local code-signature checks, and installed into the
  standard macOS Applications directory. No combined shell or extra DMG archive
  was produced.
- All three installed applications emitted accepted interactive boot receipts
  for 3.4.0. See
  [native boot evidence](../releases/usability-evidence/3.4.0/macos-installed-desktop-smoke.json).
- The sealed 3.4.0 payload contains native agents, a native static frontend
  server, and a self-contained Orchestra release. The installed services use
  this payload rather than a source-tree or Node runtime fallback.
- Before activation, the old runtime was confirmed not running. A SQLite `VACUUM INTO` copy
  transferred the existing database without overwriting a destination file.
  Integrity passed and all 2 projects, 12 models, 17 jobs, and 17 result records
  were preserved. This bounded local transfer is not a general migration claim.
- Installed Headless `direct_bar_1d` ran with the real service executor and
  research posture. Inputs were force 1200 N, length 1 m, area 0.01 m2, and
  Young's modulus 210 GPa. Tip displacement was `5.714285714285714e-7` m,
  maximum stress `120000` Pa, and support reaction `-1200` N.
- Assertions compared displacement with `F*L/(E*A)` (absolute tolerance `1e-15`),
  stress with `F/A` (`1e-8`), and reaction with `-F` (`1e-8`). The installed
  runtime was restarted and the fetched result matched the original field for
  field, without another solve.

## Remaining Boundaries

This report records local verification before the release commit and push;
GitHub CI for that commit must be checked separately. Linux's original libdbus crash was not reproduced on
this macOS host; remote CI remains the confirmation gate for that environment.
Windows and Linux package metadata were aligned but native packages were not
rebuilt here. Signing is local ad-hoc signing, not notarization. No SDK registry
or download server publication was performed. This release adds no blanket
numerical, million-node, or all-platform qualification claim.
