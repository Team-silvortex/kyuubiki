# Native heat / Q9 package chain

Date: 2026-11-11. Host: macOS arm64, source workspace 3.7.8.
Status: bounded, partial validation. No model API calls or credential access.
No installed runtime or GUI replacement; no commit or version bump.

User-facing instructions: [HTML research guide](../docs/research-model-driven.html#q9-package).
This follows the [native Q9 package report](model-research-q9-package-20261111.md);
it does not replace that report's earlier, narrower execution evidence.

## Contract and architecture

The optional `operator.thermal-q9-rect` package now provides
`workflow_bridge.heat_to_thermal_q9_rect_2d` as well as its Q9 solver.
The bridge uses the existing JSON C ABI and generic TaskIR package dispatch.
No Q9 branch or new physical formulation was added to the engine.

The bridge accepts `kyuubiki.heat-to-thermal-q9-nodal/v1`:

- Complete native Q4 heat result, including source input and indexed results.
- Explicit `temperature_unit: kelvin` and finite nonnegative reference temperature.
- A valid, bounded `kyuubiki.thermal-plane-q9-rect/v1` seed model.
- Identical node IDs and coordinates, four oriented heat subcells of equal
  thickness per mechanical Q9 cell. Cyclic corner shifts are accepted, reversed
  orientation or other connectivity is not.

All Q9 nodes, including edge midpoints and centres, are actual heat unknowns.
The bridge does not invent midside temperatures or interpolate unrelated meshes.
Only temperature deltas are replaced; loads, supports and materials are preserved.
The output summary is a directly consumable Q9 input, with no extra diagnostics
inserted into the strict solver model. Source-node or result-array order is not
the joining key: unique identity and consistent source indices are required.

The native Q9 semantic validator is exposed without assembly/solving and reused
by the package. The thermal source correspondence checks remain package-owned.
The portable [JSON schema](../schemas/heat-to-thermal-q9-nodal.schema.json)
describes shape; runtime checks also enforce identities, topology and geometry.
Native heat input retains its existing default fields. The new wrapper/payload,
empty config and Q9 target reject unknown fields; the inherited heat structs are
not being changed to a new strict wire format in this patch.

## Independent reference

Geometry: length 1 m, height 0.25 m, thickness 0.01 m, two equal-height layers.
Young's moduli are 70/20 GPa; Poisson ratios are 0.2/0.35.
The stress-free reference is 293.15 K. Top/bottom temperatures prescribe the
listed temperature rises. Interior heat input temperatures are not a reference
solution: they are free unknowns, initialized to zero in the request.

For layer conductivities k0/k1, the reference gradient follows two thermal
resistances in series: q = (rise_top - rise_bottom) / (0.125/k0 + 0.125/k1).
The two slopes are q/k0 and q/k1; temperature is continuous at the interface.
Section force/moment integrals determine extension and curvature. Closed-form
quadratic-edge traction integrals prescribe the end loads, independently from
the kernel's Gauss integration. Rigid-motion restraints remove three modes.
Analytical displacements and integrated elastic energy provide the comparisons.

This manufactured, interface-aligned solution is piecewise quadratic and
representable by Q9. It is not a general traction-free bending qualification.

## Real Agent measurements

The source-built Agent actually executes native heat RPC, then the bridge and
mechanics through the dynamically loaded package. Both package executions verify
the library digest. The test creates a temporary package directory and loopback
Agent, then cleans up its own process/files. Installed services are untouched.

Heat meshes have twice the mechanical cell count in each direction. The largest
case has 16 x 8 Q9 cells, 32 x 16 Q4 heat cells, 561 shared nodes and 1,119 free
mechanical DOFs. Eight cases passed:

| Q9 mesh | k0/k1 W/(m K) | alpha0/alpha1 1/K | bottom/top rise K | Reference energy J | Absolute energy error J |
| --- | --- | --- | --- | --- | --- |
| 2 x 2 | 10/40 | 1e-5/3e-5 | 0/20 | 0.3030522088353 | 7.771561e-15 |
| 8 x 4 | 10/40 | 1e-5/3e-5 | 0/20 | 0.3030522088353 | 3.053113e-15 |
| 16 x 8 | 10/40 | 1e-5/3e-5 | 0/20 | 0.3030522088353 | 6.388140e-11 |
| 2 x 2 | 10/40 | 1e-5/3e-5 | 10/10 | 0.1265060240964 | 2.275957e-15 |
| 8 x 4 | 10/40 | 1e-5/3e-5 | 10/10 | 0.1265060240964 | 1.137979e-15 |
| 16 x 8 | 10/40 | 1e-5/3e-5 | 10/10 | 0.1265060240964 | 3.170395e-11 |
| 8 x 4 | 40/10 | 3e-5/1e-5 | 20/0 | 0.3480321285141 | 4.179990e-14 |
| 8 x 4 | 1/100 | 5e-6/4e-5 | -10/20 | 1.358138206684 | 1.953993e-14 |

Largest observed nodal temperature error: 5.684342e-14 K.
Largest observed displacement error: 4.366494e-14 m.
Acceptance bounds: 1e-8 K, 1e-10 m and 1e-8 J respectively.
Local regression additionally checks both heat-flux components against the
independent series-resistance result and exact preservation of the seed fields.
No existing Q4 acceptance gate or earlier 4.919177% Q4 result was altered.

## Rejection and recovery

- Before each of the eight live chains, wrong-unit and stale-node bridge tasks
  are rejected. Valid bridge and mechanics calls then work on the same Agent.
- Existing live wrong-library-digest and malformed-Q9 geometry checks still run.
- Package tests cover missing temperature/reference, duplicate IDs/indices,
  mismatched coordinates/load/connectivity, invalid conductivity/thickness/area,
  negative Kelvin, missing node/element collections and malformed target geometry.
- Thermal contact inputs/results and unknown bridge controls are rejected.
- Reindexed heat input, reordered output arrays and cyclic Q4 corner shifts
  produce the same target model through the actual C ABI, including buffer free.

Reproduce from the repository root:

```sh
cargo test --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-operator-thermal-q9
cargo build --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-operator-thermal-q9
cargo test --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test operator_q9_live -- --ignored --nocapture
```

The live test requires permission to bind loopback ports. No model or network
service is required. It is explicitly client-sequenced RPC/TaskIR execution,
not durable WorkflowGraph scheduling, crash recovery or an atomic transaction.

## Remaining boundaries

This validates nodal transfer and selected heat/mechanical solutions, not the
authenticity or correctness of arbitrary user-supplied result JSON. Execution
receipts and independent verification remain necessary for research evidence.
Matching-node transfer does not turn the Q4 heat solver into a Q9 heat solver.
Arbitrary mesh interpolation, interface temperature jumps, curved/distorted cells,
nonlinear materials, dynamic-library cancellation/progress propagation, process
isolation and cross-platform package qualification remain open. The manifest
therefore remains `partial` rather than claiming industrial validation.

## Verification scope

- Package: 1 unit test and 4 integration tests passed, including six numerical
  chain cases within the parameterized integration test and actual C ABI transfer.
- Solver: 29 focused heat, thermal-plane and Q9 integration tests passed.
- Agent: 2 live tests passed, including the eight chain cases above and bad-input
  recovery. Parameter cases and repeated reruns are not counted as distinct tests.
- Strict Clippy passed for the package and CLI, all targets. Scoped Rust formatting,
  JSON syntax and `git diff --check` passed.
- Documentation book/inventory and module topology checks passed. Tensor structure
  and command checks passed; its existing 2 maturity gaps, 23 evidence-grade gaps
  and 18 P0 gaps remain. The new claim is explicitly `partial`, not a global upgrade.
- Version-line audit: 231 exact contracts, zero mismatches. No version changed.

This round did not rerun the complete repository suite, Linux/Windows packages,
installed-app workflows or model-driven reviews.
