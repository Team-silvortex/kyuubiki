# Contact Gap 1D Closed-Form Qualification Scope

This evidence packet covers `solve.contact_gap_1d` for a retained
one-dimensional linear spring with one penalty stop. The retained fixture has a
fixed root, one loaded tip, one linear spring, and one gap contact on the tip.

## Closed-Form Contract

For the inactive branch, when `F / k < gap`, the contact contributes no force:

```text
u = F / k
contact_force = 0
```

For the active penalty branch, equilibrium is:

```text
k * u + kc * (u - gap) = F
u = (F + kc * gap) / (k + kc)
```

The retained regression checks both branches, contact activation count,
penetration, spring force, contact force, and force-split equilibrium.

## Qualification Boundary

This qualification is intentionally narrow:

- one-dimensional contact
- linear spring element
- penalty normal stop
- static incremental solve
- single contact point

It does not qualify multidimensional contact, friction, impact, large
deformation, nonlinear contact laws, or industrial contact search.
