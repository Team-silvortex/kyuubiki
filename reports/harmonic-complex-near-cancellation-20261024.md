# Harmonic Complex Nonzero Near-Cancellation Recovery

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted
[RHS range](harmonic-complex-rhs-range-20261024.md),
[coefficient normalization](harmonic-coefficient-normalization-range-20261024.md)
and [zero-cancellation](harmonic-complex-zero-cancellation-20261024.md) repairs.
This follow-up extends Solver arithmetic replay, not a release version or
physical model. Engine production code, Agent, TaskIR, input/result contracts
and installed runtimes remain unchanged.

## Reproduced Defect

The previous zero trigger did not protect small nonzero results. Seven new
tests failed before this repair; an eighth test then established the relative
routing boundary. References use exactly representable dyadic inputs:

- With `d = 3*2^-27`, the exact update `1-(1-d)*(1+d)` is `9*2^-54`, but separate
  binary64 multiplication and subtraction returned `8*2^-54`. The result was
  finite, normal and nonzero, with a relative error of `1/9`.
- Complex products erase the same product tail in either real or imaginary
  components. Controls use `d = 3, 5, 7` times `2^-27`, both signs and scales
  `2^-500`, one and `2^500`; the small reference component is checked exactly.
- Complex division with numerator `1+i*(1+d)` and denominator `(1-d)+i` loses
  the imaginary determinant tail while still returning a nonzero component.
  The analytic reference is `-d*d/((1-d)^2+1)` and the comparison allows four
  binary64 epsilons of relative error. Correct rounding is not claimed.
- A complete row starts at `2^-54`, subtracts one, then subtracts 1,023 copies
  of `-1/1024` and a final `-1/1024-2^-30`. Ordinary accumulation returned
  `2^-30`, not `2^-30+2^-54`. No intermediate zero exposed the earlier loss.
  The final update is not locally strongly cancelled relative to `1/1024`,
  but is strongly reduced relative to the row's earlier unit-size peak.
- Manufactured unit-diagonal triangular systems with coupling `1-d` require
  the tiny unknown `d*d`. Both dense and path implementations returned the
  nonzero rounded product difference instead, in forward and backward controls.

These are private arithmetic and manufactured linear-system counterexamples,
not newly reproduced incorrect successful results from public physical models.
Existing public harmonic-force and serialized Engine/workflow targets remain
consumer regressions; they are not relabeled as new physical accuracy evidence.

## Replay Contract

The shared component predicate now selects replay when a result's magnitude is
at most `2^-26` times the relevant nonzero operand scale. This is
`sqrt(binary64 epsilon)`, a conservative routing heuristic for strongly reduced
components, not a physical residual tolerance, singularity floor or error bound.
It includes the previous zero trigger.

Complex products and division compare their component result with the scalar
product magnitudes. Complete-row accumulation additionally retains independent
real and imaginary peaks across earlier sums and product components, so gradual
shrinkage can trigger replay even without a single severe local subtraction.
Once triggered, replay uses the original complete expression and existing
binary-scaled FMA-tail/compensated accumulator, not just the last update.

The ordinary path keeps its floating-point operation order and bits when the
new predicate does not trigger. Existing bit controls and zero/empty-term
controls remain unchanged. Boundary controls at relative `2^-27`, `2^-26` and
`2^-25` verify replay selection and exact value preservation across signed
power-of-two scales. The heuristic may also replay exactly computed values;
it does not assert that every selected expression was inaccurate.

Only scalar peak state and predicates are added. There is no row/matrix copy,
new stage ID, alternate factorization, arbitrary-precision dependency or
persistent state. Pivot floors, coefficient preparation guards, final range
restoration and original-equation admission remain unchanged. Existing range
failures still reject; no automatic retry or pivot relaxation is introduced.
Additional predicates and replay have a cost; no performance neutrality or
speedup is claimed without a separate benchmark.

## Recovery And Validation

Eight new unit tests cover signed/scaled component and determinant tails,
both factorization directions, gradual cancellation, relative routing, late
nonfinite operands and cancellation followed by clean identical-input reuse.
The gradual-row test uses 2,050 terms including trailing zeros and cancels
bounded work in replay entry, both accumulation passes and component traversal.
A nonfinite final operand still fails even after an earlier replay condition;
explicitly correcting that input permits the expected result.

Pinned Rust 1.88.0, locked offline dependencies and existing build directories
are used on macOS AArch64 and Linux x86_64. Linux reuses the preceding overlay
workspace, not a clean Git checkout or an installed-runtime test. The two
preexisting files matched the prior recorded digests before transfer; the new
test file was absent. All three transferred files matched final local SHA-256
digests. No server deployment configuration or credentials changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 817 | 817 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Engine harmonic integration target | 16 | 16 | Existing serialized/workflow regression scope |
| Public complex-force integration target | 7 | 7 | Existing public regression scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets only |
| Total | 1,581 | 1,581 | Not the full workspace or all-physics suite |

The new profile selects eight new near-cancellation units, eight zero-cancellation
units, ten range/bit units and 23 existing public/Engine integration tests.
These 49 tests overlap the table and must not be counted again. Earlier reports
retain their historical source provenance, controls and counts.

The native catalog validates 92 profiles without executing them; the new profile
separately executes all four commands and its 49 selected tests successfully.
macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. The 26-file HTML book, documentation inventory and
800/2,000-line organization audit pass with zero tracked line-limit debt.
Tensor structure and command checks pass for 13 modules and 11 paradigms;
qualification remains blocked with zero structural gaps, two maturity gaps,
23 evidence-grade gaps and 18 P0 gaps. Thresholds and release grades are not
promoted by this bounded evidence.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_near_cancellation
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_cancellation
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_complex_force_range --test harmonic_constraint_admission
```

Profile: `harmonic-complex-near-cancellation-local-reliability`, in the registered
`harmonic-cancellation.json` shard. The separate test file and configuration
remain below the 800-line source/config limit.

## Remaining Boundary

This is bounded harmonic complex near-cancellation recovery, not a forward-error certificate, arbitrary precision, independent physical qualification or durable restart.

The replay threshold does not certify all cancellation patterns or all results
outside its trigger. Compensation is still finite precision. Assembly roundoff,
upstream normalization/factor errors, general conditioning, other complex
operators, independent physical references, scale performance and installed/SDK
journeys remain separate work. Cooperative cancellation and a new same-input
call are not process-kill, power-loss or persisted-continuation evidence.
