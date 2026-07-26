# Cohesive Interface 1D Screening Evidence

Operator: `solve.cohesive_interface_1d`

Release line: `moxi 2.0.x`

## Retained model

The component operator evaluates a scalar Mode-I bilinear traction-separation
law over an ordered separation history. Its contract exposes initial tensile
stiffness, independent compressive closure stiffness, peak traction, failure
separation, and every retained separation step.

Damage starts at:

```text
delta_0 = peak_traction / initial_stiffness
```

For historical maximum opening `kappa` between onset and failure, the retained
damage law is:

```text
d = delta_f * (kappa - delta_0)
    / (kappa * (delta_f - delta_0))
```

Opening traction is `(1 - d) K delta`. New monotonic softening follows the
linear envelope from peak traction to zero, while unloading and reloading use
the damaged secant stiffness. Damage and maximum opening never decrease.
Negative separation uses the independent compressive penalty stiffness and
does not heal tensile damage.

## Retained checks

- the onset, midpoint, and failure points match the bilinear closed form
- the softening tangent matches the derivative of the active envelope
- unloading and reloading freeze damage and maximum opening
- complete tensile failure carries zero opening traction
- a failed interface still carries compressive closure traction
- empty, non-finite, non-positive, and degenerate contracts are rejected
- the same request crosses protocol serialization, Agent RPC, engine workflow,
  Rust headless route discovery, and the self-hosted Web submission API

## Scope boundary

This is a real history-dependent constitutive interface operator, not a visual
placeholder. It is not yet a finite-element interface formulation: there is no
independent 2D/3D displacement jump interpolation, mixed-mode coupling,
frictional post-failure contact, or assembly into the frame fiber section.
Those remain separate gates before claiming laminate delamination simulation.
