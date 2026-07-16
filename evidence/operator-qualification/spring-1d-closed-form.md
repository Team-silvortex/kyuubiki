# Spring 1D Closed-Form Evidence

This evidence applies to `solve.spring_1d` for the retained linear static
series-spring scope.

For a fixed-root spring chain with a single tip load, the series equivalent
stiffness is:

```text
1 / k_eq = sum(1 / k_i)
u_tip = F / k_eq = F * sum(1 / k_i)
extension_i = F / k_i
force_i = F
energy_i = 0.5 * F * extension_i
```

The retained regression
`workers/rust/crates/solver/tests/spring_1d_closed_form.rs` checks a
three-element series chain against that closed form. It also checks the
zero-load boundary where displacements, element forces, and strain energy must
all remain zero.

Scope limits:

- This qualifies the current 1D linear static spring-chain scope.
- It does not claim nonlinear springs, dynamics, contact, or arbitrary 2D/3D
  spring networks.
