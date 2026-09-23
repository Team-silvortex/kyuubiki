# Harmonic spring reliability, 2026-09-23

This is bounded local harmonic-spring validation, not general mechanical or frequency-domain FEM qualification.

Base revision: `7c82708c` (`daji 3.3.4`) plus the working tree, including
the preceding thermal, multiphysics output, modal and transient reliability
changes. No version bump, installed-app update or remote benchmark is implied.

## Reproduced failures

Three public-solver regression tests failed before the fix:

- With angular frequency `1e100`, mass `1e-200`, stiffness `2`, zero damping
  and force `1e150`, the solver returned finite displacement `1e150` and
  velocity `1e250`, but an infinite acceleration in a successful result.
  Derived fields were not checked before publication.
- With angular frequency `1`, mass `1e-300`, stiffness `2e-300`, zero damping
  and force `1e8`, displacement/velocity/acceleration near `1e308` and element
  force `2e8` are representable. The residual denominator overflowed during
  validation and incorrectly rejected the response.
- A cancellation observer installed around a harmonic solve never fired:
  frequency solving and result recovery had no internal cooperative control
  checkpoints. The test checks the solver's own return, not only the wrapper.

Two additional private numerical-kernel tests reproduced false overflow in
complex division. `(1e308 + i 1e308) / (0.6 + i 0.6)` and
`1e308 / (0.5 + i 0.5)` have finite components and amplitudes, but intermediate
normalization or addition overflowed. These are floating-point range fixtures,
not physically admissible large-displacement material models.

## Changes

The requested samples still solve
`(K - omega^2 M + i omega C) u = f` with real applied force phasors.
Complex tridiagonal and bounded dense elimination live in the internal
`harmonic_spring_linear` module. The engine dispatcher, protocol and Rust
headless action are unchanged; no physical implementation moved into the engine.

Complex matrix row scaling uses the largest component rather than an
unrepresentable complex norm. Division applies bounded denominator weights
before adding numerator terms, with reordered scaling for overflowing
intermediate quotients. This is not an arbitrary-precision arithmetic guarantee.

Residuals use a coefficient scale and incident-displacement scale for each
free row. Their error denominators remain bounded in the tested extreme
cases, and an independent soft equation is not erased by a globally much
larger coefficient. An internal negative control deliberately corrupts a soft
row beside a `1e400` stiffness contrast and confirms that validation rejects it.
Unrepresentable load normalization is rejected rather than silently certified.

Node displacement, velocity and acceleration amplitudes and element extension
and force amplitudes must all be finite. Frequency failures carry a zero-based
sample index and its value. A failed or cancelled sweep cannot return previously
computed frames as partial success. Input order and duplicate samples remain
unchanged; a peak tie retains the last matching sample, as before.

Cooperative checkpoints cover layout mapping, assembly, both numerical routes,
row validation, per-frequency execution, node/element recovery and summaries.
`HarmonicSweep` and `HarmonicResidual` are appended without changing existing
stage numeric codes. Initial input checks and path-forest recognition are
still synchronous passes; no strict cancellation-latency bound is claimed.

## Coverage and limits

- Thirteen new solver integration tests cover overflow rejection, finite-range
  preservation, zero frequency, shrinking damping at resonance, singular
  middle-frequency failure, sample order, owned/borrowed agreement, heterogeneous
  independent components and cancellation/replay.
- A two-free-node path and a three-free-node cycle independently check complex
  response against an analytical graph-mode decomposition over five frequencies.
  They exercise tridiagonal and dense routes respectively. The cycle's repeated
  relative mode and damped common/relative resonances are included.
- Both networks check average applied power against damping dissipation:
  `P_in = -0.5 omega sum(f_i Im(u_i))` and
  `P_damp = 0.5 sum(c_e omega^2 abs(u_j-u_i)^2)`.
- Cancellation is injected at eight factorization/validation/result stages,
  plus the second frequency after one frame has completed. A fresh execution
  matches the uninterrupted baseline. No durable checkpoint continuation is added.
- Four CLI integration tests build Rust headless plans, resolve the real engine
  route and exercise finite resonance output, derived-field errors, singular
  failure/correction and cancellation/replay. These run in process, not over
  remote Agent transport.
- Three new private unit tests cover complex-division range and the corrupted
  soft-row negative control. Existing validation retains adjacent pivoting,
  common scale invariance, invalid input and a shuffled 10,000-node path forest.

This is a linear, lumped-mass, one-dimensional spring/damper model with fixed
constraints and real load phasors. It does not add harmonic 2D/3D solids,
complex applied loads, nonlinear damping, fatigue, adaptive frequency search
or a conditioning certificate. `peak_frequency_hz` is a maximum among supplied
samples, not the true continuous resonance peak. Non-path networks retain the
512-free-DOF dense limit. Arbitrary subnormal products, extreme cancellation
and all coefficient scales are not certified. Large-scale overhead and remote
recovery have not been benchmarked again in this round.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test harmonic_spring_balance -p kyuubiki-cli --test harmonic_spring_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib harmonic_spring
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test harmonic_spring_balance -p kyuubiki-cli --test harmonic_spring_operator
./scripts/kyuubiki check-operator-validation --execute --profile harmonic-spring-local-reliability --out tmp/harmonic-spring-20260923.json
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test mechanical_convergence --test transient_spring_balance --test modal_spectrum_reliability
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test harmonic_spring_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification results

Local macOS ARM64 results:

- Core protocol, solver, engine and headless-SDK library tests: 1136 passed,
  7 existing ignored, no failures.
- All 20 new tests passed in debug and release builds: 3 private kernel tests,
  13 solver integration tests and 4 headless engine-route tests. The filtered
  kernel command also runs one existing harmonic test.
- The scoped `harmonic-spring-local-reliability` profile executed all 5 commands
  successfully, totaling 31 test executions including retained controls.
- Adjacent mechanical convergence, modal spectrum and transient-spring balance:
  41 passed.
- Strict solver all-target and CLI integration-test Clippy checks, touched-file
  formatting and whitespace checks passed.
- Profile registry validation passed with 37 profiles. This is registry
  validation, not an execution of all 37 profiles in this round.
- Tensor structure/command checks, project organization audit and documentation
  inventory passed. Source/document limits remain 800/2000 with zero tracked
  line-limit debt. The harmonic entry/validation file is 542 lines and its
  private complex linear-algebra module is 338 lines.

The new tensor claim is local `verified` evidence for numerical validation and
recovery only. Global tensor status remains blocked: 4 maturity gaps,
16 evidence-grade gaps and 11 P0 gaps. No operator-family promotion or closure
of unrelated coverage gaps is implied. No app reinstall, remote deployment,
large benchmark, version bump or Git commit was performed.
