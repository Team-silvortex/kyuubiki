# Spring and Gap Contact Explicit Tolerance Contract

Date: 2026-10-20

## Reproduced Defect

Both public spring/contact requests accept any finite positive tolerance.
The shared load path nevertheless applied `max(1e-14)` to that tolerance.
Small loads could therefore pass every load step at zero displacement, even
though the reported residual was much larger than the caller's explicit limit.

The initial six-test regression run failed five tests; the omitted-tolerance
control already passed. One failing linear example used:

- Unit scale `s = 2^-50`, stiffness 1, load `8*s`.
- Requested tolerance `s*2^-40`, approximately `8.077935669463161e-28`.
- Returned zero displacement, residual `7.105427357601002e-15` and
  `converged: true` before the repair.

Other failures covered small-load contact activation, hardening, a failed
first correction and rollback after partially completed loading.

## Repair

The solver no longer relaxes explicit tolerances to `1e-14`. Existing
finite/positive input validation and the omitted default `1e-8` remain
unchanged. Protocol field comments now describe the tolerance as an absolute
free-force residual limit and explicitly allow nonconvergence when the
requested precision is unattainable.

The same requested value controls initial equilibrium, each load step,
backtracking acceptance and final convergence. A sufficient-decrease trial
may still be accepted above tolerance as an intermediate Newton state, but it
cannot commit a load step until the tolerance is met. Finite failure retains
the last committed state and reports the failed trial's residual separately.

Only the solver behavior changed in production. No engine special case,
headless routing change, serialization shape change, new retry policy,
iteration-budget increase or load-step increase was introduced.

## Tests

The seven new solver tests cover:

- Signed linear loads at unit scales `2^-50`, `2^-100` and `2^-500`, with
  1, 2, 4 and 8 load steps, through both borrowed and owned entry points.
- Gap-contact force splitting at those scales. Independent solves at each
  target step load are checked against the piecewise analytical equilibrium.
- Scaled cubic hardening at `2^-50`, `2^-100` and `2^-250`, with 1, 4 and
  16 steps, recovering a manufactured equilibrium with and without contact.
- One-correction failures whose residual is below `1e-14` but above the
  requested limit; they must not be reported as successful.
- Partial-contact rollback retaining achieved load factor `0.5`, rather than
  publishing the failed trial as a fully completed solve.
- The unchanged omitted default and large support loads excluded from free
  equilibrium equations.
- Twenty scalar cases with the smallest positive `f64` tolerance. Any success
  must satisfy that limit in the solver's floating-point arithmetic; failures
  remain finite, bounded and uncommitted.

Workflow graph and direct engine results are compared for failure and clean
replay. Rust headless tests use the actual plan/route/engine path. Benchmark
controls reject insufficient-budget small-scale runs and accept adequately
converged replays, rather than counting skipped computation as fast success.

## Verification

Rust 1.88.0, existing offline build caches, local macOS AArch64 and the lab's
isolated Linux x86_64 CI checkout were used. Only changed Rust source/test
files were synced; installed services were not modified.

| Selected group | macOS | Linux |
| --- | ---: | ---: |
| Spring/contact solver suites and mechanical convergence controls | 87 passed | 87 passed |
| Engine workflow suites | 8 passed | 8 passed |
| Rust headless plan-to-engine suite | 11 passed | 11 passed |
| Benchmark failure/replay controls | 3 passed | 3 passed |
| Total distinct selected tests per platform | 109 passed | 109 passed |

Local Clippy with `-D warnings` passed for the solver library and changed
solver, engine and CLI test targets, plus benchmark tests. Workspace formatting
and diff-whitespace checks passed. The 59-profile structural validation passed
without executing every profile command.

Reproduce from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test nonlinear_spring_tolerance --test nonlinear_spring_globalization --test nonlinear_spring_topology --test nonlinear_spring_recovery --test nonlinear_spring_range_reliability --test nonlinear_spring_reliability --test nonlinear_spring_1d_closed_form --test nonlinear_spring_1d_review --test contact_gap_reliability --test contact_gap_1d_closed_form --test contact_gap_1d_refinement --test contact_gap_1d_review --test mechanical_convergence
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --test nonlinear_spring_recovery_workflow --test nonlinear_spring_topology_workflow --test nonlinear_spring_workflow --test contact_gap_workflow
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test nonlinear_spring_range_operator
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-benchmark spring_recovery
```

## Evidence and Limits

The existing spring committed-recovery profile and tensor claim include the
new test/report paths and anchors. Claim IDs, coordinate assignments and
grades are unchanged. This is targeted local numerical-contract evidence,
not a new release-readiness grade.

Tensor structure and command checks passed with 13 modules, 11 paradigms and
no structural gaps. Overall readiness remains blocked, with 2 maturity gaps,
23 evidence-grade gaps and 18 P0 gaps; this repair does not close unrelated
coordinates.

Honoring a requested tolerance does not provide arbitrary precision or a
rigorous real-number error bound. These calculations still use `f64`; an
evaluated zero residual does not establish an exact physical solution.
Overly strict tolerances may require more work or exhaust the bounded search
or Newton budget. Finite-range checks and cancellation remain unchanged.

Full workspace, GUI, Windows, remote Agent transport, large-scale performance
and installed application suites were not rerun. No desktop package was
rebuilt. The benchmark tests above check correctness classification, not
throughput or latency.
