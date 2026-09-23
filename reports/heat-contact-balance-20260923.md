# Heat-contact balance and bounded refinement, 2026-09-23

This is local interface-balance screening, not general material or release qualification.

Base revision: `7c82708c` (`daji 3.3.4`); results include this working-tree change.

## Failure reproduced

The eight-node two-material fixture was tested with area-specific contact
resistance `1e-12 m^2 K/W`. The prior quad operator returned success and contact
power `13.33377852574813 W`, while the independent serial-resistance solution is
`13.333333333315556 W`. All output values were finite, but relative error was
about 33 ppm. The first expanded regression failed on that result. Jointly
scaling bulk and contact conductance already passed.

The assembled matrix mixes large contact coefficients with much smaller bulk
terms. Passing its residual check does not independently verify the physically
recovered contact heat flow.

## Correction and ownership

- A solver-private interface ledger reconstructs bulk transfer from local
  temperature differences and combines it with signed contact endpoint power
  and nodal sources. It validates each free contact node at relative `1e-8`,
  without a global-power scale or an absolute-watt floor.
- A normal 2178-node iterative fixture initially exceeded this stricter local
  threshold slightly. The operator now permits at most two residual correction
  solves through the existing generic linear solver and repeats the physical
  check. It does not loosen the gate or replace contact with ideal continuity.
- Matrix coefficients, material properties, boundary conditions and the contact
  law stay unchanged. Nonfinite recovery and cancellation are not treated as
  refinable imbalance. Failure after the bounded corrections remains a failure.
- Engine dispatch, scheduling, protocol schemas and SDK model payloads are
  unchanged. The only shared linear-helper change is crate-local visibility of
  the existing residual-vector routine; it receives no physical-domain logic.
- The quad profile counts additional solver iterations, recomputes final matrix
  residual norm and records the conditional contact-balance timing stage.

The current ledger uses linear-size working storage in the node count and
scans bulk elements. It is only allocated for models with contacts. No runtime
speedup or large-scale memory improvement is claimed.

## Coverage

- Triangle and split-quad material/contact scaling from `1e-12` to `1e12`
  preserves temperatures and normalized heat flow.
- Contact resistances `1e-6`, `1e-9`, `1e-12`, `1e-15` must either return flows
  agreeing with the analytical reference or fail explicitly. This is not a
  promise that all these contrasts converge.
- Serial chains of 2, 3, 8 and 20 materials compare every nodal temperature,
  bulk heat flux and contact power with a resistance-network reference, also
  reversing contact orientation and input ordering.
- Interface nodal sources and sinks of -28, 4 and 28 W are counted exactly
  once, with independently derived interface temperatures and signed bulk/
  contact powers, including flow reversal into a prescribed-temperature wall.
- Two independent prescribed-temperature branches have a `1e12` power ratio.
  A healthy high-power branch cannot hide unresolved contact on the weaker
  branch; replacing only the bad model produces a valid clean replay.
- The sparse fixture has 2178 nodes, 2048 quads, 32 contacts and 2112 free
  temperatures. It exercises Jacobi, symmetric Gauss-Seidel and IC(0), verifies
  contact and bulk flux, and requires the conditional-refinement path to run.
- Unit checks cover relative ledger scaling, finite large-term accumulation,
  nonfinite rejection, exactly bounded retry, invalid-result non-retry and
  cancellation followed by a successful clean replay.
- The existing engine operator route must propagate unresolved balance with a
  node diagnostic, rather than label it a successful study, and accept a later
  valid request.

The 20-material case is one heat operator with a multi-body mesh. It is not
evidence for a 20-stage distributed operator workflow. The sparse fixture is a
local correctness case, not a remote scalability benchmark.

## Reproduction and results

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-cli --test heat_plane_contact --test heat_plane_contact_reliability --test heat_contact_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-cli --test heat_plane_contact --test heat_plane_contact_reliability --test heat_contact_operator
./scripts/kyuubiki check-operator-validation --execute --profile heat-plane-contact-screening --out tmp/heat-contact-balance-20260923.json
./scripts/kyuubiki check-operator-validation --execute --profile thermal-plane-patch --out tmp/thermal-plane-patch-contact-balance-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test heat_contact_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

Local macOS ARM64 results:

- Protocol, solver, engine and Rust headless SDK unit tests: 1119 passed,
  7 existing ignored tests. The solver total includes six new balance and
  refinement-control checks.
- Contact integration tests: 25 passed in debug and 25 passed in release
  (15 existing solver cases, six reliability cases and four engine-route/SDK
  cases per build mode).
- Strict solver all-target Clippy and the CLI contact test target passed with
  warnings denied. Two existing test-only iterator warnings were corrected
  without changing their assertions.
- The updated `heat-plane-contact-screening` profile executed all four commands
  successfully (31 tests). This includes the integration and unit checks above,
  not 31 additional unique cases.
- The existing `thermal-plane-patch` profile executed all 11 commands
  successfully (50 passed, one existing ignored microbenchmark), covering
  no-contact heat/thermal cases and SDK temperature/source projection paths.
- All 35 validation profiles passed the registry check; only the two profiles
  named above were executed here. Tensor structure/command checks, project
  organization and documentation inventory passed.
- The tensor still reports four maturity gaps, 16 evidence-grade gaps and
  11 P0 gaps, with overall daji readiness blocked. This local evidence does
  not promote deployment or general research qualification.

No app installation, remote run, 1M-node test, new contact editor, nonmatching
mesh, 3D surface, transient or pressure-dependent contact law is included.
