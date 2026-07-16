# Frame 3D Closed-Form Qualification

Candidate: `frame-3d-closed-form`

Operator: `solve.frame_3d`

Release line: `moxi 2.0.x`

## Fixture

The qualification fixture is a single x-aligned 3D frame member with the root
fully fixed and a transverse `y` load `P` at the free tip. The test uses a
prismatic member with bending about local `z`, so the Euler-Bernoulli closed
form is:

```text
uy = -P L^3 / (3 E Iz)
rz = -P L^2 / (2 E Iz)
Mroot = P L
sigma_b = Mroot / Sz
U = 0.5 * P * abs(uy)
```

The retained regression checks fixed root displacement and rotation, tip
displacement, tip rotation, shear force, bending moment, bending stress,
combined stress, and total strain energy against these formulas.

## Scope

This qualifies the current single-member linear 3D frame cantilever path for
small-strain static bending. It does not claim multi-member frame stability,
geometric nonlinearity, warping, plastic hinges, or dynamic response.
