# CI Browser Preparation Budget

Reviewed October 28, 2026. Scope: CI workflow scheduling and its source-level
regression guards, not changes to GUI behavior, numerical solvers or cancellation.

## Observed Failure

[CI run 40319852892](https://github.com/Team-silvortex/kyuubiki/actions/runs/40319852892)
tested commit `471e50c5766af42a9021e0340766bedcfbcf7ab7`. The run was cancelled;
20 jobs succeeded, `desktop-gui-workflow-2` was cancelled and the required desktop
aggregate failed because not every suite had passed. The same commit's independent
[Windows desktop qualification](https://github.com/Team-silvortex/kyuubiki/actions/runs/40319852858)
passed.

GitHub's annotation for job `121563690206` explicitly reports that it exceeded
the maximum execution time of 30 minutes. The job log shows:

| Stage | UTC | Observation |
| --- | --- | --- |
| Job begins | 12:10:00 | Original whole-job deadline: 30 minutes |
| Playwright installation begins | 12:10:43 | System packages and browser download share one step |
| APT transfer finishes | 12:23:53 | 20.3 MB fetched in 13m 1s, reported 25.9 kB/s |
| Browser installation finishes | 12:24:06 | No browser-install failure |
| Regression execution begins | 12:24:50 | 14 of 42 files, shard 2/3 |
| Last completed assertion | 12:40:11 | Test 237 passed; no failing test assertion in the log |
| Task cancelled | 12:40:14 | Job deadline reached before suite completion |

This is a preparation/execution budget collision. A successful partial test log is
not a passing suite; the aggregate correctly refused to turn green. The slow mirror
is not evidence of a GUI deadlock or a numerical solver failure.

## Scoped Change

- Separate browser system dependency installation (15-minute ceiling) from Chromium
  installation (3-minute ceiling). Use the locked local Playwright CLI for both.
  System packages are still installed; no dependency check is skipped or ignored.
- Give desktop/mechanical/thermal tests an explicit 20-minute execution ceiling and
  workflow shards an explicit 30-minute ceiling. These were previously whole-job
  limits. Their outer job budgets become 40 and 50 minutes to add preparation
  headroom. This raises worst-case job time, not the normal duration of a test.
- Apply the same preparation split to workflow preflight, with a 15-minute test
  ceiling and a 40-minute job limit. These are bounded stages, not an unconditional
  reservation if unrelated setup or cleanup also consumes the outer job budget.
- Pin these browser jobs to the observed Ubuntu 24.04 baseline. The failing run
  also announced an upcoming `ubuntu-latest` image migration; adopting that image
  is separate from fixing this regression.
- Retain all six desktop suites, all three workflow shards, serial test execution,
  failure/cancellation artifacts, `fail-fast: false` and the strict aggregate gate.
  No `continue-on-error`, test retry, exclusion or assertion relaxation is added.

## Validation

Two new Rust source-contract tests failed before the workflow change and passed
after it. They guard the bounded preparation steps, explicit test ceilings,
preparation headroom, unchanged suite/shard mapping and fail-closed aggregation.
The source reader normalizes CRLF for Windows checkouts. These lightweight guards
are not a general YAML parser or proof of GitHub's scheduler behavior.

The workflow also passes YAML parsing and actionlint 1.7.12. The temporary validator
was downloaded from its official release and its archive SHA-256 matched the
published release checksum. This check does not execute Ubuntu package installation.

Final local results:

| Check | Result |
| --- | --- |
| Script-runner unit suite | 415 passed, zero failures |
| Complete workflow shard 2/3 (14 files) | 242 passed, zero failures, skips or cancellations; 691.7 seconds |
| All GitHub workflow files | actionlint passed |
| Rust formatting, toolchain contract, organization and documentation inventory | Passed |
| Whitespace check | Passed |

The GUI shard ran on macOS with Node 24.6.0 using isolated browser fixtures. This
confirms the complete shard can finish beyond test 237, but does not substitute for
the Ubuntu/Node 20 CI rerun or reproduce the remote mirror's transfer speed.
No runtime code, test-file selection, numerical assertion or dependency version
was changed. The downloaded validator's temporary directory was removed after
validation; existing unrelated workflow-recovery changes remain outside this fix.
