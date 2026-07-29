# Nonlinear Spring 1D Closed-Form Qualification

Candidate: `nonlinear-spring-1d-closed-form`

Operator: `solve.nonlinear_spring_1d`

Release line: `moxi 2.0.x`

## Fixture

The qualification fixture uses a single hardening spring between a fixed root
and one free loaded tip. The constitutive law is:

```text
F = k u + c u^3
```

with positive linear stiffness `k`, positive cubic stiffness `c`, and a
positive tip load `F`.

## Closed-Form Root

Dividing by `c` gives a depressed cubic:

```text
u^3 + (k / c) u - (F / c) = 0
```

For the monotone hardening case, the real root is:

```text
p = k / c
q = -F / c
u = cbrt(-q / 2 + sqrt((q / 2)^2 + (p / 3)^3))
  + cbrt(-q / 2 - sqrt((q / 2)^2 + (p / 3)^3))
```

The retained regression compares the solver's Newton result against this
closed-form displacement, then checks:

- root displacement and fixed-root displacement
- element extension and force
- tangent stiffness `k + 3 c u^2`
- monotonic load-step factors
- converged residual at every retained load step

## Convergence And Robustness

The active convergence evidence also reruns the Cardano comparison across load,
linear-stiffness, and cubic-stiffness perturbations. Each retained branch checks
the same root, force, tangent stiffness, and load-step convergence properties,
so the qualification is not tied to a single lucky numeric fixture.

The robustness evidence rejects non-finite node coordinates or loads and
zero-length spring elements before Newton iteration starts. These boundaries
keep invalid input from being misclassified as nonlinear nonconvergence.

## Scope

This qualifies the current single hardening spring path for monotone
one-dimensional static response. It does not claim cyclic hysteresis,
softening, snap-through, dynamic nonlinear response, or multi-branch material
laws.
