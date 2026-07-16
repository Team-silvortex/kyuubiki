# Acoustic Bar Closed-Form Evidence

This evidence applies to `solve.acoustic_bar_1d` for the retained 1D
frequency-domain duct scope.

The qualification fixture uses a two-node, one-element acoustic bar with one
fixed pressure node and one volume-velocity source node. For this compact
system, the free pressure degree of freedom is solved from the reduced scalar
system:

```text
p_source = (q * omega - k10 * p_fixed) / k11
```

where:

- `omega = 2 * pi * frequency_hz`
- `stiffness = area / (density * length)`
- `mass = area * length / (6 * bulk_modulus)`
- `dynamic = omega^2 * mass * (1 + damping_ratio)`
- `k10 = -stiffness + dynamic`
- `k11 = stiffness + 2 * dynamic`

The retained regression
`workers/rust/crates/solver/tests/acoustic_bar_closed_form.rs` checks that the
solver matches the scalar closed form at two frequencies. It also verifies:

- angular frequency
- speed of sound
- wave number
- pressure gradient
- particle velocity
- acoustic intensity
- damping loss
- zero loss for an undamped fixture

Scope limits:

- This qualifies the current 1D linear frequency-domain acoustic bar scope.
- It does not claim ducts with branching networks, nonlinear acoustics,
  broadband transient propagation, or 3D acoustic cavities.
