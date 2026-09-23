# Composite layered heat validation, 2026-09-23

## Scope

Local macOS ARM64 regression of the existing Rust headless SDK's three-layer
heat reference, regional source assembly, and actual planar heat solves.
Each layer is 0.03 m wide, the panel is 0.03 m high and 0.001 m thick, the
right boundary is fixed at 35 C, and the remaining exterior is insulated.
Materials meet at shared nodes with ideal temperature continuity.

No thermal contact resistance, interface temperature jump, or general material qualification is claimed.

## Reproduced defects

Ten of the eleven initial SDK regressions failed before the fix. The physical
suite initially had three passing tests and one failed rejection/replay test.

- Cross-validation and mesh convergence divided temperature error by the
  ambient-inclusive peak. With a 1e-12 W dielectric source and conductivities
  `[390, 0.25, 160]` W/(m K), the predicted rise is about 2.00625e-9 K. A result
  with no rise at all was incorrectly reported as passing both checks.
- A positive predicted rise rounded away when added to 35 C could pass as a
  zero-heat solution. A one-ULP temperature error for true zero heat also passed.
- Invalid conductivity in the unheated upstream layer was ignored by the
  analytic calculation, and negative generation could validate as cooling.
- Regional nodal loads could underflow to zero or disappear at a shared node
  behind a larger layer's load. An absolute `max(1 W, total)` comparison hid
  tiny-power losses, while nonfinite totals could bypass the comparison.
- An untrusted `usize::MAX` refinement level panicked during Debug dimension
  arithmetic. Duplicate/out-of-order levels and nonfinite samples were labeled
  missing rather than invalid.

## Corrected contract

The analytic heat reference validates all conductivities, non-negative finite
source powers, and representable resistances and temperature drops. Its error
scale is the analytic temperature rise above the fixed boundary. Zero rise
requires exact equality. A positive rise lost in the absolute-temperature
representation is failed, not approved by increasing a tolerance floor.

The public report shape and tolerance constants remain unchanged. Invalid
references report `fail`, with unavailable error metrics; undefined numeric
reference values remain nonfinite, not fabricated finite temperatures.
Consumers must inspect status before using metrics. Ordinary incomplete level
prefixes remain `missing`; invalid levels/temperatures report `fail`. Unsupported
levels do not enter dimension arithmetic (their count fields are zero).

Distributed/regional refinement builders validate conductivities. Regional
assembly reuses the checked four-node power distributor and verifies actual
increments at every node, each layer's power, and the complete model's power.
Checks use relative power error at 1e-12, without a dimensional absolute floor.
The legacy infallible interface-load fixture builder's signature is unchanged;
invalid requests from it remain subject to the real solver's validation.

## Physical cross-checks

`composite_heat_interfaces` runs real `solve_heat_plane_quad_2d` calls:

- Nine regional cases: conductivity sets `[390, 0.25, 160]`, `[0.5, 20, 2]`,
  `[10, 10, 10]`, crossed with loads `[0.01, 0.02, 0.03]`, `[0.02, 0, 0]`,
  `[0, 0, 0.02]` W. Each runs levels 1, 2, 4, 8 per layer: 36 solves.
- An independent Fourier-law integral checks every nodal temperature. The
  expected cell-center power is upstream power plus the local generated power
  at that position; transverse heat flow is zero. Adding/subtracting half a
  cell's generation reconstructs shared-face power and total outlet power.
- Four more solves check an explicit 0.02 W source at the first interface:
  temperature is continuous, upstream flux is zero, and downstream power
  jumps by exactly the interface source.
- Three solves compare a finest-grid baseline with conductivity and source
  scales of 1e-6 and 1e6. Temperature is unchanged and flux scales accordingly.
- One solve verifies a valid request still works after a rejected source
  distribution. There is no mutable shared solver state to recover or reset.

That is 44 physical solves in four tests, with at most 50 nodes/24 elements.
Node/flux checks allow 2e-8 times the case's characteristic rise/power, not
2e-8 times the ambient temperature. SDK peak cross-check and mesh convergence
thresholds remain 1e-9 and 1e-8 relative to analytic rise, respectively.

## Reproduction

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --test composite_heat_validation_reliability
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test composite_heat_interfaces
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk -p kyuubiki-cli --release --test composite_heat_validation_reliability --test composite_heat_interfaces
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --lib --tests
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-material-explore
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --lib --tests -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test composite_heat_interfaces --no-deps -- -D warnings
./scripts/kyuubiki check-operator-validation --execute --profile thermal-plane-patch --out tmp/composite-heat-interface-validation-20260923.json
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

Observed results:

- All 15 new tests passed in both Debug and Release.
- 270 SDK unit tests, 14 feedback integration tests, and 39 material research
  entry tests passed in Debug.
- All 11 commands in `thermal-plane-patch` passed, including the 18 previous
  heat-to-structural projection regressions. Its opt-in mapping benchmark was
  intentionally skipped; there is no new performance result in this report.
- Strict SDK and scoped CLI Clippy checks passed without warning exemptions.
- Validation-profile, tensor-structure, project-organization and documentation
  inventory checks passed; scoped formatting and diff-whitespace checks passed.

The tensor still reports 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps,
with overall readiness blocked. This evidence does not override those gates.

## Limits

This is a fixed three-layer reference, not an analytic oracle for arbitrary
input geometry, boundary conditions or material layout. Its generation-only
reference does not validate negative heat loads as a cooling model. Equal
four-node source lumping is retained. Floating-point absolute temperatures
still limit resolvable small rises; no higher-precision field is introduced.
No new contact/interface schema, large-mesh benchmark, installed-app check,
remote-platform run, or release qualification is included. The coverage tensor
records bounded numerical evidence only; global readiness is not promoted.
