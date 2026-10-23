# Tridiagonal Forward-Work Range Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[row-validation](tridiagonal-row-validation-reliability-20261023.md) and
[back-substitution](tridiagonal-back-substitution-range-reliability-20261023.md)
repairs. No version bump, commit, package or service deployment is included.

Follow-up: [forward-underflow recovery](tridiagonal-forward-underflow-reliability-20261023.md)
extends the trigger to verified loss in tiny finite updates. The overflow-only
trigger and finite-underflow limitation below describe the original iteration;
the follow-up records the current behavior and separately completed validation.

## Reproduced failures

Four new tests failed with `tridiagonal elimination diverged` before this
repair. With `h = 2^1023` and `q` the smallest positive subnormal:

- `A = [[8,-8],[-8,16]]`, `b = [h,h]` has the finite solution
  `[3h/8,h/4]`, but its forward vector contains `2h`.
- `A = [[4,-4],[8,8]]`, `b = [1.5h,-1.5h]` needs an adjacent row swap.
  The forward update overflows although the solution is `[3h/32,-9h/32]`.
- A three-row chain can carry `2h` and `3h` into later rows; a different
  triangular construction cancels an extended `2h` back to `-h/2`. Both
  solutions are finite, so rejecting at the first intermediate is premature.
- Appending an independent unit equation with RHS `q` must preserve its `q`
  response while recovering the first overflowing block. Globally scaling
  the RHS down would erase this independent load.

The controls cover both signs. These are arithmetic constructions, not extreme
physical-load qualification cases.

## Recovery design

The ordinary ordered-FMA path is unchanged. A non-finite forward work entry
triggers one exceptional replay from the original factor-space RHS using the
same immutable factors and adjacent-swap decisions. The failed ordinary work
vector is dropped before allocating the replay workspace.

`chain_tridiagonal_forward_range.rs` stores each work entry as a normalized
f64 mantissa and a separate integer exponent. It does not rescale the whole
load vector. Ordinary representable updates still use their previous FMA;
extended updates align only the current row's terms. An irreversible alignment
returns an error instead of erasing a contribution. Zero couplings do not make
independent weak equations inherit an unrelated strong equation's exponent.

Back substitution shares `linear_substitution`'s range arithmetic through an
extended-RHS entry point. The original finite-row path and its quantized-tail
policy remain intact. Recovery does not refactor, regularize or switch to a
dense matrix. Every candidate still passes the original-equation residual gate,
including candidates computed from equilibrated factors.

The replay adds O(n) work and storage only on the exceptional path. Its work
vector stores one f64/i32 pair per row plus a separate f64 solution vector;
normal workspace storage is released first. This is more workspace than the
ordinary substitution, not a claimed memory optimization.
No unbounded retry or retained snapshot is added, and no latency/RSS benchmark
is claimed. The engine/operator boundary and tolerances are unchanged.

Checked exponent differences/final sums and saturating exponent classification
prevent integer overflow in range restoration. This only widens the supported
exponent range of intermediates; it does not increase mantissa precision.

`TridiagonalForwardRange` is appended as stage 84 after stage 83. Entry,
conversion, forward replay and back substitution are cooperatively cancellable;
chunked observations occur every 64 global steps and at phase boundaries,
ending at `3n`. Back-substitution range rows also retain their stage-83
observations. These are diagnostics, not persisted power-loss checkpoints.

## Controls and limits

Twelve tests were added: eleven numerical/adapter tests and one stage-identity
test. Existing ordinary-path bitwise tests now also forbid forward replay.
Additional controls include:

- Equilibrated factors whose original rows still reject a deliberately
  corrupted saved equation, followed by clean replay.
- Coupled chains of 2, 3, 17 and 65 rows, cross-checked against normal-load
  scaling and a separate dense-LU reference at representable load magnitude.
- Ordinary and pivoted 514-row block systems with cancellation during
  conversion, forward processing, backward processing and final completion,
  followed by exact clean replay and no dense-factor observations.
- An independent erased weak response, a connected irreversibly aligned
  minimum load, true final-response overflow and invalid internal factor/RHS
  shapes. Failure does not modify the reusable factors.
- Extreme integer exponents, invalid mantissas/pivots/operands, an RHS above
  f64 range whose quotient is finite, and an RHS below f64 range whose quotient
  is representable. These controls must return a value or error without panic.

Factor formation and RHS equilibration can still reject range loss before this
recovery is entered. An extremely weak term connected to an extended strong
term may also be conservatively rejected when the local alignment cannot be
reversed. Finite underflow in an otherwise ordinary forward update is not a
new trigger here. This repair therefore does not claim all mathematically
finite systems are recoverable, arbitrary precision or general conditioning.

## Validation

Completed with Rust 1.88.0, offline locked dependencies and existing caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver library excluding `modal_frame_spectrum::` | 639 passed, 9 ignored | 639 passed, 9 ignored |
| Engine library unit suite | 711 passed, 1 ignored | 711 passed, 1 ignored |
| 42 solver/engine/CLI/agent integration targets | 247 passed | 247 passed |
| Total excluding repeated focused runs | 1,597 passed, 10 ignored | 1,597 passed, 10 ignored |

The twelve new tests are included in the solver total, not counted twice.
Integration coverage includes the 10,000-node transient chain and 52 live-agent
cancellation tests. The solver library run excludes 292 modal-spectrum tests;
this iteration did not start the long-running modal matrix. Existing opt-in
benchmarks remain ignored. This is targeted regression, not full-project or
general physical qualification.

Local Clippy passed for all solver and engine targets with `-D warnings`.
Workspace formatting and whitespace checks passed. Changed source/config files
remain below 800 lines and reports below 2,000 lines. No additional build root,
retained numerical dump, application package or server configuration was added.

Native operator validation passed with 72 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. These metadata checks are separate from
the executed numerical tests and do not promote a general maturity claim.

## Reproduction

From the repository root with the pinned Rust toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib -- --skip modal_frame_spectrum:: --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-engine --lib -- --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
cargo fmt --manifest-path workers/rust/Cargo.toml --all -- --check
```

The 42 integration targets and command are the same as in the
[row-validation report](tridiagonal-row-validation-reliability-20261023.md#reproduction).

This is bounded forward-work range recovery, not arbitrary precision, general factorization range, physical qualification or durable restart.
