# Modal checked order handoff cost and reliability

October 4, 2026. Local macOS ARM64 source overlay on commit `dc5647af`
(`daji 3.4.6`); Cargo and shipping/development manifests remain 3.4.5.
Rust toolchain 1.88.0. This campaign extends the
[ranked two-policy comparison](modal-banded-ranked-policy-cost-20261004.md),
not its earlier full-portfolio baseline.

This is test-only checked-order handoff evidence, not production admission or complete budget qualification.

The later [request reassembly and material boundary follow-up](modal-banded-handoff-request-reassembly-20261004.md)
retains 24 fresh numbering successes but two of eighteen changed material inputs
reject even the independently rebuilt full physical portfolio. The six original
successes below are not general material coverage or production admission.

## What changed

The ranked normalized stage still tries grid-norm then reverse, with two fits
and thirteen certificates at most. Its accepted `Order` tag now selects a
single physical proposal strategy instead of restarting the ranked portfolio.
That tag describes a strategy, not a coordinate permutation: the physical
stage builds its own mass-aware canonical ordering, matrix, seed and factor.
Neither a QR factor nor an internal residual certificate is reused as a
physical certificate. Engine, Agent, TaskIR, public Solver and SDK paths are
unchanged; this alternative is under the existing test-only module boundary.

Physical success still requires original positive masses, the unchanged
1e-8 residual gate and strict 1e-10 unit-shape gate, a fresh final operator
certificate, restored-coordinate validation and independent physical JSON
readback. A numerical rejection or operator fault fails this chain: there is
no second physical strategy, fixture-identity dispatch or appended full route.
A successful internal strategy is not proof that a physical proposal will pass
on arbitrary input.

## Work limits and observed receipts

At 256 active degrees of freedom, the internal cap remains two factors and
thirteen certificates; the single physical cap is one factor and seven
certificates. Formal chain caps therefore drop from four/twenty-six to
three/twenty. Observed maxima drop from four factors/sixteen certificates to
three/twelve. Formal limits and observed counts must not be conflated.

| Grid-only bound at size 256 | Independent ranked | Checked-order handoff |
| --- | ---: | ---: |
| Internal maximum reserved visits | 673,546,240 | 673,546,240 |
| Physical maximum reserved visits | 673,546,240 | 338,001,920 |
| Summed maximum reserved visits | 1,347,092,480 | 1,011,548,160 |
| Maximum live numeric grid payload per stage, bytes | 6,914,048 | 6,914,048 |

These are conservative component-visit reservations, not executed instruction
counts. They exclude preparation/eigensolve, banded initialization, ordinary
physical polish, certificate operator work and independent JSON reconstruction.
Stage payloads exclude the request, retained operators, references, vector
headers and allocator overhead. They are not additive peak heap or process RSS.
The internal reservation still exceeds the production 350-million proposal
bound; no production budget has been enlarged or newly qualified.

All six paired outputs retain exact root, initializer seed, internal direction,
physical shape and independent residual bits relative to the independent ranked
route. Actual callbacks, not duplicate nested observer checkpoints, determine
certificate counts. One borrowed banded factor and four substitutions remain
unchanged, as do at most four ordinary physical polish steps.

Cells below list independent-ranked / handoff; factor and certificate terms
are internal+physical.

| Case | Grid factors | Certificates | Reserved grid visits | Independent handoff residual |
| --- | --- | --- | --- | ---: |
| graded-unit | 1+1 / 1+1 | 4+4 / 4+4 | 676,003,840 / 676,003,840 | 3.435873e-9 |
| graded-large | 1+1 / 1+1 | 4+4 / 4+4 | 676,003,840 / 676,003,840 | 3.961312e-9 |
| graded-tiny | 1+1 / 1+1 | 4+4 / 4+4 | 676,003,840 / 676,003,840 | 3.730245e-9 |
| layered-unit | 1+1 / 1+1 | 4+4 / 4+4 | 676,003,840 / 676,003,840 | 4.891325e-9 |
| layered-large | 1+1 / 1+1 | 4+4 / 4+4 | 676,003,840 / 676,003,840 | 4.420681e-9 |
| layered-tiny | 2+2 / 2+1 | 8+8 / 8+4 | 1,347,092,480 / 1,011,548,160 | 8.589527e-9 |

## Paired timing results

Six separate optimized processes each perform one warmup per route and three
fresh samples per route. Pair order alternates independent/handoff,
handoff/independent, independent/handoff. Each member rebuilds its own request,
assembly, Jacobi pair and retained refinement; no shared cached fixture or oracle
seed exists. Every receipt must exactly match its route's fresh baseline.

Recovery time is initializer+internal+physical summed within each sample, then
the median of those three sums. It is not a sum of independently selected phase
medians. Physical and total columns give the individual phase medians.

| Case | Independent recovery ms | Handoff recovery ms | Handoff change | Physical ms, independent / handoff | Fresh total ms, independent / handoff |
| --- | ---: | ---: | ---: | ---: | ---: |
| graded-unit | 43.625 | 44.889 | 2.9% | 21.755 / 22.363 | 655.898 / 667.850 |
| graded-large | 43.697 | 44.851 | 2.6% | 21.818 / 22.337 | 653.959 / 608.020 |
| graded-tiny | 41.278 | 42.289 | 2.4% | 20.458 / 20.976 | 721.881 / 692.420 |
| layered-unit | 42.379 | 42.390 | 0.0% | 21.001 / 21.083 | 712.620 / 725.930 |
| layered-large | 41.580 | 42.179 | 1.4% | 20.690 / 20.855 | 760.441 / 729.678 |
| layered-tiny | 87.619 | 62.289 | -28.9% | 44.218 / 20.891 | 954.672 / 779.524 |

Layered-tiny removes the repeated rejected grid-norm physical fit, changing
four factors to three and recovery median 87.619 to 62.289 ms (28.9% lower).
Its physical-stage median changes 44.218 to 20.891 ms. The new three recovery
samples are 62.316, 62.249 and 62.289 ms; their absence of a long tail does not
prove a tail bound. The earlier campaign's roughly 407 ms ranked tiny-layer
sample remains retained in its historical report, not deleted or mixed into
this paired comparison.

Other cases perform the same two fits and eight certificates. Their recovery
medians are 0.0% to 2.9% slower in this small sample. No statistical significance,
universal speedup or latency SLO is claimed. Fresh total timing is dominated by
Jacobi preparation and must not be attributed entirely to this strategy change.
Process high-water RSS ranges 12,336
to 13,488 KiB, includes both routes and is not incremental heap.

## Fail closed controls

The genuine layered-large and layered-tiny internal successes feed independently
assembled physical proposals. For each case, forcing every physical certificate
to reject consumes one factor/three actual callbacks and returns bounded policy
exhaustion. A fault injected into the fourth, final operator check also consumes
one factor and returns the injected error, even when its text contains a
numerical-rejection phrase. Neither error starts another physical fit.

Actual physical restoration cancellation is triggered at
`ModalRoundoffValidate(5)`. Both cases produce no result, observe exactly one
factor and then replay to exact healthy output bits with independent readback.
Original request-derived seeds, physical seeds and frozen roots stay unchanged.
Eighteen controls across all six cases at amplitudes -2, 0.5 and 2 retain exact
signed/scale-adjusted directions, order tags, work counts, physical output and
checked readback. A separate six-fixture fresh campaign runs independent ranked,
handoff and a fresh handoff replay, checking exact receipts each time.

## Replay and scope

Run from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release modal_banded_handoff_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release modal_banded_handoff_pipeline_cost_graded_unit --locked --offline -- --ignored --nocapture --test-threads=1
```

Run each cost suffix separately and sequentially: `graded_unit`, `graded_large`,
`graded_tiny`, `layered_unit`, `layered_large`, `layered_tiny`. Ignored cost
entries reject Debug builds. Registry inspection alone is not execution.

Existing validation and benchmark evidence gains this narrow comparison. No
new claim or qualified production dimension is added: 226 retained claims,
58/77 coordinate targets and 10/32 scenario targets met remain unchanged.
All six actual public heterogeneous requests still reject; arbitrary topology,
independently rebuilt candidate requests, multimode, complete work/allocation
qualification and production integration remain open. No remote, installed App,
live Agent or Headless study journey was performed in this campaign.

## Verification

The focused optimized campaign passed three tests, failed none and explicitly
ignored six cost entries; each of those six cost entries was subsequently
executed in its own process and passed. The initial rejection-control assertion
used the lower-level error label; it was corrected to the actual bounded-policy
error without changing solver behavior, and the complete focused campaign was
rerun successfully.

| Final source check | Observed result |
| --- | --- |
| Complete optimized Solver library after the type-alias cleanup | 682 passed, 0 failed, 34 explicit cost tests ignored; 88.76 seconds |
| Focused optimized handoff controls | 3 passed, 0 failed, 6 ignored; 15.11 seconds; included in the full count |
| Six isolated optimized paired processes | One requested benchmark passed per process; six actual executions |
| Debug physical rejection/final-fault/cancellation/replay | 1 passed, 0 failed; 14.08 seconds |
| Debug eighteen sign/binary-amplitude controls | 1 passed, 0 failed; 40.22 seconds |
| Tensor source regressions | 21 passed, 0 failed |
| Solver/CLI and Script Runner all-target strict Clippy | Passed with warnings denied |
| Native tensor check and self-test | Passed; 0 structural, 4 maturity and 19 evidence-grade gaps; qualification remains blocked |
| Native operator registry check and self-test | Passed; 59 profiles; registry inspection reports executed=false |
| Documentation inventory and HTML book check | Passed; 26 HTML files; development/shipping manifests stay 3.4.5 |
| Project organization audit and self-test | Passed; source limit 800, document limit 2000, tracked debt 0 |
| Rust formatting and whitespace checks | Passed |

The full-suite and Debug runs repeat existing controls; they do not increase
independent scenario coverage by their raw test counts. Registry cost commands
remain explicit opt-in entries, and the operator checker did not execute them.

## Retained paired receipts

All timing samples, including slower outcomes, are retained below.

### graded-unit

```json
{
  "active_dofs": 256,
  "case": "graded-unit",
  "checked_order_handoff": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 3.4358728797234872e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      667.849833,
      622.753375,
      0.8620410000000001,
      21.688833000000002,
      22.362541999999998,
      0.20525,
      0.33537500000000003,
      618.119208,
      3.9911670000000004,
      0.343333
    ],
    "phase_samples_ms": [
      [
        604.775625,
        558.979333,
        0.8890410000000001,
        22.000125,
        22.677084,
        0.22775,
        0.33537500000000003,
        554.045458,
        4.251333000000001,
        0.346458
      ],
      [
        667.849833,
        622.753375,
        0.837666,
        21.688833000000002,
        22.362541999999998,
        0.20525,
        0.33883300000000005,
        618.119208,
        3.97375,
        0.32087499999999997
      ],
      [
        771.886041,
        727.8730410000001,
        0.8620410000000001,
        20.986792,
        21.961292,
        0.200625,
        0.310958,
        723.226541,
        3.9911670000000004,
        0.343333
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "coordinate_scale": 1,
  "elements": 128,
  "grid_payload_per_stage_bytes": 6914048,
  "handoff_max_grid_certificates": 20,
  "handoff_max_grid_factors": 3,
  "handoff_max_grid_visits_reserved": 1011548160,
  "independent_max_grid_visits_reserved": 1347092480,
  "independent_ranked": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 3.4358728797234872e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      655.897583,
      612.075792,
      0.8514170000000001,
      21.05775,
      21.755,
      0.197542,
      0.318083,
      607.231958,
      4.187625000000001,
      0.336875
    ],
    "phase_samples_ms": [
      [
        623.0607500000001,
        577.74225,
        0.8514170000000001,
        21.830875,
        22.429917,
        0.204084,
        0.31012500000000004,
        572.906875,
        4.187625000000001,
        0.336875
      ],
      [
        655.897583,
        612.075792,
        0.878833,
        20.990792000000003,
        21.755,
        0.19487500000000002,
        0.318083,
        607.231958,
        4.2015,
        0.32291600000000004
      ],
      [
        736.6079169999999,
        692.970417,
        0.836375,
        21.05775,
        21.543583,
        0.197542,
        0.332791,
        688.292709,
        4.00525,
        0.338875
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "internal_order": "GridNorm",
  "phase_order": [
    "total",
    "preparation",
    "initializer",
    "internal",
    "physical",
    "readback",
    "input_validation_assembly",
    "dense_matrix_jacobi",
    "retained_pair_refinement",
    "normalized_directions"
  ],
  "process_peak_rss_after_kib": 12352,
  "process_peak_rss_before_kib": 2944,
  "samples_per_strategy": 3,
  "scope": "test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification",
  "warmups_per_strategy": 1
}
```

### graded-large

```json
{
  "active_dofs": 256,
  "case": "graded-large",
  "checked_order_handoff": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 3.961311840787512e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      608.019875,
      564.1246669999999,
      0.8620410000000001,
      21.560292,
      22.337374999999998,
      0.215958,
      0.320167,
      559.600584,
      3.9891249999999996,
      0.322667
    ],
    "phase_samples_ms": [
      [
        608.019875,
        564.1246669999999,
        0.813917,
        21.026875,
        21.835834,
        0.215958,
        0.311292,
        559.600584,
        3.897375,
        0.314666
      ],
      [
        592.184583,
        546.1234579999999,
        0.9075,
        21.560292,
        23.3835,
        0.207291,
        0.32329199999999997,
        541.2447500000001,
        4.216584,
        0.33775
      ],
      [
        795.8196250000001,
        750.736625,
        0.8620410000000001,
        21.65175,
        22.337374999999998,
        0.22929100000000002,
        0.320167,
        746.1039579999999,
        3.9891249999999996,
        0.322667
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "coordinate_scale": 100000000000000,
  "elements": 128,
  "grid_payload_per_stage_bytes": 6914048,
  "handoff_max_grid_certificates": 20,
  "handoff_max_grid_factors": 3,
  "handoff_max_grid_visits_reserved": 1011548160,
  "independent_max_grid_visits_reserved": 1347092480,
  "independent_ranked": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 3.961311840787512e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      653.959333,
      609.6481249999999,
      0.834167,
      21.030291,
      21.818125000000002,
      0.215167,
      0.320167,
      605.111084,
      3.911583,
      0.33829200000000004
    ],
    "phase_samples_ms": [
      [
        624.169917,
        581.3344999999999,
        0.796667,
        20.578958,
        21.242959,
        0.21475,
        0.319917,
        576.746916,
        3.911583,
        0.355125
      ],
      [
        653.959333,
        609.6481249999999,
        0.834167,
        21.150209,
        22.107167,
        0.217542,
        0.320167,
        605.111084,
        3.877875,
        0.33829200000000004
      ],
      [
        769.6492499999999,
        725.734833,
        0.848625,
        21.030291,
        21.818125000000002,
        0.215167,
        0.329167,
        721.118916,
        3.963375,
        0.32283300000000004
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "internal_order": "GridNorm",
  "phase_order": [
    "total",
    "preparation",
    "initializer",
    "internal",
    "physical",
    "readback",
    "input_validation_assembly",
    "dense_matrix_jacobi",
    "retained_pair_refinement",
    "normalized_directions"
  ],
  "process_peak_rss_after_kib": 12336,
  "process_peak_rss_before_kib": 2944,
  "samples_per_strategy": 3,
  "scope": "test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification",
  "warmups_per_strategy": 1
}
```

### graded-tiny

```json
{
  "active_dofs": 256,
  "case": "graded-tiny",
  "checked_order_handoff": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 3.730244795953667e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      692.41975,
      650.070791,
      0.808208,
      20.504167,
      20.976292,
      0.20637499999999998,
      0.322791,
      645.526792,
      3.9611660000000004,
      0.314542
    ],
    "phase_samples_ms": [
      [
        652.5595,
        610.061625,
        0.808208,
        20.504167,
        20.976292,
        0.206875,
        0.309125,
        605.4767909999999,
        3.9611660000000004,
        0.314042
      ],
      [
        692.41975,
        650.070791,
        0.793125,
        20.484417,
        20.863291999999998,
        0.205709,
        0.322791,
        645.526792,
        3.906,
        0.314542
      ],
      [
        837.343917,
        793.302375,
        0.856791,
        21.021792,
        21.95375,
        0.20637499999999998,
        0.328375,
        788.6228749999999,
        4.003834,
        0.346292
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "coordinate_scale": 1e-10,
  "elements": 128,
  "grid_payload_per_stage_bytes": 6914048,
  "handoff_max_grid_certificates": 20,
  "handoff_max_grid_factors": 3,
  "handoff_max_grid_visits_reserved": 1011548160,
  "independent_max_grid_visits_reserved": 1347092480,
  "independent_ranked": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 3.730244795953667e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      721.88125,
      680.551958,
      0.8057500000000001,
      20.0155,
      20.457625,
      0.205292,
      0.329583,
      675.979583,
      3.944792,
      0.314
    ],
    "phase_samples_ms": [
      [
        620.742166,
        579.258375,
        0.8057500000000001,
        20.0155,
        20.4565,
        0.200459,
        0.336166,
        574.757917,
        3.849584,
        0.314
      ],
      [
        721.88125,
        680.551958,
        0.802833,
        19.861208,
        20.457625,
        0.205292,
        0.311583,
        675.979583,
        3.944792,
        0.315375
      ],
      [
        814.1732499999999,
        770.995084,
        0.806459,
        20.671708,
        21.480249999999998,
        0.217,
        0.329583,
        766.384666,
        3.966916,
        0.31325
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "internal_order": "GridNorm",
  "phase_order": [
    "total",
    "preparation",
    "initializer",
    "internal",
    "physical",
    "readback",
    "input_validation_assembly",
    "dense_matrix_jacobi",
    "retained_pair_refinement",
    "normalized_directions"
  ],
  "process_peak_rss_after_kib": 13360,
  "process_peak_rss_before_kib": 2944,
  "samples_per_strategy": 3,
  "scope": "test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification",
  "warmups_per_strategy": 1
}
```

### layered-unit

```json
{
  "active_dofs": 256,
  "case": "layered-unit",
  "checked_order_handoff": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 4.891324560229968e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      725.9297919999999,
      683.4839579999999,
      0.805458,
      20.50075,
      21.083416,
      0.1865,
      0.308958,
      678.992875,
      3.868542,
      0.314666
    ],
    "phase_samples_ms": [
      [
        661.079083,
        618.503791,
        0.805458,
        20.50075,
        21.083416,
        0.1835,
        0.308958,
        613.997709,
        3.8562499999999997,
        0.340125
      ],
      [
        725.9297919999999,
        683.4839579999999,
        0.797042,
        20.402290999999998,
        21.057541,
        0.1865,
        0.30725,
        678.992875,
        3.868542,
        0.314666
      ],
      [
        789.814208,
        746.122,
        0.815416,
        21.025708,
        21.655458,
        0.193083,
        0.312125,
        741.5835,
        3.9123329999999994,
        0.31333300000000003
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "coordinate_scale": 1,
  "elements": 128,
  "grid_payload_per_stage_bytes": 6914048,
  "handoff_max_grid_certificates": 20,
  "handoff_max_grid_factors": 3,
  "handoff_max_grid_visits_reserved": 1011548160,
  "independent_max_grid_visits_reserved": 1347092480,
  "independent_ranked": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 4.891324560229968e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      712.62,
      670.053,
      0.8215,
      20.557334,
      21.001125,
      0.18512499999999998,
      0.308875,
      665.3311659999999,
      3.86925,
      0.32075000000000004
    ],
    "phase_samples_ms": [
      [
        658.754375,
        616.332459,
        0.8215,
        20.557334,
        20.850458,
        0.190334,
        0.307584,
        611.8597500000001,
        3.8435420000000002,
        0.32075000000000004
      ],
      [
        712.62,
        670.053,
        0.8042499999999999,
        20.574084,
        21.001125,
        0.18512499999999998,
        0.308875,
        665.3311659999999,
        4.083667,
        0.328625
      ],
      [
        823.439084,
        780.201625,
        0.836417,
        20.518583,
        21.695583,
        0.18433300000000002,
        0.31024999999999997,
        775.707125,
        3.86925,
        0.31416700000000003
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "internal_order": "GridNorm",
  "phase_order": [
    "total",
    "preparation",
    "initializer",
    "internal",
    "physical",
    "readback",
    "input_validation_assembly",
    "dense_matrix_jacobi",
    "retained_pair_refinement",
    "normalized_directions"
  ],
  "process_peak_rss_after_kib": 13488,
  "process_peak_rss_before_kib": 2960,
  "samples_per_strategy": 3,
  "scope": "test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification",
  "warmups_per_strategy": 1
}
```

### layered-large

```json
{
  "active_dofs": 256,
  "case": "layered-large",
  "checked_order_handoff": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 4.420681051879533e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      729.6783340000001,
      687.2725419999999,
      0.823834,
      20.366292,
      20.854708,
      0.19720800000000002,
      0.312125,
      682.786209,
      3.848958,
      0.31675
    ],
    "phase_samples_ms": [
      [
        664.2379999999999,
        621.65525,
        0.823834,
        20.7045,
        20.854708,
        0.19720800000000002,
        0.312125,
        617.167584,
        3.845708,
        0.32895800000000003
      ],
      [
        729.6783340000001,
        687.2725419999999,
        0.828667,
        20.366292,
        20.983667,
        0.224667,
        0.30975,
        682.786209,
        3.8605,
        0.31525
      ],
      [
        776.463875,
        734.82225,
        0.815458,
        20.061625,
        20.566750000000003,
        0.195458,
        0.321541,
        730.3340830000001,
        3.848958,
        0.31675
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "coordinate_scale": 100000000000000,
  "elements": 128,
  "grid_payload_per_stage_bytes": 6914048,
  "handoff_max_grid_certificates": 20,
  "handoff_max_grid_factors": 3,
  "handoff_max_grid_visits_reserved": 1011548160,
  "independent_max_grid_visits_reserved": 1347092480,
  "independent_ranked": {
    "grid_visits_reserved": 676003840,
    "independent_readback_residual": 4.420681051879533e-9,
    "internal_checks": 4,
    "internal_factors": 1,
    "phase_medians_ms": [
      760.440625,
      718.660875,
      0.804917,
      20.08025,
      20.689916999999998,
      0.200666,
      0.30874999999999997,
      714.1655420000001,
      3.858334,
      0.313792
    ],
    "phase_samples_ms": [
      [
        663.455375,
        619.3545839999999,
        0.7905,
        21.514875,
        21.591834000000002,
        0.200666,
        0.30874999999999997,
        614.873708,
        3.858334,
        0.313042
      ],
      [
        760.440625,
        718.660875,
        0.809708,
        20.08025,
        20.689916999999998,
        0.197583,
        0.308625,
        714.1655420000001,
        3.86975,
        0.31645799999999996
      ],
      [
        821.741042,
        780.087792,
        0.804917,
        20.041417,
        20.597417,
        0.207041,
        0.392625,
        775.533,
        3.847708,
        0.313792
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "internal_order": "GridNorm",
  "phase_order": [
    "total",
    "preparation",
    "initializer",
    "internal",
    "physical",
    "readback",
    "input_validation_assembly",
    "dense_matrix_jacobi",
    "retained_pair_refinement",
    "normalized_directions"
  ],
  "process_peak_rss_after_kib": 13472,
  "process_peak_rss_before_kib": 2944,
  "samples_per_strategy": 3,
  "scope": "test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification",
  "warmups_per_strategy": 1
}
```

### layered-tiny

```json
{
  "active_dofs": 256,
  "case": "layered-tiny",
  "checked_order_handoff": {
    "grid_visits_reserved": 1011548160,
    "independent_readback_residual": 8.589526570335008e-9,
    "internal_checks": 8,
    "internal_factors": 2,
    "phase_medians_ms": [
      779.52375,
      717.073834,
      0.8182499999999999,
      40.540625,
      20.890667,
      0.199209,
      0.330292,
      712.570666,
      3.881375,
      0.31383300000000003
    ],
    "phase_samples_ms": [
      [
        757.265792,
        694.7445829999999,
        0.8312499999999999,
        40.461375000000004,
        21.023083,
        0.203792,
        0.371708,
        690.026334,
        4.032084,
        0.31383300000000003
      ],
      [
        779.52375,
        717.073834,
        0.817625,
        40.540625,
        20.890667,
        0.199209,
        0.30845900000000004,
        712.570666,
        3.881375,
        0.312667
      ],
      [
        878.888417,
        816.401167,
        0.8182499999999999,
        40.616167000000004,
        20.854833,
        0.195959,
        0.330292,
        811.896708,
        3.858625,
        0.314792
      ]
    ],
    "physical_checks": 4,
    "physical_factors": 1,
    "physical_polish_steps": 4
  },
  "coordinate_scale": 1e-10,
  "elements": 128,
  "grid_payload_per_stage_bytes": 6914048,
  "handoff_max_grid_certificates": 20,
  "handoff_max_grid_factors": 3,
  "handoff_max_grid_visits_reserved": 1011548160,
  "independent_max_grid_visits_reserved": 1347092480,
  "independent_ranked": {
    "grid_visits_reserved": 1347092480,
    "independent_readback_residual": 8.589526570335008e-9,
    "internal_checks": 8,
    "internal_factors": 2,
    "phase_medians_ms": [
      954.6721670000001,
      815.344125,
      0.8274590000000001,
      42.514542000000006,
      44.21825,
      0.212084,
      0.320709,
      806.1597499999999,
      4.645958,
      0.324625
    ],
    "phase_samples_ms": [
      [
        1000.3303750000001,
        815.344125,
        0.8274590000000001,
        138.454792,
        45.461833,
        0.23966600000000002,
        0.331625,
        806.1597499999999,
        8.509833,
        0.341583
      ],
      [
        833.559042,
        749.1575419999999,
        0.8215,
        40.317167,
        43.057541,
        0.203083,
        0.307209,
        744.617708,
        3.9179159999999995,
        0.31375000000000003
      ],
      [
        954.6721670000001,
        866.839292,
        0.886042,
        42.514542000000006,
        44.21825,
        0.212084,
        0.320709,
        861.547208,
        4.645958,
        0.324625
      ]
    ],
    "physical_checks": 8,
    "physical_factors": 2,
    "physical_polish_steps": 4
  },
  "internal_order": "Reverse",
  "phase_order": [
    "total",
    "preparation",
    "initializer",
    "internal",
    "physical",
    "readback",
    "input_validation_assembly",
    "dense_matrix_jacobi",
    "retained_pair_refinement",
    "normalized_directions"
  ],
  "process_peak_rss_after_kib": 12336,
  "process_peak_rss_before_kib": 2944,
  "samples_per_strategy": 3,
  "scope": "test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification",
  "warmups_per_strategy": 1
}
```
