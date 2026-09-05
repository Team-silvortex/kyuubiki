# Electric-Conduction Plane Quad 2D Component Evidence

## Scope

`solve.electric_conduction_plane_quad_2d` is a steady, linear, isotropic Ohmic
conduction operator. Each four-node region is represented by two constant-
gradient triangles sharing the `i-k` diagonal. The retained component scope
includes prescribed voltage, nodal current injection, explicit contact
resistance, and finite-impedance voltage terminals.

This evidence is a component-validation input. It is not a release
qualification record and does not imply transient conduction, nonlinear or
anisotropic conductivity, skin effect, induction, thermal feedback inside the
operator, or arbitrary warped/self-intersecting quadrilateral support.

## Analytic Reference

For a rectangular conductor of length `L`, width `W`, thickness `t`, scalar
conductivity `sigma`, and voltage difference `delta_v`, the retained reference
is:

```text
|E| = delta_v / L
|J| = sigma |E|
I = |J| W t
P = I delta_v = sigma |E|^2 L W t
```

The review regression applies this reference before and after a rigid in-plane
rotation. It checks potential gradient, electric field, current density,
injected/extracted current, bulk Joule power, source power, and both power-
balance diagnostics.

## Refinement Reference

A manufactured potential `V(x) = delta_v x / L` is retained on regular
`1 x 1`, `2 x 2`, `4 x 4`, and `8 x 8` quad meshes. Left and right boundaries
are prescribed while interior and insulated-edge nodes remain free. Every
refinement must preserve nodal potential, element field/current density,
terminal current, Joule power, and free-node residual within the documented
test tolerance.

## Boundary Contract

The input regression rejects duplicate identifiers, repeated element nodes,
non-finite material values, degenerate triangles, self-intersecting quad
ordering, globally unanchored systems, and disconnected unanchored islands.
Every topology component must be anchored by a prescribed potential or a
finite-impedance terminal; finite-resistance contacts participate in component
connectivity. Malformed geometry and floating components must fail before
matrix assembly rather than surfacing a borrowed heat-operator diagnostic or
producing an apparently valid result.

The existing solver, Agent RPC, and Engine Workflow tests retain contact and
terminal power partitioning plus the public transport contracts. Promotion
still requires a versioned tolerance policy, independent material/reference
provenance, retained release evidence, and explicit reviewer approval.
