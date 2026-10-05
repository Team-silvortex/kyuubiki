# Modal Ranked Strategy Bounds And Paired Cost

Date: 2026-10-04
Source checkpoint: working-tree overlay on `dc5647af`, Git commit title `daji 3.4.6`.
Solver package and shipping manifests remain `3.4.5`; no version or installation is changed.
Platform: local macOS ARM64, Rust 1.88, optimized build.

## Scope And Decision

This is a test-only lower-cap strategy comparison, not production admission or complete budget qualification.

The [later checked-order handoff comparison](modal-banded-checked-order-handoff-20261004.md)
removes one repeated physical fit in layered-tiny. The historical full/ranked
timings and regression below are preserved, not replaced with its newer samples.

The [preceding banded candidate cost record](modal-banded-inverse-pipeline-cost-20261004.md)
measured six successful candidates but up to six grid factors and 23 actual
certificates. This follow-up keeps the same initializer, frozen roots, discrete
radius, four-pass fit, original compensated operator checks, strict physical
unit-norm gate and independent result JSON reconstruction. It does not append
new policies or larger budgets after that full portfolio.

The new separate route tries grid-column-norm order first, then reverse only
after a typed numerical rejection. It never tries natural order or a third fit.
Every new stage has two factor slots and thirteen certificate slots, versus
three/nineteen in the retained full portfolio. A single-order comparison entry
has one/seven. Faults and cancellation do not authorize the next factor.
The existing full-portfolio entry keeps its order and caps, and all new code
remains under test-only registration. Public Solver still rejects the six inputs.

All six new candidate chains pass independent physical JSON readback and exact
fresh replay. Five use one factor per stage; layered-tiny uses two per stage.
Maximum allowed chain factors drop from six to four and the combined grid
reservation drops from 2,018,181,120 to 1,347,092,480 visits. Observed maximum
certificates across these fixtures drop from 23 to 16; the formal combined
certificate cap is 26, not 16.

This is not universal acceleration. Tiny-layer work doubles, and layered-large
retains two factors but has slightly slower recovery phases in the paired samples.
Dense Jacobi still dominates complete fresh-input timing. Retain both strategies
as research comparisons, not an automatic universal production replacement or
a fixture-specific runtime selector.

## Isolated Order Evidence

Each of six rebuilt fixtures starts from its actual retained refined pair and
the same borrowed four-step inverse initializer. Three internal orders are run
independently, each with one observed factor and at most seven operator callbacks.
Only accepted internal proposals proceed to three independently selected
physical orders, again with one factor each. Candidates are not chained between
these comparisons.

| Case | Internal reverse / natural / grid norm | Physical reverse / natural / grid norm for accepted grid-norm internal |
| --- | --- | --- |
| graded unit | reject / reject / accept | accept / reject / accept |
| graded large | reject / reject / accept | reject / reject / accept |
| graded tiny | reject / reject / accept | accept / reject / accept |
| layered unit | reject / reject / accept | accept / reject / accept |
| layered large | accept / reject / accept | accept / reject / accept |
| layered tiny | accept / reject / reject | no grid-norm internal result |

Layered-tiny reverse internal then physical reverse succeeds; physical grid norm
rejects. Layered-large reverse internal additionally admits physical reverse and
grid norm. Across 18 internal proposals, seven succeed (reverse two, natural zero,
grid norm five). Across their 21 physical proposals, twelve succeed (reverse six,
natural zero, grid norm six). These twelve proposals are not twelve independent
fixtures. Typed rejection counts remain one attempt; unexpected faults fail the test.

The evidence rules out a single universal successful order on this sample.
It supports comparing two fixed strategies with lower caps; it does not prove
natural order is unnecessary on arbitrary matrices or the existing broader
full-portfolio baseline.

## Bounded Recovery And Numerical Results

At 256 active coordinates:

| Route | Factors per stage | Certificates per stage | Grid visits reserved per stage | Numeric payload per stage |
| --- | --- | --- | --- | --- |
| Single order | 1 | 7 | 338,001,920 | 6,914,048 bytes |
| Ranked then reverse | 2 | 13 | 673,546,240 | 6,914,048 bytes |
| Retained full portfolio | 3 | 19 | 1,009,090,560 | 6,914,048 bytes |

Reservations are charged before each fit and never reset between attempts.
Tests exhaust each reduced factor and certificate cap, verify no further
reservation is charged after rejection, exercise exact final-slot acceptance,
and reject invalid or overflow-prone dimensions before operator calls. Injected
nonfinite/mismatched certificates, operator errors carrying numerical-rejection
words, final certificate failure and strict norm failure cannot become retries
outside the selected policy set. Fresh healthy replay remains available.

Six-fixture results. Each work column is retained full / ranked. Readback residual
is for the ranked route; all pass the unchanged `1e-8` gate and strict `1e-10`
physical unit-norm gate. Ranked outputs need not equal another valid strategy's
vector bits, but root and initializer seed bits match full and each route repeats
its own accepted internal/physical bits exactly.

| Case | Total grid factors | Total actual grid certificates | Summed grid visits reserved | Ranked independent JSON residual |
| --- | --- | --- | --- | --- |
| graded-unit | 4 / 2 | 16 / 8 | 1,347,092,480 / 676,003,840 | 3.435872880e-9 |
| graded-large | 6 / 2 | 23 / 8 | 2,018,181,120 / 676,003,840 | 3.961311841e-9 |
| graded-tiny | 4 / 2 | 16 / 8 | 1,347,092,480 / 676,003,840 | 3.730244796e-9 |
| layered-unit | 4 / 2 | 16 / 8 | 1,347,092,480 / 676,003,840 | 4.891324560e-9 |
| layered-large | 2 / 2 | 8 / 8 | 676,003,840 / 676,003,840 | 4.420681052e-9 |
| layered-tiny | 2 / 4 | 8 / 16 | 676,003,840 / 1,347,092,480 | 8.589526570e-9 |

The strongest cost win is graded-large (six factors to two); the worst new case
is layered-tiny (two to four). Eighteen sign/binary-amplitude controls on all six
fixtures retain scaled normalized and signed physical bits and exact counts.
A real graded-large final normalized operator fault returns failure without
trying reverse. A real final physical restoration cancellation returns no result;
fresh physical recovery exactly repeats accepted bits and independent readback.
Original seeds and frozen eigenvalues remain unchanged.

The visits exclude initialization, request/assembly/Jacobi/refinement, ordinary
physical polish and independent JSON work. The grid certificate cap excludes
ordinary physical residual calls and the independent reference. Even a
673,546,240 per-stage reservation remains above the existing production
350-million bound. The lower cap is progress, not production budget compliance.
Numeric payload planning remains unchanged; no whole-fixture allocation or RSS
qualification is inferred.

## Paired Optimized Measurement

Six isolated processes each perform one full and one ranked warmup, then three
fresh samples per strategy. Pair order alternates full/ranked, ranked/full,
full/ranked. Every member builds its own input, operator, Jacobi result and
retained pair; neither shares cached preparation with its partner.
Compilation and other heavy regressions start after retained timing finishes.
Observer and receipt checks stay included.

Recovery duration below is computed by summing initializer, internal and physical
phase durations within each sample, then taking the median of those sums.
It is not the sum of independently computed phase medians. Complete total
includes fresh preparation and independent JSON/reference readback, but excludes
fixture destruction, inertia diagnostics and public-failure comparisons.

Milliseconds; percentage is ranked versus full recovery median.

| Case | Full recovery median | Ranked recovery median | Change | Full / ranked complete total median |
| --- | --- | --- | --- | --- |
| graded-unit | 86.368 | 43.588 | -49.5% | 865.742 / 769.963 |
| graded-large | 128.560 | 43.368 | -66.3% | 952.795 / 841.226 |
| graded-tiny | 85.135 | 48.090 | -43.5% | 848.038 / 781.120 |
| layered-unit | 85.705 | 45.087 | -47.4% | 747.646 / 660.249 |
| layered-large | 44.050 | 47.079 | 6.9% | 773.799 / 760.853 |
| layered-tiny | 48.278 | 91.461 | 89.4% | 660.934 / 856.035 |

Layered-tiny has ranked recovery samples 89.394, 91.461 and 407.591 ms.
The long-tail sample is retained, not discarded. Full samples are 43.331,
48.278 and 53.346 ms. This campaign cannot justify a universal speedup or
latency SLO. Complete total differences also contain Jacobi/desktop scheduling
variation; reduced factor counts are a more stable explanation of proposal
cost than attributing all total-time differences to the new ordering.

Native process high-water RSS spans 11,808 to 13,136 KiB across paired processes.
It includes both strategies, warmups and samples; it cannot assign memory savings
to one strategy or prove incremental heap bounds. The fixed local numeric
payload plan does not decrease merely because there are fewer sequential fits.

### Retained Receipts

Phase order: total, preparation, initializer, normalized recovery, ordinary
physical preparation/grid recovery, JSON/reference readback, input
validation/assembly, dense matrix/Jacobi, retained-pair refinement, normalized
direction construction. Values are milliseconds; RSS fields are KiB.

```json
[
  {
    "case": "graded-unit",
    "baseline": {
      "grid_visits_reserved": 1347092480,
      "independent_readback_residual": 7.70586095641011e-9,
      "internal_checks": 12,
      "internal_factors": 3,
      "phase_medians_ms": [
        865.7415000000001,
        779.173959,
        0.832375,
        63.979499999999994,
        21.555833,
        0.19720800000000002,
        0.33883300000000005,
        774.532083,
        4.0015,
        0.31920899999999996
      ],
      "phase_samples_ms": [
        [
          707.059833,
          622.335875,
          0.817583,
          62.464125,
          21.242959,
          0.19720800000000002,
          0.33883300000000005,
          617.7295419999999,
          3.9481669999999998,
          0.318666
        ],
        [
          865.7415000000001,
          779.173959,
          0.832375,
          63.979499999999994,
          21.555833,
          0.197958,
          0.320334,
          774.532083,
          4.0015,
          0.31920899999999996
        ],
        [
          921.2255,
          832.4400840000001,
          0.87175,
          65.27079099999999,
          22.444542000000002,
          0.193417,
          0.344792,
          827.473125,
          4.237875000000001,
          0.383083
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "ranked": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 3.4358728797234872e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        769.9630830000001,
        726.17975,
        0.840334,
        21.252375,
        21.541541000000002,
        0.195542,
        0.33225000000000005,
        721.572167,
        3.9508340000000004,
        0.322
      ],
      "phase_samples_ms": [
        [
          752.825667,
          706.846333,
          0.9836249999999999,
          22.623167,
          22.164040999999997,
          0.20566700000000002,
          0.32345799999999997,
          701.346084,
          4.662125,
          0.512709
        ],
        [
          769.9630830000001,
          726.17975,
          0.821084,
          21.225250000000003,
          21.541541000000002,
          0.193584,
          0.34275,
          721.572167,
          3.9508340000000004,
          0.312958
        ],
        [
          902.67375,
          859.049833,
          0.840334,
          21.252375,
          21.333417,
          0.195542,
          0.33225000000000005,
          854.45325,
          3.941541,
          0.322
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "process_peak_rss_before_kib": 2912,
    "process_peak_rss_after_kib": 12064
  },
  {
    "case": "graded-large",
    "baseline": {
      "grid_visits_reserved": 2018181120,
      "independent_readback_residual": 3.961311840787512e-9,
      "internal_checks": 12,
      "internal_factors": 3,
      "phase_medians_ms": [
        952.794875,
        822.788375,
        0.892958,
        63.93045899999999,
        62.892584,
        0.212583,
        0.32533300000000004,
        818.013375,
        4.071375000000001,
        0.332125
      ],
      "phase_samples_ms": [
        [
          719.4794169999999,
          594.203959,
          0.816208,
          62.829083,
          61.420415999999996,
          0.208,
          0.321083,
          589.6318339999999,
          3.931917,
          0.318291
        ],
        [
          952.794875,
          822.788375,
          1.2687499999999998,
          65.628459,
          62.892584,
          0.214625,
          0.32533300000000004,
          818.013375,
          4.116625,
          0.332125
        ],
        [
          1013.2301249999999,
          884.455667,
          0.892958,
          63.93045899999999,
          63.736582999999996,
          0.212583,
          0.339667,
          879.710958,
          4.071375000000001,
          0.33270799999999995
        ]
      ],
      "physical_checks": 11,
      "physical_factors": 3,
      "physical_polish_steps": 4
    },
    "ranked": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 3.961311840787512e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        841.2262499999999,
        797.649541,
        0.833542,
        21.253083,
        21.270500000000002,
        0.2105,
        0.326084,
        792.948625,
        3.9361669999999997,
        0.319666
      ],
      "phase_samples_ms": [
        [
          816.416666,
          770.550791,
          0.823792,
          22.959375,
          21.867791,
          0.212916,
          0.322458,
          765.981166,
          3.927333,
          0.319083
        ],
        [
          849.179958,
          805.7680419999999,
          0.833542,
          21.14875,
          21.21775,
          0.2105,
          0.326084,
          801.185583,
          3.9361669999999997,
          0.319666
        ],
        [
          841.2262499999999,
          797.649541,
          0.8445,
          21.253083,
          21.270500000000002,
          0.207125,
          0.341875,
          792.948625,
          4.037708,
          0.32054099999999996
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "process_peak_rss_before_kib": 2912,
    "process_peak_rss_after_kib": 12112
  },
  {
    "case": "graded-tiny",
    "baseline": {
      "grid_visits_reserved": 1347092480,
      "independent_readback_residual": 7.07847211304148e-9,
      "internal_checks": 12,
      "internal_factors": 3,
      "phase_medians_ms": [
        848.0382910000001,
        763.5900829999999,
        0.829333,
        63.22520900000001,
        21.090999999999998,
        0.209791,
        0.331,
        758.938875,
        4.001541,
        0.32899999999999996
      ],
      "phase_samples_ms": [
        [
          708.661291,
          623.3169160000001,
          0.818416,
          63.22520900000001,
          21.090999999999998,
          0.207833,
          0.334083,
          618.714334,
          3.9386669999999997,
          0.32899999999999996
        ],
        [
          848.0382910000001,
          763.5900829999999,
          0.829333,
          62.456875000000004,
          20.949959,
          0.209791,
          0.32875,
          758.938875,
          4.001541,
          0.319916
        ],
        [
          852.402917,
          765.478709,
          0.833708,
          64.169375,
          21.708875,
          0.210333,
          0.331,
          760.479291,
          4.335625,
          0.331708
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "ranked": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 3.730244795953667e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        781.119542,
        732.811375,
        0.9006249999999999,
        24.592167,
        22.673792000000002,
        0.21329199999999998,
        0.33337500000000003,
        728.1775,
        4.152125000000001,
        0.332542
      ],
      "phase_samples_ms": [
        [
          781.119542,
          732.811375,
          0.8237500000000001,
          24.592167,
          22.673792000000002,
          0.216167,
          0.325916,
          728.1775,
          3.984667,
          0.32249999999999995
        ],
        [
          745.6477500000001,
          688.1421250000001,
          1.340042,
          30.104292,
          25.844959,
          0.21329199999999998,
          0.341042,
          683.315291,
          4.152125000000001,
          0.332542
        ],
        [
          891.272375,
          845.515208,
          0.9006249999999999,
          22.541291,
          22.103583,
          0.208791,
          0.33337500000000003,
          839.8684589999999,
          4.3721250000000005,
          0.939709
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "process_peak_rss_before_kib": 2912,
    "process_peak_rss_after_kib": 13120
  },
  {
    "case": "layered-unit",
    "baseline": {
      "grid_visits_reserved": 1347092480,
      "independent_readback_residual": 8.333676261348751e-9,
      "internal_checks": 12,
      "internal_factors": 3,
      "phase_medians_ms": [
        747.645583,
        664.902,
        0.838042,
        61.548834,
        21.770584,
        0.190417,
        0.328542,
        660.2357079999999,
        4.103583,
        0.319791
      ],
      "phase_samples_ms": [
        [
          995.719208,
          907.370667,
          0.988708,
          65.396541,
          21.770584,
          0.190042,
          0.324459,
          901.074834,
          5.632416,
          0.337167
        ],
        [
          672.774458,
          586.870666,
          0.838042,
          61.548834,
          23.318208,
          0.196708,
          0.352833,
          582.094166,
          4.103583,
          0.319083
        ],
        [
          747.645583,
          664.902,
          0.836209,
          60.608000000000004,
          21.107292,
          0.190417,
          0.328542,
          660.2357079999999,
          4.0169999999999995,
          0.319791
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "ranked": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 4.891324560229968e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        660.249125,
        614.917333,
        0.9175420000000001,
        21.535249999999998,
        22.701416,
        0.242458,
        0.336166,
        610.3099579999999,
        3.9944170000000003,
        0.33754199999999995
      ],
      "phase_samples_ms": [
        [
          660.249125,
          614.917333,
          0.850666,
          21.535249999999998,
          22.701416,
          0.242458,
          0.336166,
          610.3099579999999,
          3.9505410000000003,
          0.31975
        ],
        [
          483.010625,
          427.253583,
          0.9175420000000001,
          26.275958,
          28.160917,
          0.399625,
          0.3545,
          421.853083,
          4.707167,
          0.33754199999999995
        ],
        [
          715.131666,
          671.265125,
          0.930125,
          20.896958,
          21.797542,
          0.239458,
          0.33025,
          666.59625,
          3.9944170000000003,
          0.342916
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "process_peak_rss_before_kib": 2912,
    "process_peak_rss_after_kib": 12672
  },
  {
    "case": "layered-large",
    "baseline": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 8.98197842588466e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        773.7994580000001,
        729.53975,
        0.852917,
        22.117416000000002,
        21.627,
        0.21366700000000002,
        0.32983300000000004,
        724.781459,
        4.033791999999999,
        0.331666
      ],
      "phase_samples_ms": [
        [
          625.796416,
          581.988375,
          0.823584,
          21.137709,
          21.627,
          0.21775,
          0.327291,
          577.295917,
          4.033791999999999,
          0.330459
        ],
        [
          773.7994580000001,
          729.53975,
          0.852917,
          22.117416000000002,
          21.079375000000002,
          0.207833,
          0.40425,
          724.781459,
          4.021542,
          0.331666
        ],
        [
          801.9997079999999,
          753.625416,
          0.8861249999999999,
          25.39425,
          21.877708,
          0.21366700000000002,
          0.32983300000000004,
          748.762084,
          4.199583,
          0.332542
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "ranked": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 4.420681051879533e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        760.852583,
        713.09075,
        0.945833,
        23.379,
        22.310792,
        0.2095,
        0.339167,
        708.455833,
        3.98375,
        0.319042
      ],
      "phase_samples_ms": [
        [
          634.9989579999999,
          587.709292,
          0.83075,
          23.937,
          22.310792,
          0.2095,
          0.360375,
          583.05975,
          3.9691250000000005,
          0.319042
        ],
        [
          760.852583,
          713.09075,
          0.945833,
          23.379,
          23.208459,
          0.22612500000000002,
          0.331542,
          708.455833,
          3.98375,
          0.318791
        ],
        [
          1002.3014999999999,
          955.539417,
          1.448333,
          23.094666999999998,
          22.013917,
          0.20274999999999999,
          0.339167,
          950.821209,
          4.002417,
          0.375084
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "process_peak_rss_before_kib": 2912,
    "process_peak_rss_after_kib": 13136
  },
  {
    "case": "layered-tiny",
    "baseline": {
      "grid_visits_reserved": 676003840,
      "independent_readback_residual": 8.589526570335008e-9,
      "internal_checks": 4,
      "internal_factors": 1,
      "phase_medians_ms": [
        660.9343749999999,
        612.427625,
        0.910334,
        22.396917,
        24.258875,
        0.22187500000000002,
        0.345833,
        607.631833,
        4.094,
        0.337792
      ],
      "phase_samples_ms": [
        [
          622.7675,
          569.1947080000001,
          2.6245000000000003,
          22.396917,
          28.324958000000002,
          0.22187500000000002,
          0.328375,
          564.014833,
          4.094,
          0.754792
        ],
        [
          660.9343749999999,
          612.427625,
          0.910334,
          23.109125000000002,
          24.258875,
          0.225416,
          0.345833,
          607.631833,
          4.111292,
          0.337792
        ],
        [
          686.651208,
          643.106291,
          0.862,
          21.200375,
          21.268917,
          0.2115,
          0.355833,
          638.4802080000001,
          3.950917,
          0.3185
        ]
      ],
      "physical_checks": 4,
      "physical_factors": 1,
      "physical_polish_steps": 4
    },
    "ranked": {
      "grid_visits_reserved": 1347092480,
      "independent_readback_residual": 8.589526570335008e-9,
      "internal_checks": 8,
      "internal_factors": 2,
      "phase_medians_ms": [
        856.03525,
        618.771666,
        0.885708,
        42.874834,
        48.209500000000006,
        0.22270800000000002,
        0.360958,
        613.394792,
        4.147125,
        0.3365
      ],
      "phase_samples_ms": [
        [
          999.992167,
          591.84,
          0.878666,
          202.67154200000002,
          204.041084,
          0.5568329999999999,
          0.360958,
          583.9357080000001,
          7.2008339999999995,
          0.341292
        ],
        [
          708.3892910000001,
          618.771666,
          0.885708,
          42.874834,
          45.633834,
          0.21995900000000002,
          0.946291,
          613.394792,
          4.095958,
          0.33366599999999996
        ],
        [
          856.03525,
          764.348292,
          0.9037080000000001,
          42.34825,
          48.209500000000006,
          0.22270800000000002,
          0.33325,
          759.5305,
          4.147125,
          0.3365
        ]
      ],
      "physical_checks": 8,
      "physical_factors": 2,
      "physical_polish_steps": 4
    },
    "process_peak_rss_before_kib": 2912,
    "process_peak_rss_after_kib": 11808
  }
]
```

## Replay And Qualification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release modal_banded_ranked_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release triangular_grid_selected_strategies_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release modal_banded_single_policy_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release modal_banded_ranked_pipeline_cost_graded_unit --locked --offline -- --ignored --nocapture --test-threads=1
```

Run each paired-cost suffix separately and sequentially: `graded_unit`,
`graded_large`, `graded_tiny`, `layered_unit`, `layered_large`, `layered_tiny`.
Ignored benchmark entries reject Debug builds. Three explicit control commands
and six paired-cost commands are registered in the modal validation profile.
Registry inspection is not a benchmark execution.

The existing verified validation and local-cost claims gain this scoped
comparison. Claim count remains 226, coordinate targets met remain 58/77 and
scenario targets met remain 10/32. No production execution binding, qualified
whole-budget evidence, live Agent, installed Headless study or release promotion
is added. Arbitrary topology, independent candidate request reassembly, multimode,
complete allocation/work accounting and non-stacked production integration remain open.

## Final verification

All checks below were run locally on macOS ARM64 against the same uncommitted
source overlay. Focused tests are included in the full-suite count, not extra
independent coverage. Registered benchmark commands are not counted as runs.

| Check | Observed result |
| --- | --- |
| Complete optimized Solver library | 679 passed, 0 failed, 28 explicit cost tests ignored; 170.00 seconds |
| Optimized ranked replay/final-fault/sign controls | 3 passed, 0 failed; 6 paired-cost entries ignored |
| Optimized selected-strategy aggregate budget/fault controls | 2 passed, 0 failed |
| Optimized isolated single-order comparison | 1 passed, 0 failed |
| Six isolated paired optimized cost processes | Each process passed its one requested benchmark; six actual executions |
| Debug selected-strategy aggregate budget/fault controls | 2 passed, 0 failed |
| Debug actual final fault/cancellation/replay | 1 passed, 0 failed; 5.71 seconds |
| Debug sign/binary-amplitude/frozen-root controls | 1 passed, 0 failed; 18 exact checked controls; 40.37 seconds |
| Tensor source regression tests | 21 passed, 0 failed |
| Solver and CLI all-target strict Clippy | Passed with warnings denied |
| Script Runner all-target strict Clippy | Passed with warnings denied |
| Native tensor check and self-test | Passed; 0 structural gaps, 4 maturity gaps, 19 evidence-grade gaps; qualification remains blocked |
| Native operator registry check and self-test | Passed; 59 profiles; registry inspection reports executed=false |
| Documentation inventory and HTML book check | Passed; development and shipping manifests remain 3.4.5 |
| Project organization audit and self-test | Passed; source limit 800, document limit 2000, tracked debt 0 |

No remote, cross-platform, installed-app or live Agent execution was performed
in this campaign. These checks do not change the production solver's rejection
of the retained six heterogeneous inputs or qualify the complete work budget.
