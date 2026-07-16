# Advection-Diffusion Bar Closed-Form Evidence

This evidence applies to `solve.advection_diffusion_bar_1d` for the retained
1D steady, constant-coefficient transport scope.

The retained fixture uses a two-node, one-element bar with fixed inlet and
outlet concentrations. For that compact system, the diagnostic fields have
direct closed forms:

```text
average_concentration = (c_left + c_right) / 2
concentration_gradient = (c_right - c_left) / length
diffusive_flux = -diffusivity * concentration_gradient
advective_flux = velocity * average_concentration
total_flux = diffusive_flux + advective_flux
peclet_number = abs(velocity) * length / (2 * diffusivity)
```

The retained regression
`workers/rust/crates/solver/tests/advection_diffusion_bar_closed_form.rs`
checks both diffusion-dominant and advection-dominant Peclet regimes. It also
checks that the zero-velocity limit reports zero advective flux and that total
flux equals diffusive flux in that limit.

Scope limits:

- This qualifies the current 1D steady constant-coefficient transport bar
  scope.
- It does not claim transient transport, nonlinear reactions, multidimensional
  advection-diffusion, turbulent mixing, or arbitrary stabilization schemes.
