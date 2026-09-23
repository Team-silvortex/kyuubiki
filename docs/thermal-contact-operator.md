# Steady planar thermal contact

The existing `solve.heat_plane_triangle_2d` and `solve.heat_plane_quad_2d`
operators accept optional finite-resistance contacts between matching boundary
edges. This is a steady, linear, zero-thickness interface model. The two sides
have independent temperature degrees of freedom, even at identical coordinates.

## Contract and ownership

- Protocol: `HeatPlaneContactInput` / `HeatPlaneContactResult` are serializable
  data types in `kyuubiki-protocol`. They have no execution or engine dependency.
- Operator: the heat solver validates the interface, integrates its local
  matrix, assembles it with bulk conduction, and recovers signed heat transfer.
- Engine: existing operator IDs, typed entrypoints, workflow dispatch and
  lifecycle controls are reused. Only discovery descriptions/tags are extended.
- Headless SDK: existing model payloads carry the contact data unchanged. The
  same-mesh temperature projector keeps the two sides separate by node identity.

The engine does not assemble or interpret the thermal contact law.

## Input

Add this optional array to a heat model with `nodes` and `elements`:

```json
{
  "contact_interfaces": [
    {
      "id": "bond",
      "side_a": [1, 2],
      "side_b": [4, 7],
      "thermal_resistance_m2_k_w": 1.0
    }
  ]
}
```

This is a fragment; the
[complete two-material model](../workers/rust/crates/protocol/fixtures/heat-plane-contact-quad.json)
includes all nodes, elements and exterior temperature conditions. The deliberately
large resistance makes the interface jump easy to verify; it is not a suggested
material property for a real bonded joint.

`side_a` and `side_b` each identify one entire two-node bulk boundary edge.
All four node indices must be distinct and in range. The side endpoints must
coincide within `1e-10` times the edge length; side B may be supplied in reverse
order and is aligned to side A in the returned result. Adjacent elements must
lie on opposite sides and have matching positive thickness, within relative
`1e-12`. Interface area is edge length times that thickness.

The area-specific resistance is in **m^2 K/W**, not total K/W. It must be finite,
strictly positive, and produce representable integration weights. Every contact
needs a nonblank unique ID. Each edge can belong to only one contact and exactly
one bulk element. Internal, duplicated, nonmanifold or unmatched edges fail.
Every component connected through bulk elements and contacts needs a prescribed
temperature. A source-driven body may obtain that support through another body.

Missing or empty `contact_interfaces` retains the original heat model and
serialized result shape. Rust struct literals must initialize the new field;
use `contact_interfaces: vec![]` for a model without contacts. Use shared nodes
for ideal contact; do not encode
ideal contact with zero resistance or clamp an invalid resistance. Use separate
unconnected boundaries for insulation, not an infinite resistance entry.

## Law and discretization

With area-specific resistance `R`, heat flux from A to B is `(T_A - T_B) / R`.
For edge area `A`, define `G = A / R` and the linear edge mass matrix
`M = G/6 * [[2, 1], [1, 2]]`. The four-node contact block is
`[[M, -M], [-M, M]]`. It is symmetric, has zero row sums and a non-negative
quadratic form. Opposite blocks transfer the same heat with opposite sign;
contact resistance does not create an extra heat source.

This is consistent edge integration, not independent endpoint resistors. A
nonuniform jump contributes to both endpoint residuals. The model uses a
conduction-only interface law; see the primary
[MOOSE interface heat-transfer documentation](https://mooseframework.inl.gov/source/interfacekernels/SideSetHeatTransferKernel.html)
for the distinction between interfacial conduction and additional convective
or radiative mechanisms. No MOOSE code or dependency is included here.

## Results and downstream use

Each contact result retains its ID and aligned sides, with `area_m2`,
`average_temperature_jump_k`, `max_abs_temperature_jump_k`,
`heat_flow_a_to_b_w`, `heat_flux_a_to_b_w_m2`, and the two consistent
`nodal_heat_flow_a_to_b_w` contributions. A negative flow means B to A.
Zero net flow can coexist with nonzero, opposing local endpoint flows.
Recovery uses relative solved temperatures, avoiding loss of jump accuracy
from an unrelated large absolute temperature reference.

Do not add contact flow to applied source power or to the legacy bulk
`total_abs_heat_flow_rate` summary. An interface transfers heat internally;
that legacy summary is not the outlet power balance of the coupled model.

Thermal-to-structural transfer must retain unique node IDs on both sides. The
existing SDK projector handles coincident positions with distinct IDs without
averaging the temperatures. Thermal contact does not imply a mechanical bond:
the structural model must define its own connectivity and constraints.

## Numerical acceptance

A small assembled-matrix residual is not sufficient for very stiff contact:
large contact coefficients can mask an inaccurate temperature jump and its heat
flow. Before publishing a result, the operator independently reconstructs heat
balance at each free interface node, combining bulk element transfer, signed
contact endpoint contributions and the applied nodal source.

Bulk terms use `K_ij * (T_j - T_i)` before combining them with contact transfer,
not a second multiplication of the assembled bulk/contact matrix by absolute
temperature. The acceptance residual is the absolute net imbalance divided by
the sum of absolute contributions at that node, with tolerance `1e-8`. Scaled,
compensated accumulation avoids overflow and an arbitrary absolute-watt floor.
Unrelated high-power bodies cannot set the tolerance for a weaker interface.
Prescribed-temperature nodes exchange heat with external reservoirs and are
not required to have zero net flow. Interior bulk equations retain the existing
linear solver's convergence checks.

If the interface check fails, the operator may perform at most two residual
correction solves on the same reduced linear system, then recover and check the
contact fields again. Resistance, materials, supports and the acceptance
tolerance are never changed. Invalid recovery and cancellation propagate
immediately; unresolved balance returns an error identifying the node rather
than a successful study. A later clean request can be retried normally.

The quad profile includes correction iterations in `solver_iterations` and
recomputes `solver_residual_norm` for the final solution. The optional memory/
timing trace records `contact_balance`, or `contact_balance_refinement` when a
correction was needed. These are numerical corrections, not physical time steps.
Models without contacts skip balance allocation, scanning and refinement.

Both heat-plane element types also validate the recovered bulk output before
returning success, whether or not contacts exist. Flux magnitudes use `hypot`;
temperature means use scaled compensated sums; split-quad gradients use
normalized area weights. Element flow-rate products combine the extreme
factors first to avoid a spurious intermediate overflow or underflow.
Unrepresentable node temperatures, fluxes, element flow rates or total flow
rates return a diagnostic instead of nonfinite numbers serialized as `null`.
A nonzero gradient whose conductive flux collapses to zero is rejected; an
exact constant-temperature field remains a valid zero-flow case.

These checks retain the historical output convention: a split quad reports
the area-weighted mean flux vector, and `heat_flow_rate` is its magnitude times
element area and thickness. Summing that element metric is not a signed
boundary-power integral or an independent proof of global conservation. See
the [output-range regression](../reports/heat-plane-output-reliability-20260923.md).

See the [balance and refinement regression](../reports/heat-contact-balance-20260923.md)
for the bounded multi-material and sparse-solver fixtures. This does not turn
arbitrarily high conductivity contrasts into resolvable double-precision models.

## Deployment and limits

Use a rebuilt worker that actually advertises `thermal-contact-resistance`.
An operator ID alone is not feature negotiation with an old binary: older
workers may ignore newly added model fields. Check the executing worker's
capabilities and require matching contact results before accepting a study.
The central/UI catalog alone is not proof that a remote worker was upgraded.

Supported: conforming linear edges, constant isotropic bulk conductivity,
constant positive contact resistance, equal adjacent thickness, steady state.
Not included: automatic contact search, nonmatching/mortar meshes, finite gaps,
3D surfaces, pressure/temperature-dependent contact laws, radiation, transient
storage, opening/closing contact or coupled mechanical interface constraints.
Very stiff contacts can still be ill-conditioned; numerical failure is not
silently replaced with ideal contact. No GUI contact editor is added in this
change. No deployed or installed application is automatically upgraded.

The independent `heat-plane-contact-screening` profile and
[regression report](../reports/heat-plane-contact-20260923.md) cover this feature.
The earlier ideal-interface heat qualification is not extended by this change.
