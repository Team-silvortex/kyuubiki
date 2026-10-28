# CI Benchmark Gate Follow-Up

Reviewed October 28, 2026. Scope: the paired Rust benchmark gate, failure evidence
retention and independent timing/memory thresholds. No solver algorithm, runtime
recovery behavior, dependency version or product version changes are included.

## Observed Failure

The [failed CI run](https://github.com/Team-silvortex/kyuubiki/actions/runs/40652915227)
at `7e08fceac1e11515e493eb0031b8fd4856abbadb` had one failing job: `rust-test`,
at `Gate current performance against the paired baseline`. Web, solver, GUI,
architecture, SDK and dependency-audit jobs passed. This was a gate exit code 1,
not a timeout or a recurrence of the Bandit security audit failure.

Its `plane-panel-medium` median moved from 2.276514 ms to approximately 3.7788 ms
(+65.99%), exceeding the unchanged 25% limit. The current minimum was 2.2749 ms;
the other five case medians stayed within about 1.4% of baseline. The baseline
revision was `59b95efe648d9e8bd0feb12839f227328504e611`. Both revisions have the
same `workers/rust` Git tree, `c7b1a389afc2a781b1e233237ba6d0ffdcdbc95f`.
The evidence is consistent with short-sample runner variability, not a Rust
source regression; aggregate timings alone do not identify the host disturbance.

The uploaded artifact contained the baseline, comparison Markdown and revision
IDs, but no current JSON: the runner exited on gate failure before writing it.
Inspection also found that the baseline timing floor skipped RSS evaluation,
although the flag is documented as a timing-only limit.

## Changes

- CI still measures the previous/PR-base revision and current revision on the
  same runner with process-isolated cases. Both now use `core/large`, 31 repeats,
  rather than seven `core/medium` samples. All six families remain present.
- The 25% median and 20% RSS thresholds and 1-ms CI timing floor are unchanged.
  The job budget is 35 minutes to accommodate the larger paired measurements;
  no failure is ignored and there is no retry-until-green loop.
- Completed current JSON is written before comparison errors or gate exits.
  The original baseline is read first, so coincident input/output paths cannot
  silently compare the current run against itself. Failed gates remain exit 1;
  missing or unreadable baselines remain configuration errors, exit 2.
- The timing floor now controls only timing. Configured RSS gates also apply to
  short cases; scope mismatches and missing baseline cases continue to fail.

## Verification

Before production changes, the two timing-floor tests yielded one failure and
one passing control. The four CLI artifact tests yielded three failures and one
passing control after fixing the test harness's option spelling and temporary
directory collision. The failures covered missing current JSON, failed-baseline
evidence, and stale output when comparison input and output paths coincide.

After the fix, `cargo test --locked -p kyuubiki-benchmark` passed 75 unit tests
and four real-CLI integration tests; 21 existing large-scale generation tests
remain explicitly ignored in this lane. Tests verify rejection is retained, JSON
matches stdout, same-path comparison uses the original baseline, and timing/RSS
thresholds operate independently. These are benchmark-harness checks, not new
numerical or full-workspace qualification.

The local unoptimized `core/large` pair used 31 repeats per case and process
isolation, comparing a copied pre-fix benchmark executable against the rebuilt
one. Solver/generator code was unchanged. An initial current measurement overlapped
repository format/document checks and failed for truss (+64.37%) and space-frame
(+69.59%); its current JSON was correctly retained. That confounded run is not
reported as passing or used to relax thresholds. With those checks finished, one
additional current measurement against the same baseline passed all six cases:

| Case | Baseline median ms | Current median ms | Median delta | RSS delta |
| --- | ---: | ---: | ---: | ---: |
| axial-bar-large | 0.2460 | 0.2715 | +10.38% | -1.00% |
| truss-roof-large | 312.2925 | 313.7995 | +0.48% | -0.76% |
| space-frame-large | 735.9074 | 738.5405 | +0.36% | +0.42% |
| plane-panel-large | 1878.8840 | 1916.9167 | +2.02% | +0.00% |
| plane-quad-panel-large | 1939.1843 | 1971.3723 | +1.66% | +4.73% |
| heat-plane-quad-large | 212.9724 | 214.4520 | +0.69% | -1.46% |

The axial case remains below the 1-ms timing floor, but its RSS is now gated.
This shows the larger lane is executable locally, not immunity to sustained host
load. CI does not add that manual diagnostic retry: each revision is measured
once with the configured repeat count, and any gate failure remains blocking.
`cargo fmt --check`, whitespace checks, the YAML sampling/threshold policy check,
HTML book/inventory checks and the project organization audit pass. No source
file exceeds 800 lines and the touched documents stay under 2,000 lines.

The downloaded CI artifacts, copied executable and locally generated reports
were removed with their owned temporary directory after verification. No raw
benchmark data or new source checkout is retained in Git. This macOS validation does not itself prove
the next Ubuntu CI run will pass or eliminate all shared-runner timing noise.
