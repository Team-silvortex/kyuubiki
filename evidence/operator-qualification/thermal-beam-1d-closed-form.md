# Thermal Beam 1D Closed-Form Qualification Scope

This evidence packet covers `solve.thermal_beam_1d` for a retained linear
Euler-Bernoulli beam fixture with a single fixed root, one free tip, no
distributed mechanical load, and a constant through-depth temperature gradient.

## Closed-Form Contract

The retained fixture reduces the thermal response to a constant free curvature:

```text
kappa = alpha * gradT / depth
theta_tip = kappa * L
uy_tip = 0.5 * kappa * L^2
```

Because the beam is free to curve after the fixed root, the retained
closed-form case expects near-zero internal shear, moment, stress, and strain
energy apart from floating-point roundoff.

## Qualification Boundary

The retained scope is intentionally narrow:

- linear beam kinematics
- constant thermal expansion
- constant through-depth temperature gradient
- single-member free-curvature fixture
- zero-gradient limit

It does not qualify arbitrary thermal frame assemblies, nonlinear thermal
material behavior, transient heat transfer, plasticity, or buckling.
