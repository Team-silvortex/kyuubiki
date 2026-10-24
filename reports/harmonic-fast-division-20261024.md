# Ordinary Harmonic Complex Division

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted arithmetic and
[exceptional division](harmonic-general-division-20261024.md) follow-ups.
Ordinary complex division now admits a normal candidate only after exact
integer comparisons with its rounding-cell midpoints. Unsupported or ambiguous
cases use checked exact replay. Production changes remain private Solver
arithmetic; Engine, Agent, TaskIR and operator contracts are unchanged.
No version bump, package publication or desktop rebuild is included.

## Failures Before Repair

Two private tests and one public operator test failed before implementation.
For `(2+3i)/(4+5i)`, exact components are `23/41` and `2/41`. The old weighted
reciprocal fast path returned an imaginary component two binary64 ULPs below
the independently rounded ratio. For `1/(2+3i)`, both components were one ULP
away from the references `2/13` and `-3/13`. All operands and answers are normal;
this is not just a subnormal or extreme-contrast problem.

A public grounded single-spring oscillator uses stiffness three, damping three,
unit mass and load, and angular frequency one. Its represented dynamic diagonal
is `2+3i`, so the same rational response reaches the public operator. The norm
of the separately rounded reference components has bits `4598667915111950486`;
the previous successful amplitude had `4598667915111950485`. This is a small
rounding error, not a large physical-response error. Both satisfy the existing
original-equation tolerance. The norm comparison follows the public operation
on rounded components, not a correctly rounded analytic norm certificate.

One existing range test computed its expected division with the same legacy
weighted reciprocal order, preserving that order's one-ULP error for
`(2+0.25i)/(0.75-0.125i)`. It is renamed to
`harmonic_complex_range_preserves_product_update_bits_and_exact_division_reference`.
Its product and update bit assertions remain unchanged; division now compares
with independent exact rational components `94/37` and `28/37`. Live evidence
anchors and profile invariants reflect the intentional division-bit change.
Historical reports retain their earlier source scope and counts.

## Candidate And Exact Admission

The compact path shares binary64 product decoding with the exceptional kernel,
rather than maintaining a second interpretation of signs and exponents. Each
53-bit significand product is exact in `u128`. A pair is eligible only if its
alignment and signed sum fit at most 126 magnitude bits. Strong cancellation
retains the established observed replay path; literal zero input needs no replay
when its denominator fits the compact path.

High/low floating approximations of the integer numerator and denominator yield
a candidate refined by an FMA residual. That refinement is not the acceptance
criterion: even its low part can round. Before returning a nonzero normal value,
the exact integer ratio is compared with both neighboring rounding midpoints.
Equality is admitted only for an even candidate significand. The lower midpoint
uses the narrower spacing below a normal power-of-two boundary, except at the
minimum normal where subnormal spacing is unchanged. The upper midpoint of
the largest finite candidate also remains an exact finite dyadic comparison.

Midpoint comparison uses a checked three-limb `Integer192`, not the 70-limb
long-division loop. A 126-bit denominator times a midpoint of at most 54 bits
fits within 180 bits; shifts and carries still have explicit guards. A small
pair occupies at most 48 bytes and an `Integer192` is 24 bytes. Several values
coexist, so these are type-size bounds, not total stack-use measurements.
Successful arithmetic adds no heap allocation or new dependency.

Capacity failure, an unconfirmed candidate or a subnormal/nonfinite candidate
returns control to existing exact replay, not a numerical success or a new
regularization rule. True overflow and nonzero components rounding to zero
still fail closed. Exact zero sums follow replay's zero convention. Pivot
thresholds, product/update arithmetic and original-equation acceptance do not
change. Ordinary division results are deliberately not bitwise compatible with
the old reciprocal formula when that formula rounded incorrectly.

## Tests And Path Controls

Fourteen new test functions cover nine private units, two public Solver tests
and three Engine integration tests:

- Independent small integer references, signs, real-load response and dense/path
  single-row factorizations.
- A deterministic 2,560-case test across five binary scales. Small signed integer
  products construct exact numerator and denominator references; one hardware
  division and an exact normal binary shift give the expected components.
  All 2,560 fixtures were admitted by the compact path on macOS. The test also
  checks the complete dispatch and requires at least 75 percent compact admission.
- Twelve full-significand input tuples, each repeated at three common binary
  scales. Fixed expected bits come from independent rational evaluation of the
  literal binary64 operands, not the production decoder or heavy kernel.
- Exact ties and adjacent values around one, a significand carry, minimum-normal
  and largest-finite boundaries, plus deliberately forged neighboring candidates.
- Checked 192-bit shift/multiply bounds, small-pair capacity, invalid operands,
  extreme contrast, nonzero underflow, true overflow and subnormal replay.
- No range-replay observations for a normal positive control and empty zero
  input; strong cancellation still enters cancellable replay and reuses cleanly.
- Public signed loads, reversed element orientation, borrowed/owned equality,
  force readback, byte roundtrips and Engine direct/workflow publication.
- Cancellation at row and result boundaries, or a later frequency range error,
  cannot publish a successful prefix; explicit fresh reuse succeeds.

The 2,560-case result is a controlled path-admission observation, not a production
hit rate, a timing benchmark or proof that the compact path is faster than the
old formula. It now performs exact admission work; exceptional contrasts may
use the heavier path more often. Scale performance requires separate measurement.
Cooperative cancellation is not forced process termination or durable restart.

## Platform Verification

Pinned Rust 1.88.0 and locked offline dependencies reuse existing build
directories on macOS AArch64 and Linux x86_64. Linux uses the preceding source
overlay, not a clean Git checkout or installed runtime. Three existing source
files matched their preceding digests before transfer and the six new files
were absent. All nine transferred files matched local SHA-256 digests.
No deployment configuration or credentials changed.

| Completed final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 857 | 857 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Existing Engine constraint integration | 16 | 16 | Selected harmonic scope |
| Previous quotient and general division integrations | 10 | 10 | 6 Engine and 4 public Solver tests |
| New ordinary division integrations | 5 | 5 | 3 Engine and 2 public Solver tests |
| Existing public complex-force integration | 7 | 7 | Selected force scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets |
| Total | 1,636 | 1,636 | Not full workspace, installed or all-physics qualification |

macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. The earlier complex quotient, cancellation and
range controls remain in these selected scopes.

The native catalog validates 97 profiles without execution. The new profile
separately executes its three commands and all 24 selected tests successfully.
The 26-file HTML book, documentation inventory and 800/2,000-line organization
audit pass with zero tracked line-limit debt. Tensor structure and command
checks pass for 13 modules and 11 paradigms; qualification remains blocked with
zero structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_fast_division -- --nocapture
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_fast_division
```

Profile: `harmonic-fast-division-local-reliability`, in the registered
`harmonic-cancellation.json` shard. Its three commands select nine new units,
ten existing range units and five new integration tests. These 24 tests are a
subset of the table, not additional tests to sum.

## Remaining Boundary

This is bounded certified fast-path component rounding for represented harmonic operands, not exact assembly or factorization, an all-input correctness proof, a whole-solver forward-error certificate, independent physical qualification or durable restart.

Exact midpoint admission certifies the rounding cell of a represented component,
not its accuracy relative to a physical model before assembly and elimination.
Previously rounded coefficients or row numerators cannot be reconstructed from
the final divide. Independent material references, all-model forward-error
budgets, large-scale performance, installed/SDK journeys and persisted recovery
still require separate evidence. No qualification threshold or release grade
is promoted by these local controls.
