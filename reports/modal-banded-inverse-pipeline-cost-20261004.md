# Banded Modal Candidate Storage And Full Pipeline Cost

The subsequent [lower-cap paired policy comparison](modal-banded-ranked-policy-cost-20261004.md)
retains this three-policy campaign as historical evidence. It separately measures
ranked-then-reverse recovery, including the tiny-layer regression, and does not
inherit a universal speedup or complete production budget qualification.

Date: 2026-10-04
Source checkpoint: working-tree overlay on `dc5647af`, Git commit title `daji 3.4.6`.
The Solver Cargo package remains `3.4.5`; this work does not bump versions or rebuild installed applications.
Platform: local macOS ARM64, Rust 1.88, optimized build.

## Scope And Decision

This is test-only banded candidate cost evidence, not a complete production work or memory qualification.

The [preceding six-fixture recovery](modal-banded-inverse-candidate-reliability-20261004.md)
still separates wide proposal accuracy from actual normalized and physical
floating-point acceptance. This follow-up makes its initializer less wasteful
and measures fresh preparation through independent physical JSON readback.
No old Gram or direct-grid recovery is stacked before the measured banded path.
The public Solver still rejects all six retained graded/layered 128-element
requests; numerical gates, Engine/Agent boundaries and production budgets are unchanged.

Six isolated optimized processes each execute one warmup and three fresh runs.
All six succeed in the candidate harness, and each sample exactly replays its
warmup root, proposed seed, accepted internal/physical vector bits, independent
residual bits, observed work and modeled grid reservation. They are repeated
executions of six fixtures, not 24 independent physical models.

The initializer is inexpensive here, but the hardest graded-large chain
consumes every allowed grid factor: three internal and three physical.
Its summed grid reservation alone is 2,018,181,120 component visits.
This is not compatible with treating the existing production 350-million
per-stage budget as a complete budget for this new recovery chain.
Do not promote by increasing that limit or appending this path after old retries.
Dense Jacobi preparation dominates measured time in this particular harness.
These measurements do not prove a general public-runtime bottleneck.

## Borrowed Final Only Initializer

The factor now borrows the shifted normalized matrix and reconstructs each
accessed unshifted diagonal as the same wide addition used by the old dense
copy. It never receives an independently computed eigenvalue. The retained
root and original seed are unchanged.

A shared iteration core supports diagnostic history and a final-only result.
The latter keeps only the current iterate and one substitution vector, with
four solves and the same final cancellation checkpoint. Both paths preserve
the same arithmetic order. Across six fixtures and steps one through four,
24 comparisons match both high and low bits against the copied-matrix/history
reference. Shift range, malformed data, zero/oversized iteration budgets,
including `usize::MAX`, reject before unbounded allocation. Six actual
factor/forward/diagonal/backward/final-iteration cancellation points reject and
allow a fresh exact replay.

At 256 active coordinates:

| Numeric storage | Bytes | Interpretation |
| --- | --- | --- |
| Factor lower, diagonal and scale | 16,400 | Actual vector capacities plus scale; 64n + 16 |
| Final-only initializer owned peak | 24,592 | Factor plus two live wide vectors; not RSS |
| Avoided dense matrix clone | 1,048,576 | `16 * n * n`; former preparation allocation |
| Avoided accumulated direction history | 16,384 | Four wide vectors; not additive peak savings |
| Existing grid reservation per stage | 6,914,048 | Existing conservative local numeric plan |
| Caller retained wide and physical dense matrices | 1,572,864 | Outside initializer ownership; not complete fixture storage |

The factor and wide iterate are destroyed before normalized grid recovery starts.
The rounded initializer is destroyed before physical recovery; the accepted
internal vector remains available. No history is carried into publication.
The initializer payload omits vector headers, allocator overhead and the borrowed
matrix. Caller storage also includes request objects, sparse stiffness/mass,
seeds, maps and result/reference JSON not covered by the dense-matrix row.
Neither adding stage payloads nor comparing RSS with an 8 MiB numeric plan
qualifies full-production peak memory.

## Measured Fresh Pipeline

Each process generates and validates a new input, assembles stiffness/mass,
runs dense Jacobi, retains the actual best pair after failed ordinary four-step
refinement, and constructs normalized directions. Timing then includes the
borrowed banded initializer, normalized grid portfolio, ordinary physical
proposal, physical direction preparation and grid portfolio, constructed
result JSON and independent stiffness/mass reassembly.

Measured total excludes fixture destruction, independent inertia diagnosis and
public-failure comparisons. Work observers and receipt bookkeeping are included.
The eigensolve and failed refinement are preparation, not successful production
admission. Separate phase medians are not additive, and independent processes
do not isolate desktop/OS activity. No comparative latency speedup or SLO is claimed.

Milliseconds. RSS is native cumulative process high water in KiB, including
warmup and samples, not incremental heap or a stage-local peak.

| Case | Total median | Total samples | Preparation | Initializer | Internal | Physical | Readback | Peak RSS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| graded-unit | 631.255 | 578.018, 631.255, 703.594 | 547.329 | 0.832 | 63.289 | 20.983 | 0.195 | 13344 |
| graded-large | 708.317 | 708.314, 775.268, 708.317 | 583.786 | 0.816 | 62.877 | 60.634 | 0.209 | 12496 |
| graded-tiny | 550.675 | 550.675, 557.638, 550.512 | 466.895 | 0.836 | 62.007 | 20.773 | 0.199 | 12496 |
| layered-unit | 670.072 | 599.099, 670.072, 741.355 | 587.194 | 0.811 | 60.734 | 21.048 | 0.195 | 13360 |
| layered-large | 525.049 | 531.226, 523.902, 525.049 | 480.589 | 0.825 | 22.129 | 21.359 | 0.199 | 13504 |
| layered-tiny | 644.935 | 644.935, 664.066, 566.292 | 602.244 | 0.805 | 20.680 | 20.843 | 0.194 | 12480 |

All initial process peaks are 2,864 KiB. Final peaks span 12,480 to 13,504 KiB.
Independent readbacks preserve the preceding residuals, including graded-large
at `3.961311840787512e-9`, beneath the unchanged `1e-8` gate.

### Recovery Work Ledger

Grid factor counts come from observed `DenseFactor,0` entries. Certificate
counts come from actual normalized/physical operator callbacks, independently
matched to returned usage. Lower-level fit and portfolio searches emit duplicate
`ModalRoundoffSearch` checkpoints; event counts must not be interpreted as
certificate counts. The initial implementation caught this mismatch (23 events
versus 12 actual normalized calls) and was corrected before retaining measurements.

Every fixture has exactly one banded factor and four banded solves, zero grid
factors during initialization and four ordinary physical polish steps.
The two grid stages retain at most six factors and 38 actual certificates
combined, including their fresh final certificates. Actual physical callbacks
are also independently counted, not inferred only from receipt metadata.

| Case | Internal / physical factors | Internal / physical certificates | Physical polish steps | Summed grid visits reserved | Independent JSON residual |
| --- | --- | --- | --- | --- | --- |
| graded-unit | 3 / 1 | 12 / 4 | 4 | 1,347,092,480 | 7.705860956e-9 |
| graded-large | 3 / 3 | 12 / 11 | 4 | 2,018,181,120 | 3.961311841e-9 |
| graded-tiny | 3 / 1 | 12 / 4 | 4 | 1,347,092,480 | 7.078472113e-9 |
| layered-unit | 3 / 1 | 12 / 4 | 4 | 1,347,092,480 | 8.333676261e-9 |
| layered-large | 1 / 1 | 4 / 4 | 4 | 676,003,840 | 8.981978426e-9 |
| layered-tiny | 1 / 1 | 4 / 4 | 4 | 676,003,840 | 8.589526570e-9 |

Visits are summed existing conservative reservations, not instruction counts.
The maximum combined grid reservation is 2,018,181,120. They exclude banded
initialization, input/assembly/Jacobi/refinement, physical proposal and JSON/reference
work. The 38-certificate grid cap excludes ordinary physical residual evaluation
and the independent readback. These explicit exclusions prevent reporting this
ledger as a full-production work/check bound. Ordinary tests execute twelve fresh
chains and compare six exact repeated receipts, so the assertions are not limited
to explicitly ignored benchmark entries.

### Retained Timing Samples

Order: total, preparation, initializer, normalized grid, physical preparation/grid,
JSON/reference readback, input validation/assembly, dense matrix/Jacobi,
ordinary retained-pair refinement, normalized directions. Values are milliseconds.
The replay command also emits initializer storage and the work ledger fields.

```json
[
  {
    "case": "graded-unit",
    "phase_samples_ms": [
      [
        578.017875,
        492.331125,
        0.806083,
        63.289334000000004,
        21.394584000000002,
        0.19512500000000002,
        0.335625,
        487.744167,
        3.924708,
        0.325708
      ],
      [
        631.255416,
        547.328833,
        0.834041,
        61.918583,
        20.982917,
        0.18920800000000002,
        0.324208,
        542.821208,
        3.8595,
        0.32329199999999997
      ],
      [
        703.594334,
        618.338042,
        0.8317910000000001,
        63.34970800000001,
        20.873124999999998,
        0.199708,
        0.330792,
        613.7342500000001,
        3.9482079999999997,
        0.32404200000000005
      ]
    ],
    "process_peak_rss_before_kib": 2864,
    "process_peak_rss_after_kib": 13344
  },
  {
    "case": "graded-large",
    "phase_samples_ms": [
      [
        708.314333,
        582.267666,
        0.816125,
        62.78570799999999,
        62.233167,
        0.209042,
        0.332417,
        577.500375,
        4.1120410000000005,
        0.321667
      ],
      [
        775.268208,
        649.825625,
        0.817417,
        63.897042,
        60.510374999999996,
        0.215708,
        0.332125,
        645.212958,
        3.9569999999999994,
        0.322625
      ],
      [
        708.316792,
        583.786417,
        0.81125,
        62.876625000000004,
        60.633582999999994,
        0.206958,
        0.331708,
        579.126625,
        4.016209,
        0.31054200000000004
      ]
    ],
    "process_peak_rss_before_kib": 2864,
    "process_peak_rss_after_kib": 12496
  },
  {
    "case": "graded-tiny",
    "phase_samples_ms": [
      [
        550.675333,
        466.89545799999996,
        0.7969160000000001,
        62.007209,
        20.772958000000003,
        0.200875,
        0.326083,
        462.416,
        3.8427499999999997,
        0.31
      ],
      [
        557.638375,
        474.568458,
        0.8362499999999999,
        61.279875,
        20.754749999999998,
        0.19683299999999998,
        0.31712500000000005,
        470.032,
        3.9074169999999997,
        0.311
      ],
      [
        550.5124999999999,
        466.489666,
        0.848834,
        62.085875,
        20.8805,
        0.198875,
        0.3155,
        461.904417,
        3.958458,
        0.310625
      ]
    ],
    "process_peak_rss_before_kib": 2864,
    "process_peak_rss_after_kib": 12496
  },
  {
    "case": "layered-unit",
    "phase_samples_ms": [
      [
        599.099084,
        513.382625,
        0.8109999999999999,
        63.302792,
        21.357042,
        0.244167,
        0.34025,
        508.838459,
        3.8801669999999997,
        0.322959
      ],
      [
        670.0716669999999,
        587.1944169999999,
        0.898458,
        60.733957999999994,
        21.04775,
        0.19525,
        0.319833,
        582.628958,
        3.932333,
        0.312583
      ],
      [
        741.3552920000001,
        658.8550829999999,
        0.810333,
        60.677457999999994,
        20.817875,
        0.19320900000000002,
        0.3265,
        654.282791,
        3.933583,
        0.311458
      ]
    ],
    "process_peak_rss_before_kib": 2864,
    "process_peak_rss_after_kib": 13360
  },
  {
    "case": "layered-large",
    "phase_samples_ms": [
      [
        531.226042,
        487.380167,
        0.8433750000000001,
        21.215625,
        21.585791999999998,
        0.199459,
        0.318959,
        482.86825,
        3.858292,
        0.33387500000000003
      ],
      [
        523.902042,
        479.392459,
        0.824625,
        22.129458,
        21.359166,
        0.194666,
        0.317542,
        474.86375,
        3.888959,
        0.321416
      ],
      [
        525.048625,
        480.589417,
        0.8167909999999999,
        22.135208,
        21.300458000000003,
        0.20216599999999998,
        0.32125,
        476.085125,
        3.8718749999999997,
        0.31033299999999997
      ]
    ],
    "process_peak_rss_before_kib": 2864,
    "process_peak_rss_after_kib": 13504
  },
  {
    "case": "layered-tiny",
    "phase_samples_ms": [
      [
        644.934667,
        602.2435419999999,
        0.807709,
        20.825,
        20.843375,
        0.212583,
        0.403917,
        597.6611670000001,
        3.863709,
        0.31416700000000003
      ],
      [
        664.066375,
        621.6695840000001,
        0.79925,
        20.67975,
        20.723875,
        0.192125,
        0.319667,
        617.129958,
        3.909209,
        0.310084
      ],
      [
        566.291625,
        523.167458,
        0.804541,
        20.639333,
        21.483999999999998,
        0.194,
        0.315583,
        518.6237500000001,
        3.9172089999999997,
        0.31012500000000004
      ]
    ],
    "process_peak_rss_before_kib": 2864,
    "process_peak_rss_after_kib": 12480
  }
]
```

## Replay And Qualification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release modal_banded_inverse_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release modal_banded_pipeline_cost_graded_unit --locked --offline -- --ignored --nocapture --test-threads=1
```

Execute each suffix separately and sequentially: `graded_unit`, `graded_large`,
`graded_tiny`, `layered_unit`, `layered_large`, `layered_tiny`.
Debug benchmark builds reject. Six exact named commands are registered in the
modal operator profile; profile checks inspect those commands rather than execute
the benchmark. The existing local cost claim gains this separately measured
campaign. Claim count remains 226; coordinate and scenario targets remain 58/77
and 10/32. Qualified production work/memory scope remains unbound and unmet.

| Final check | Result |
| --- | --- |
| Optimized banded inverse regression | 9 passed, 0 failed; includes fresh work receipts |
| Complete optimized Solver library | 673 passed, 0 failed, 22 explicit cost entries ignored; 75.39 seconds |
| Six isolated cost processes | One named test passed in each process, no failures |
| Debug final-only cancellation, exact wide-bit and fresh work receipt controls | Three named tests passed, no failures; receipt replay 68.30 seconds |
| Solver and CLI all-target strict Clippy | Passed with `-D warnings` |
| Tensor source regression | 21 passed, 0 failed |
| Script runner all-target strict Clippy | Passed with `-D warnings` |
| Tensor checker and self-test | Structure passed; 4 maturity and 19 evidence-grade gaps, 14 P0 gaps retained |
| Operator registry and self-test | Passed; 59 profiles; `executed=false` |
| Documentation inventory and book | Passed; 26 HTML files; development/shipping manifests remain 3.4.5 |
| Organization audit and self-test | Passed; source <=800, docs <=2000; tracked debt 0 |
| Formatting and whitespace | Passed |

The focused optimized tests are included in the full suite, not additive physical
coverage. Debug controls are a subset of the same algorithm checks. No installed
application, new live Agent, Headless study, remote or cross-platform run is
claimed. Complete production allocation/certificate/work accounting, narrower
non-stacked correction cost, independently rebuilt candidate requests and actual
Agent/Headless recovery journeys remain open.
