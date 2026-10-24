# Harmonic Original-Equation Residual Enclosure

Date: 2026-10-24

Source base: `70340d07`, retaining the uncommitted harmonic assembly and inertia
precision follow-ups. Package metadata and the installed runtime remain 3.7.1.
No version bump, commit, service deployment or desktop rebuild is included.
The change is private Solver arithmetic; Engine, Agent, TaskIR and the operator
protocol remain separate and unchanged.

## Reproduced False Rejection

A grounded oscillator has represented angular frequency one, stiffness one,
damping one half, load one half, and mass `(1+EPSILON)*2^-1000`. Its independently
rounded complex response is `0.4 - 0.2i`. The exact finite mass perturbs the
continuum response, but not those rounded components. Amplitude controls use
the norm of the represented components, not a claim of exact continuum norm.

The previous original-equation checker rejected both the explicitly supplied
state and public solving because a tiny product tail could not round-trip
through the common row scale. The before-repair unit and public regressions
both report `residual normalization would lose numerical range`. Correct
coefficient assembly and successful elimination were insufficient for admission.

This is an availability defect at the residual gate, not evidence that the old
gate silently accepted a wrong answer. Its fail-closed intent is retained.

## Conservative Range Replay

Rows whose normalized high/low product parts round-trip exactly keep their
existing compensated sums, norm calculation and final admission path. An
irreversible finite alignment marks the row for a separate complete replay;
partially accumulated fast-path sums are never used for its certificate.

Replay revisits original loads, every admitted inertia part, and both incident
element terms. It does not substitute a rounded dynamic diagonal. Each scaled
product part is either exact or bounded by neighboring binary64 values. A
nonzero part that underflows is retained as a signed interval, not erased as an
exact zero. Endpoint additions round outward. Nonfinite operands, invalid
bounds and invalid exponent arithmetic fail closed.

For a replayed row, let `U` be the outward-rounded L1 upper bound of the complex
residual. This bounds its L2 norm without relying on a hypot accuracy promise.
For each four-product term, the maximum of its component absolute lower bounds
is no greater than that term's L2 norm. Summing these and the load's absolute
lower bound downward gives `L`, a lower bound for the original equation scale.
Admission requires finite `L > 0` and an upward-rounded `U/L` no larger than
the unchanged tolerance `(128*EPSILON*free_count).clamp(1e-12,1e-8)`.

An uncertain or invalid certificate is an error, not success. This rule can
conservatively reject a boundary case. Pivot floors, matrix normalization,
coefficient range rules and physical result validation are not relaxed.

Only flagged rows allocate sparse ordered enclosure entries. A row lookup is
binary search; no sorting or matrix snapshot is added. One additional original
term traversal occurs only when some row needs range replay. The ordinary row
state remains at most 56 bytes in the tested build. These are implementation
and type-layout controls, not measured whole-solve performance or memory claims.

## Deliberate Regression Updates

Two live tests previously demanded unconditional range-loss rejection. They now
check independently justified certificates instead:

- The earlier inertia fixture with a tiny stiffness tail accepts its rounded
  `load/stiffness` response, followed by clean request reuse.
- A static two-spring contrast uses stiffnesses `2^p` and `2^-p`, unit signed
  load, and rounded displacements `sign*2^-p` and `sign*2^p`, for p = 500 and
  1000. Each free-row residual is only `sign*2^-2p` against a scale of at least
  two. Erasing the middle displacement or halving the tip displacement still
  fails the backward-error gate. At p = 1000 the full solver still rejects its
  unrepresentable prepared matrix; validating a supplied state does not bypass
  factorization admission.

The earlier original-equation and inertia reports retain their historical
rejection scope. Their live test anchors are updated without relabeling those
older runs as enclosure evidence.

## Verification

Fourteen new test functions cover eight private units, three public Solver
tests and three Engine integrations. Existing contrast and inertia denial
controls are revised as described above rather than removed.

- A signed `2^-1128` integer grid independently checks 3,840 scaled high/tail
  combinations around half-quantum, subnormal and normal transitions, including
  endpoint significands. Nonzero values below the binary64 quantum stay bounded.
- Another independent integer sum checks 128 deterministic 64-term sequences
  in both orders, including negated enclosures.
- Exact-threshold uncertainty, invalid/nonfinite bounds, exponent overflow,
  scale overflow and missing sparse-row state are denial controls.
- Forged amplitude, conjugated phase, changed support values, bad reduction
  maps and erased force states remain rejected. An ordinary-row control checks
  that no range replay occurs when alignment is exact.
- Public response and member force agree with independent references for
  dense/path models, both load signs, element reversal and common powers
  -20, 0 and 50. Borrowed/owned results and serialized workflow readback agree.
- Cancellation at replay entry, after inertia accumulation and after the final
  certificate prevents publication. Fresh complete request reuse succeeds.
  A later invalid frequency cannot return the earlier successful sweep prefix.

Pinned Rust 1.88.0 and locked offline dependencies reuse the macOS AArch64 cache:

| Completed scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 918 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 711 | 1 ignored |
| Selected harmonic integrations and dynamic controls | 111 | Not every integration target |
| Total | 1,740 | Not a full workspace or all-physics run |

The selected integrations include assembly, inertia precision, original
constraint admission, force range, complex division/update/quotient regressions,
frequency range, spring balance/reliability and dynamic closed forms. Existing
common-scale `1e-300` and `1e300` controls pass. All-target Solver/Engine Clippy
with warnings denied passes. Counts overlap earlier reports and are not additive.

The dedicated local validation profile executes eight new units, nine existing
original-equation controls and six public/Engine tests, a 23-test subset of the
table. The source claim belongs to numerical-validation and cooperative-recovery
coordinates; it does not promote release qualification.

Native catalog inspection validates 104 profiles without executing them all.
Book, documentation inventory, project organization, formatting and diff checks
pass. The tensor check retains 13 modules and 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji status remains
blocked. Source/config files stay within 800 lines and documents within 2,000.

This is bounded outward enclosure of range-replayed harmonic original-equation backward error, not a proof of ordinary compensated rows, whole-solver forward accuracy, independent physical qualification or durable restart.

No Linux/Windows, remote scale, installed-payload, forced-termination or power-loss
qualification is claimed. Cooperative interruption and fresh reuse do not prove
durable restart. No global acceptance guarantee is made for extreme contrasts.
