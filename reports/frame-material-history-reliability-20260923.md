# Frame material-history reliability, 2026-09-23

This is bounded local frame material-history validation, not general cyclic plasticity, fatigue or collapse qualification.

Base revision: `b7ca95a2` (`daji 3.3.5`) plus the working tree, including the
preceding buckling assembly and P-Delta path corrections. Those edits remain
in place. This round does not change the version or install the desktop app.

## Reproduced Defects

Five initial regression tests failed before the fix:

- The bilinear tangent `E*H/(E+H)` underflowed to zero at `E=1e-300` even
  though the correct tangent `5e-302` is representable. Its multiplication
  also overflows at large finite material scales.
- A hardening ratio equal to the largest floating-point number below one,
  with `E=1e300`, formed an unrepresentable `H` and returned `NaN`.
- Perfect plasticity with `E=1e20`, yield strength `1` and strain `+/-1`
  returned zero stress instead of `+/-1` through subtractive cancellation.
- Committed perfect-plastic axial and fiber points reported the elastic
  modulus `1000` instead of a zero loading tangent. Both fixed and adaptive
  fiber quadrature are now covered.

A sixth failing regression reproduced virgin mixed fibers reporting the
parent modulus `1000` rather than the area-weighted modulus `3000`.

The extreme material scales and strains are numerical-range fixtures, not
claims about physically admissible material experiments or full-solver range.

## Corrections and Boundaries

The return map eliminates `H = E*r/(1-r)` algebraically before evaluation.
Plastic strain, backstress and the consistent tangent are computed from `r`
directly. Stress is recovered as the updated backstress plus signed yield
strength. An alternative division order is used if the first plastic-increment
expression overflows. This is not an arbitrary-precision constitutive model.

A zero tangent with accumulated plasticity is retained; only virgin default
points use an elastic fallback. Missing initial fiber histories use each
fiber's own modulus. Area-normalized weights avoid an unnecessary intermediate
area-times-modulus multiplication during effective-tangent recovery.

Section evaluation now returns a checked result. Nonfinite forces, tangents,
summary values, integration error and material history are rejected before
assembly, history commit or reporting. The check includes every adaptive
candidate history, not just the active quadrature rule. Domain errors carry
the element ID. The final recovered tangent is also checked before reporting.
Unrepresentable arithmetic is rejected, not silently changed to JSON `null`.

The existing commit-after-convergence policy is retained and tested after
plastic loading: a failed reversal returns a nonconverged path, the previously
achieved load factor and unchanged accepted material history. An ordinary
fresh solve reproduces the accepted state. This is not persistent restart
or a promise that every arithmetic failure can produce a partial result.

Engine dispatch, Worker/Headless SDK responsibilities and task/result formats
are unchanged. The production fiber-section implementation and its retained
tests are separated to stay within the 800-line source limit. No new Node,
shell, GUI, plugin or remote service dependency is introduced.

## Coverage

- Nine new kernel tests cover common material scaling, near-elastic hardening,
  subtraction cancellation, zero tangent/loading-unloading behavior, mixed
  virgin fibers, reversal finite differences, rejected nonfinite response,
  and corrupted inactive adaptive history.
- Four new public-solver tests cover common stiffness/load/yield scaling
  `1e145`/`1e160`, failed reversal after yielding, owned/borrowed cyclic
  equivalence, and a parallel-member ideal-plastic/elastic load-unload path.
- Three new integration tests construct actual Rust headless plans and use
  the engine route for finite cyclic JSON histories, failed reversal/replay,
  and invalid material rejection followed by an unaffected fresh solve.
- Retained fiber damage, dense quadrature, analytical column, fiber section,
  branch continuation and workflow tests protect adjacent functionality.

## Reproduction

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib frame_2d_material_p_delta::reliability_tests
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test frame_2d_material_p_delta_closed_form -p kyuubiki-cli --test frame_2d_material_operator
./scripts/kyuubiki check-operator-validation --execute --profile frame-material-local-history-reliability --out tmp/frame-material-history-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test frame_2d_material_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification

Local macOS ARM64 results on this working tree:

- Core protocol, solver, engine and headless-SDK libraries: 1153 passed,
  7 existing ignored, no failures.
- All 16 new tests passed in debug and release: 9 kernel, 4 public-solver
  and 3 in-process Rust headless engine-route tests. The release invocation
  also retained 8 pre-existing material-path controls.
- Expanded mechanical regression: 68 passed across 12 targets, including
  the 7 new public/headless tests and 61 retained material, fiber, P-Delta,
  branch-continuation and engine-workflow tests.
- The `frame-material-local-history-reliability` profile executed all 5
  commands successfully, totaling 50 test executions including retained
  analytical, fiber/damage/quadrature and workflow controls.
- Strict solver all-target and CLI integration-test Clippy checks, touched
  source formatting and whitespace checks passed.
- Registry validation passed with 40 profiles. Only the new scoped profile
  was executed here; this is not a rerun of all 40 profiles.
- Tensor structure/command checks, documentation inventory and project
  organization audit passed. Source/document limits remain 800/2000 with
  zero tracked line-limit debt.

The global tensor still reports 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps; the Daji qualification status remains blocked. The local result
does not change those broader release gates.

Only local numerical-validation and recovery evidence is registered in the
coverage tensor. No broad material-family qualification, remote benchmark,
installed-app validation, version bump, commit or deployment is claimed.
