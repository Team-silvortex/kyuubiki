# Heat-plane output representability, 2026-09-23

This is bounded local heat-output validation, not general material or release qualification.

Base revision: `7c82708c` (`daji 3.3.4`), plus the current working tree,
including the preceding thermal-contact balance changes. No version bump or
installed application update is implied.

## Reproduced failures

The initial six regression cases produced five failures and one pass:

- Finite flux components for conductivity `1e-200` became zero magnitude after
  squaring underflowed. A linear field with gradient `(20, -10)` should have
  magnitude approximately `2.2360679774997897e-199`, not zero.
- Squaring large finite components overflowed; successful JSON results
  consequently contained `null` magnitudes, flow rates and maxima.
- Finite nodal values with a representable temperature mean overflowed their
  intermediate sum. The same risk existed in area-weighted quad gradients.
- A truly unrepresentable conductive flux could silently become zero rather
  than fail the operator. The total flow metric could also overflow without
  invalidating success.

## Implementation and boundaries

Heat-plane recovery is shared by the triangle and split-quad operators in
`heat_plane_results.rs`. It uses `hypot`, normalized area weights, scaled
compensated temperature means and an ordered positive three-factor product.
It checks absolute node-temperature restoration, gradient/flux components,
flux magnitude, element flow rate and the total flow-rate metric before a
successful result can be serialized.

The added fallible result collector uses the existing result-stage cancellation
checkpoints, stops at the first error, and drops partially built output. It
does not allocate a temporary vector of per-item `Result` values. Physical
checks remain in the operator; the shared collector contains no heat logic.
There are no engine, transport, protocol-schema or SDK payload changes.

Output conventions are unchanged. Quad gradients and fluxes are area-weighted
means of the two split triangles. `heat_flow_rate` is the flux magnitude times
element area and thickness; its sum is not a signed boundary-power integral.
The preceding contact-node balance check remains separate.

This does not make all finite inputs solvable in double precision or certify
every small contribution against relative rounding error. Unrepresentable
positive flux components and element flow rates must not silently become zero;
exact zero gradients and constant-temperature fields remain supported.

## Coverage

- Seven solver integration cases exercise both element types at conductivities
  `1e-200`, `1`, `1e200`, large representable means, normalized quad weighting,
  large-area/small-thickness flow products, explicit range failures, aggregate
  overflow and constant-temperature zero-flow behavior. These analytical
  patches prescribe nodal temperatures to isolate recovery from convergence.
  The separate contact suite retains free-node and sparse-solver coverage.
- Two CLI integration cases build real Rust headless execution plans, resolve
  their existing engine bridge routes, and check both valid extreme fields and
  failure followed by clean replay. These are in-process route checks, not a
  remote transport or installed application test.
- Three recovery-unit cases cover near-limit means, subnormal constants,
  cancelling terms, positive-product range handling and absolute-temperature
  restoration failure/replay.
- Two fallible-collection unit cases cover cancellation boundaries, early
  failure, no tail visits, partial-result destruction and clean collection.

## Reproduction

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-cli --test heat_plane_output_reliability --test heat_plane_output_operator --test heat_plane_contact --test heat_plane_contact_reliability --test heat_contact_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-cli --test heat_plane_output_reliability --test heat_plane_output_operator --test heat_plane_contact --test heat_plane_contact_reliability --test heat_contact_operator
./scripts/kyuubiki check-operator-validation --execute --profile thermal-plane-patch --out tmp/heat-plane-output-reliability-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test heat_plane_output_operator --test heat_contact_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Results

Local macOS ARM64 verification:

- Protocol, solver, engine and Rust headless SDK unit tests: 1124 passed,
  seven existing ignored tests.
- The five contact/output integration targets above: 34 passed in debug and
  34 passed in release, including nine new integration cases.
- The full scalar reference-invariance integration target: 12 passed.
- Strict solver all-target and both touched CLI test-target Clippy checks
  passed with warnings denied. Touched Rust files pass formatting checks.
- The updated `thermal-plane-patch` profile executed all 15 commands:
  64 passed and one existing ignored microbenchmark. Some checks overlap the
  unit and integration totals above; these are not additive unique counts.
- Validation registry checks passed for all 35 profiles. Tensor structure and
  command checks, project organization and documentation inventory passed.
  Source/doc limits remain 800/2000 lines, with zero tracked line-limit debt.
- Overall tensor readiness remains blocked: four maturity gaps, 16
  evidence-grade gaps and 11 P0 gaps. No general qualification was promoted.

No installed-app, remote transport, cross-platform, large-mesh scalability or
new physical capability qualification is claimed.
