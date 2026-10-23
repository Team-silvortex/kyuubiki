# Harmonic Frequency-Product Range Recovery

Date: 2026-10-23

Version line: daji 3.6.x, after `0a65f2e8` and the uncommitted
[complex original-equation gate](harmonic-original-equation-admission-20261023.md).
This follow-up addresses coefficients lost before that gate, not another solver
backend or a change to engine/TaskIR/operator formats.

Follow-up: [complex arithmetic range recovery](harmonic-complex-arithmetic-range-20261023.md)
adds selected elimination/substitution and element-force repairs. Validation totals
below record the preceding frequency-product snapshot, not the later test count.

## Reproduced Failures

Three public regression tests failed before production edits:

- At angular frequency `2^-600`, the old `omega * omega` became zero. With
  mass `2^900`, the actual inertia coefficient `2^-300` was still normal.
  The old solve discarded inertia and published half the correct undamped
  displacement, along with zero acceleration. The preceding gate recomputed
  the same zero coefficient and therefore could not detect this defect.
- At angular frequency `2^600`, the intermediate square overflowed, so the old
  solve rejected the frequency. With mass `2^-900` and response near `2^-600`,
  the inertia, velocity, acceleration and forces were all representable.
- At angular frequency `1.5 * 2^-537`, the square partially rounded to two
  minimum-subnormal quanta instead of 2.25 quanta. Multiplication by mass
  `2^800` magnified that quantization error. The old public displacement was
  approximately 90% of the manufactured reference.

These use extreme but valid f64 inputs to expose arithmetic defects. They are
not claims that such frequencies/material units are typical research conditions.

## One Frequency-Product Policy

`harmonic_spring_frequency_range.rs` now owns internal two-factor and squared
frequency products. Ordinary normal products retain their former arithmetic order
and bits. Exceptional products split finite factors into binary mantissas and
exponents, combine all factors, and restore only the final quantity. A non-finite
angular frequency itself remains invalid; only its intermediate square is no
longer required to fit in f64.

The squared product is used for both `omega^2 * mass` and `omega^2 * amplitude`.
The two-factor product is used for `omega * damping` and `omega * amplitude`.
True zero frequency, zero damping and zero response remain zero; finite inputs
are checked before this shortcut so `0 * infinity` is never treated as valid data.

Final coefficients and published velocity/acceleration must fit. Restoration must
be reversible for the computed normalized mantissa, including subnormal outputs.
Complete erasure, partial subnormal restoration loss and true overflow return
errors rather than implicit static approximations or successful zero responses.
This is deliberately conservative; it is not an arbitrary-precision coefficient
representation or a guarantee of exact real-number multiplication. Normalized
mantissa multiplication still has ordinary f64 rounding.

Both path and dense assembly use the same inertia/element coefficient functions.
Original-equation admission recomputes coefficients from angular frequency and
the original mass/damping data; it no longer accepts a precomputed square.
Element force reporting uses the same damping coefficient policy. Error context
identifies the frequency and node/element/response field where applicable.

The complex factorization, residual tolerance, fixed-support contract and finite
result checks remain in place. There is no additional factorization, matrix,
coefficient cache, persistent state or runtime dependency. The existing O(n+e)
coefficient/validation work and memory order are unchanged; no speed improvement
is claimed. The engine remains a consumer of solver success/failure, not a second
owner of numerical range policy.

## New Tests

Fifteen tests comprise five solver units, seven solver integrations and three
engine integrations:

- Bit-for-bit ordinary product controls; recoverable overflow/underflow squares;
  exact quantum restoration, rejected partial/complete quantum loss, true zeros,
  signed operands, non-finite operands and true final overflow.
- Original-equation admission accepts the manufactured full displacement but
  rejects the old half-displacement/zero-inertia answer at both extreme scales.
- Common-mode references at `omega = 2^-600` and `2^600`, with and without damping,
  on a three-free-node path and cycle. They independently specify inertia and
  response from binary powers rather than evaluating an out-of-range square.
  Displacement, phase, velocity, acceleration, grounded-element force and
  borrowed/owned equality are checked.
- The partially rounded square case above, whose acceleration remains normal.
- Unrepresentable final inertia/damping coefficients, partial subnormal loss,
  genuinely overflowing coefficients and unrepresentable published acceleration,
  each followed by a clean solve where appropriate.
- Static models with maximum finite damping still have zero velocity/acceleration;
  high-frequency models with exactly zero response still publish exact zeros.
- Serialized dispatch/workflow parity for recovered damped responses on both
  paths, with finite amplitude/phase/acceleration references.
- Coefficient failure in the second frequency preserves failure context and
  returns no successful partial sweep. Cancellation at final original-equation
  admission of the second frequency also prevents success; clean replay matches
  the baseline through both engine entries and both matrix paths.

## Validation

Pinned Rust 1.88.0, offline locked dependencies and existing build directories:

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed |
| --- | ---: | ---: |
| Solver library excluding `modal_frame_spectrum::` | 776 | 776 |
| Engine library | 711 | 711 |
| Engine harmonic integration target | 7 | 7 |
| New solver frequency-range integration target | 7 | 7 |
| Existing harmonic balance/reliability and dynamic closed-form targets | 23 | 23 |
| Non-overlapping total | 1,524 | 1,524 |

Each platform has zero final failures, ten ignored library tests and 292 filtered
modal cases. The fifteen new tests are included once. Existing resonance, power
balance, independent row scales, cancellation/replay, finite result limits and the
10,000-node shuffled path forest remain green. Local Solver/Engine all-target
Clippy passes with warnings denied.

Native metadata checks pass for 87 validation profiles (`executed=false`); the
new profile's three commands also pass with `--execute`. Their focused reruns
are not counted again. Tensor structure/command checks pass for 13 modules and
11 paradigms, with zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps. Overall readiness remains `blocked`. Organization and
formatting/whitespace checks, the 26-file HTML book and documentation inventory
pass. Source/config files remain below 800 lines and documents below 2,000.

Only six numerical source/test files were synchronized to the existing Linux
test tree, without deleting remote content, deploying services, touching server
credentials/configuration or creating another cache/build root. No application
build, version bump, staging operation or commit was performed. All six remote
source/test SHA-256 hashes match the local final sources. Small generated check
reports remain in ignored `tmp/` storage rather than Git.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib \
  --test harmonic_frequency_range --test harmonic_constraint_admission \
  --test harmonic_spring_balance --test harmonic_spring_reliability \
  --test dynamic_spring_closed_form --no-fail-fast \
  -- --skip modal_frame_spectrum:: --quiet
```

## Limits

This repairs frequency-product construction and publication for harmonic springs,
not all complex elimination or force-postprocessing range issues. It does not
recover an unrepresentable final dynamic coefficient using another matrix format.
Irreversible alignment in the separate original-equation gate can still reject
extreme contrast. The gate and factorization share original f64 inputs and
coefficient helpers, so their agreement is not independent physical assembly;
the manufactured closed forms provide the separate bounded reference here.
No general conditioning/forward-error certificate, higher-precision engine,
performance qualification, Windows execution or durable restart was added.

This is bounded harmonic frequency-product range recovery, not general complex-solver qualification, arbitrary precision, independent physical assembly or durable restart.
