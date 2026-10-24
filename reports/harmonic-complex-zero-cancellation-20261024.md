# Harmonic Complex Zero-Cancellation Recovery

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, with the uncommitted
[RHS range](harmonic-complex-rhs-range-20261024.md) and
[coefficient normalization](harmonic-coefficient-normalization-range-20261024.md)
follow-ups retained. This is Solver arithmetic hardening, not a release bump.
Engine production code, Agent, TaskIR, operator/result contracts and installed
runtimes are unchanged.

## Reproduced Defect

Six of the initial seven new tests failed before the repair; the exact-zero
control passed. All counterexamples use finite normal operands/results rather
than intermediate overflow or subnormal erasure:

- For exactly representable `d = 2^-27` or `2^-30`, binary64 multiplication rounds
  `(1-d)*(1+d)` to one. Subtracting that rounded product from one returned zero
  instead of the independently derived `d*d`.
- Complex products with real terms `(1+d)*(1-d) - 1` erased `-d*d` while retaining
  the large imaginary component. Rotated/signed controls expose the same loss
  in the imaginary component. Powers-of-two scales `2^-500`, one and `2^500`
  preserve the analytic reference without decimal conversion ambiguity.
- Dividing `(1 + i*(1+d))` by `((1-d) + i)` erased the imaginary numerator
  `(1+d)*(1-d)-1`. Its reference quotient is
  `-d*d / ((1-d)^2+1)`; tests use a four-epsilon relative comparison, not a
  claim of correctly rounded division.
- A complete-row numerator that first rounds to zero and then subtracts
  `d*d/2` returned the wrong sign. Replaying only the last subtraction cannot
  repair that earlier lost product tail.
- A row starting at one and subtracting `2^60` then `-2^60` returned zero
  instead of one. A 2,050-term version also lost the original small RHS.
- Manufactured triangular systems with unit diagonals, coupling `1-d` and
  matching `1+d` RHS require one tiny unknown `d*d`. Both dense and path
  factorization paths returned zero in forward/backward controls.

These are private arithmetic and manufactured linear-system counterexamples.
No new public physical model is claimed to have previously published an
incorrect successful result. Existing public harmonic and serialized Engine
fixtures are rerun as consumer regressions, not relabeled as new counterexamples.

## Repair Contract

A component that becomes zero while combining nonzero terms is exceptional,
even when its inputs and intermediate scalar products are normal. Complex
products, updates, RHS updates, complete-row back substitution and division now
route that case through the existing binary-scaled FMA-tail accumulator.
If any intermediate row update triggers this condition, the complete original
row is replayed, including preceding small additions and following terms.

This adds scalar predicates, not a matrix/row copy, alternate factorization,
new persistent state, new stage ID or arbitrary-precision dependency. The
ordinary non-cancelling path retains its operation order and bits. A dedicated
observer control checks that ordinary operations and empty zero terms do not
enter replay; the preexisting bit-for-bit arithmetic control remains unchanged.

True cancellations of nonzero terms also replay, then remain admissible zeros.
No speedup or unchanged throughput is claimed. The existing range checks still
reject irreversibly aligned/restored or truly unrepresentable values. Pivot
floors and complete original-equation admission are unchanged. Other nonzero
cancellation results are not newly guaranteed accurate by this zero trigger.

## Controls And Validation

Eight new unit tests cover signed/scaled real and imaginary tails, determinant
tails, intermediate-zero and prior-addition loss, both factorization directions,
exact zeros, no-replay ordinary paths, and cooperative cancellation followed by
identical-input reuse. The long replay test cancels at entry, bounded interior
work and both accumulation passes/components before a clean subsequent run.
No successful partial solution is returned from a cancelled helper call.

Pinned Rust 1.88.0, locked offline dependencies and existing build directories
are used on both hosts. Linux x86_64 reuses the previous source-overlay workspace,
not a clean Git checkout or an installed runtime. The two existing files matched
their previous recorded digests before transfer; the new test file was absent.
All three transferred files matched final local SHA-256 digests. No deployment
configuration, remote runtime installation or credentials changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 809 | 809 | 9 ignored; 292 `modal_frame_spectrum::` cases excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Engine harmonic integration target | 16 | 16 | Existing serialized/workflow regression scope |
| Public complex-force integration target | 7 | 7 | Existing public regression scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets only |
| Total | 1,573 | 1,573 | Not full workspace or all-physics coverage |

The new validation profile selects eight new units, ten existing complex-range
units and 23 existing public/Engine integration tests. Its 41 selected tests
overlap the table and must not be counted again. Historical reports retain
their own source provenance and counts.

The native catalog validates 91 profiles without executing them; the new profile
separately executes all three commands successfully. macOS Solver/Engine
all-target Clippy with warnings denied, formatting and diff whitespace checks
pass. The 26-file HTML book, documentation inventory and 800/2,000-line
organization audit also pass with zero tracked line-limit debt.
The tensor remains blocked with 13 modules, 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. This bounded evidence
does not change qualification thresholds or promote the release to ready.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_cancellation
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_complex_force_range --test harmonic_constraint_admission
```

Profile: `harmonic-complex-zero-cancellation-local-reliability`, registered in
the catalog through its own shard to keep configuration files below 800 lines.

## Remaining Boundary

This is bounded harmonic complex zero-cancellation recovery, not arbitrary precision, all nonzero cancellation accuracy, independent physical qualification or durable restart.

In-process cooperative cancellation and fresh same-input calls are not process
kill, host power-loss or persisted workflow continuation evidence. No automatic
retry is introduced. General conditioning, assembly roundoff, accurate nonzero
near-cancellation, other complex operators, scale performance, installed/SDK
journeys and independent physical/material references remain separate work.
