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

## Section Orientation

`Frame3dElementInput.local_y_axis` optionally defines local section `+Y` as a
global-space vector. The solver projects it normal to the member axis and
derives local `+Z` from the right-handed frame. The vector must be finite and
must not be parallel to the member. Omitting it retains the legacy automatic
axis selection for compatible inputs.

The objectivity regression uses an asymmetric section with `Iy != Iz`, applies
loads and moments in all local directions, and rotates the geometry, loads,
moments, and section axis through three arbitrary rigid rotations. Nodal
translations and rotations transform covariantly; local element forces,
section stresses, scalar maxima, and strain energy remain invariant.

## Convergence And Boundaries

The convergence regression subdivides the cantilever into 1, 2, 4, 8, and 16
elements. Every refinement matches the Euler-Bernoulli tip response, root
actions, section stress, and total energy. Separate perturbations retain the
expected load, inertia, and section-modulus scaling.

Input regressions reject non-finite section axes and axes parallel to the
member before assembly. Protocol round trips retain an explicit axis, while
legacy payloads that omit the optional field continue to decode.

## Scope

This qualifies the current linear 3D frame path for single-member
Euler-Bernoulli cantilever response, mesh subdivision, explicit asymmetric
section orientation, and rigid-rotation objectivity. It does not claim
multi-member frame stability, geometric nonlinearity, warping, plastic hinges,
or dynamic response.
