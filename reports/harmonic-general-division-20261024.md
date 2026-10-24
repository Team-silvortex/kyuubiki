# Harmonic General Complex Division

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted arithmetic and
[scaled quotient](harmonic-quotient-rounding-20261024.md) follow-ups. Exceptional
general complex division now retains exact integer products until each final
ratio is rounded. This fixes an intermediate range rejection and a public
damped-response quantum error without changing ordinary fast-path arithmetic.
Production changes remain private to Solver; Engine, Agent, TaskIR and operator
contracts are unchanged. No version bump, publication or desktop rebuild is included.

## Failures Before Repair

Two private tests and one public operator test failed before implementation.
For `q=2^-1074`, dividing `1+i*q` by itself incorrectly failed with
`complex arithmetic normalization would lose numerical range`. The denominator
contains `1+q*q`: aligning the tiny squared component to a binary64 mantissa
failed even though the exact final result is `1+0i`. The repaired regression
also checks `MAX+i*q`, both signs of the imaginary component and exact zero
from cancellation.

The second private fixture divides a real unit load by a genuinely complex
divisor near the largest finite exponent. Separately rounding the two numerator
products and squared denominator can move a final component across a subnormal
midpoint. Keeping just the final scaled-scalar quotient accurate is insufficient
when its inputs have already rounded.

A public damped single-spring fixture uses angular frequency one, unit load,
unit mass, stiffness `(2^52+2)*2^971` and damping `(2^52+7)*2^971`. Subtracting
the unit inertial term leaves the represented stiffness unchanged. Let
`C=2^52+2` and `D=2^52+7`. Exact real and negative imaginary displacement
magnitudes in quantum units are `C*2^103/(C*C+D*D)` and
`D*2^103/(C*C+D*D)`. Independent rational midpoint comparisons give rounded
component bit counts `2^50-2` and `2^50-1`.

The amplitude readback derived from those rounded components has bits
`1592262918131441`; the old successful public result had
`1592262918131442`. This is a one-quantum readback difference, not a large
physical-response error. Both satisfy the existing original-equation tolerance.
The amplitude reference follows the public norm of rounded components; it is
not a correctly rounded analytic complex-norm certificate.

## Exact Products And Final Rounding

The cold helper decodes each finite binary64 operand into an integer significand,
a sign and a binary exponent. Two 53-bit significands multiply exactly in
`u128`. Signed pairs form the real numerator, imaginary numerator and positive
squared denominator in fixed integer storage. Product exponents include the
existing scaled RHS metadata and are combined in `i64`.

Each component quotient determines the exact ratio's binary binade by integer
comparison. At most 53 quotient bits are selected by checked integer subtraction;
the remaining numerator is compared with half the divisor for ties-to-even.
Subnormal answers use quantum units directly. Thus neither a floating-point
reciprocal, an independently rounded numerator/denominator nor a restored tiny
squared component lies between these exact products and final rounding.

Each magnitude has 70 `u64` limbs, or 4,480 bits. Finite binary64 product sums
need at most 4,197 bits including carry, with room for quotient alignment.
One signed pair occupies at most 576 bytes; several pairs and scratch magnitudes
coexist, so this is not a total stack-use measurement. Successful arithmetic
uses no heap allocation or new dependency. Extended scaled-RHS metadata remains
subject to the checked fixed capacity, not arbitrary precision. Excess capacity,
true overflow, invalid operands and nonzero answers rounding to zero fail closed.
Exact cancellation can still return zero.

The ordinary fast path, exceptional axial shortcut, pivot floor and
original-equation admission are unchanged. Integer reconstruction and division
cost more work on this exceptional branch; no speedup or performance neutrality
is claimed. This is not a large-model benchmark.

## Regression Coverage

Twelve new test functions cover seven private units, two public Solver tests
and three Engine integration tests:

- Extreme component identities, independent rational midpoint fixtures, ties,
  significand carry, true overflow, nonzero underflow and wide RHS exponents.
- A deterministic 2,048-case comparison across eight binary scales with exact
  small signed integer products. Normal references use one hardware division
  followed by an exact binary shift; near underflow, two exactly scaled normal
  operands are divided once. Production limb decoding is not the reference.
- Shift, carry, borrow, finite-operand and capacity bounds, including a squared
  product pair spanning 4,196 bits and a separate sum-carry control.
- Dense and path single-row readback of the independent component bit counts.
- Public signed loads, reversed element orientation, borrowed/owned equality,
  amplitude bit readback and finite force recovery.
- Byte input/result roundtrips, direct Engine dispatch and input/solve/output
  workflows, with no Engine-owned numerical implementation.
- Cancellation before each exact product pair, between quotient components and
  at bounded quotient-bit checkpoints; later frequency range failure publishes
  no successful result prefix, and explicit fresh reuse succeeds.

Existing generic range tests still exercise the three division phases and the
two preceding row replay phases. Existing ordinary arithmetic bit controls and
all previous scalar quotient tests remain in the selected library scope.
Cooperative cancellation is not forced process termination or durable restart.

## Platform Verification

Pinned Rust 1.88.0 and locked offline dependencies reuse existing build
directories on macOS AArch64 and Linux x86_64. Linux uses the preceding source
overlay, not a clean Git checkout or installed runtime. Before transfer, the
existing arithmetic source matched its preceding SHA-256 digest and the five
new source/test files were absent. All six transferred files matched local
digests. No deployment configuration or credentials changed.

The first Linux library run was interrupted before a result; after confirming
that run had stopped, the same scope was rerun to completion. The table counts
only the completed final-source run. Unrelated server tests were not stopped.

| Completed final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 848 | 848 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Existing Engine constraint integration | 16 | 16 | Selected harmonic scope |
| Previous scalar quotient integrations | 5 | 5 | 3 Engine and 2 public Solver tests |
| New general division integrations | 5 | 5 | 3 Engine and 2 public Solver tests |
| Existing public complex-force integration | 7 | 7 | Selected force scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets |
| Total | 1,622 | 1,622 | Not full workspace, installed or all-physics qualification |

macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. Historical reports retain their original source
provenance, algorithms and counts.

The native catalog validates 96 profiles without execution. The new profile
separately executes its three commands and all 22 selected tests successfully.
The 26-file HTML book, documentation inventory and 800/2,000-line organization
audit pass with zero tracked line-limit debt. Tensor structure and command
checks pass for 13 modules and 11 paradigms; qualification remains blocked with
zero structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_general_division
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_general_division
```

Profile: `harmonic-general-division-local-reliability`, in the registered
`harmonic-cancellation.json` shard. Its three commands select seven new units,
ten existing arithmetic-range units and five new integration tests. These
22 tests are a subset of the table, not additional tests to sum.

## Remaining Boundary

This is bounded exceptional complex-division range and final-rounding hardening, not correctly rounded ordinary fast-path arithmetic, a whole-solver forward-error certificate, independent physical qualification or durable restart.

The cold helper operates on the scaled values it receives; it cannot recover
terms already rounded during assembly, normalization fallback, row accumulation
or factorization. Ordinary fast-path error budgets, independent physical/material
references, exceptional-path scale performance, installed/SDK journeys and
persisted recovery require separate evidence. No qualification threshold or
release grade is promoted by these local numerical controls.
