# Harmonic Scaled Quotient Rounding

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted
[RHS range](harmonic-complex-rhs-range-20261024.md),
[coefficient range](harmonic-coefficient-normalization-range-20261024.md),
[zero](harmonic-complex-zero-cancellation-20261024.md),
[near-cancellation](harmonic-complex-near-cancellation-20261024.md),
[aligned accumulation](harmonic-complex-exact-accumulation-20261024.md) and
[binary normalization](harmonic-binary-normalization-20261024.md) follow-ups.
Production changes are private Solver arithmetic. Engine production code,
Agent, TaskIR, operator contracts and installed runtimes are unchanged.
No version bump, publication or desktop rebuild is included.

## Reproduction Before Repair

Two private arithmetic tests and one public harmonic-operator test failed
before the repair. Let `q=2^-1074` be the minimum binary64 quantum and
`e=2^-52`. The exact ratio `(1.5+e)/(1+e)*q` is just below `1.5*q` and
must round to `q`. Previously the mantissa division first rounded to `1.5`,
then the binary restore rounded that midpoint to `2*q`.

For a public example, use load `1+4*e`, stiffness `(1+e)*2^1023`, a grounded
root, one free tip and static frequency zero. Set `D=2^52+1`. The exact
displacement in quantum units is `2^51+1.5-3/(2*D)`. Its nearest bit pattern
is `2251799813685249`, but the old successful public result published
`2251799813685250`. The error is one minimum quantum, approximately
`4.4e-16` relative in this example, not a large physical-response error.
Both values satisfy the pre-existing residual admission tolerance.

These midpoint positions were independently checked using rational arithmetic.
The regression references use explicit expected bits and, separately, one
hardware division of two normal operands constructed by exact binary scaling.
They do not reconstruct expected results with the production quotient helper.

## Bounded Repair

The private scaled-scalar quotient separates finite operands into normalized
mantissas and binary exponents. Exponent metadata is combined in `i64`, so
extreme `i32` metadata cannot wrap. A normalized ratio is strictly between
one half and two; nonzero results outside the bounded exponent window are
rejected before shifts or restoration.

Near underflow, the helper converts the ratio directly to quantum units using
`u128` division and remainder, then applies ties-to-even once. The shift is
bounded to -1 through 52: the shifted numerator needs at most 105 bits and
the shifted denominator at most 54. This also covers the first normal binade
where quantum spacing remains unchanged. Higher exponents retain ordinary
mantissa division followed by binary restore. Literal signed zero is preserved;
nonzero unknowns rounding to zero still fail rather than disappearing silently.
Nonfinite inputs, zero divisors and true overflow remain errors.

Exceptional complex division with a pure-real or pure-imaginary pivot now
divides each scaled component directly. It no longer first squares that pivot
and rounds numerator products separately. Cancellation checkpoints bracket
both components at steps zero, one and two; no component pair is published
after cancellation. Ordinary nonexceptional complex division keeps its previous
fast path. General complex division still rounds its numerator and denominator
accumulations separately; it is not certified correctly rounded.

The helper is a 52-line private module with bounded integer storage, no heap
allocation and no new dependency. Pivot tolerances, original-equation admission,
task formats and recovery policy are not relaxed. No performance neutrality or
speedup is claimed without measurement.

## Tests And Publication Controls

Eleven new test functions cover six private arithmetic/factorization tests,
two public Solver tests and three Engine integration tests:

- Independent midpoint references, signs, ties-to-even, minimum-normal edges,
  normal bit controls, signed zero, invalid operands and extreme exponents.
- A deterministic 3,456-sample signed comparison with one hardware division
  of exactly scaled normal inputs. This is one test function, not 3,456 tests.
- Pure-real and pure-imaginary pivots with independently quantized components,
  plus dense and path single-row solutions.
- Public signed loads, reversed element orientation, borrowed/owned results,
  force readback and exact displacement-amplitude bits.
- Input byte serialization, direct Engine dispatch, input/solve/output workflows
  and result byte roundtrip, retaining the expected amplitude bits.
- Cancellation at each component boundary and a singular second frequency:
  no successful result prefix, reproducible failure and clean explicit reuse.

An existing generic range-cancellation fixture now uses a genuinely complex
pivot rather than a pure-real pivot. Its reference changes from `1e308+0i`
to `5e307-5e307*i`, preserving all five existing replay/division phase checks.
Dedicated axial tests separately cover the new three component checkpoints;
generic phase coverage was not removed to accommodate the shortcut.

## Cross-Platform Verification

Pinned Rust 1.88.0 and locked offline dependencies reuse existing build
directories on macOS AArch64 and Linux x86_64. Linux uses the preceding source
overlay, not a clean Git checkout or installed runtime. Before transfer, the two
existing files matched their preceding digests and the four new files were
absent. All six final transferred files matched local SHA-256 digests.
No deployment configuration or credentials changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 841 | 841 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Existing Engine harmonic constraint integration | 16 | 16 | Selected regression target |
| New Engine quotient integration | 3 | 3 | Serialized/workflow boundaries |
| New public Solver quotient integration | 2 | 2 | Selected harmonic operator |
| Existing public complex-force integration | 7 | 7 | Selected force regression target |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets |
| Total | 1,610 | 1,610 | Not full workspace, all-physics or installed coverage |

macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting
and diff whitespace checks pass. Historical reports retain their own source
provenance, algorithms and counts.

The native catalog validates 95 profiles without executing them. The new
profile separately executes all three commands and its 21 selected tests
successfully. The 26-file HTML book, documentation inventory and 800/2,000-line
organization audit pass with zero tracked line-limit debt. Tensor structure
and command checks pass for 13 modules and 11 paradigms; qualification remains
blocked with zero structural gaps, two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps. This local evidence promotes no qualification threshold or
release grade.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_quotient_rounding
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_quotient_rounding
```

Profile: `harmonic-quotient-rounding-local-reliability`, in the registered
`harmonic-cancellation.json` shard. Its three commands select six new units,
ten existing arithmetic-range units and five new integration tests. These
21 tests are a subset of the table, not additional tests to sum.

## Remaining Boundary

This is bounded scaled-scalar quotient and axial complex-division hardening, not correctly rounded general complex division, a whole-solver forward-error certificate, independent physical qualification or durable restart.

Earlier assembly, normalization fallback and factorization can still introduce
roundoff. General complex numerator/denominator coupling, all-model forward-error
budgets, independent physical/material references, scale performance and
installed/SDK journeys require separate evidence. Cancellation controls are
cooperative in-process tests, not forced termination or power-loss recovery.
