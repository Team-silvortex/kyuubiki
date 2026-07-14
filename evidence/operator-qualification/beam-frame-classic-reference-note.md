# Beam, Frame, And Torsion Classic Reference Note

Status: collecting evidence for `beam-frame-classic`.

This note starts the canonical structural benchmark packet for:

- `solve.beam_1d`
- `solve.torsion_1d`
- `solve.frame_2d`

It is not a qualification claim by itself. The goal is to pin the first
textbook-style reference cases so later regression tests can cite one stable
evidence note instead of rediscovering assumptions from source code.

## Scope

The initial packet should stay within linear, small-displacement, static
structural response:

- Euler-Bernoulli beam bending with constant `E I`
- Saint-Venant torsion of a prismatic shaft with constant `G J`
- 2D frame response assembled from axial and bending components

Do not use this note to justify nonlinear beam behavior, plasticity, large
rotation, contact, shell behavior, or 3D solid stress qualification.

## Canonical Cases

### Cantilever Beam Tip Load

Reference form:

- beam length: `L`
- bending stiffness: `E I`
- transverse tip load: `P`
- tip displacement magnitude: `P L^3 / (3 E I)`
- tip rotation magnitude: `P L^2 / (2 E I)`

Qualification packet requirement:

- record the sign convention used by Kyuubiki for transverse force,
  displacement, and rotation
- compare displacement and rotation against the closed-form values
- retain the model input, solver output, tolerance, and version provenance

### Simply Supported Beam Center Load

Reference form:

- beam length: `L`
- bending stiffness: `E I`
- centered point load: `P`
- maximum displacement magnitude: `P L^3 / (48 E I)`
- maximum bending moment magnitude: `P L / 4`

Qualification packet requirement:

- preserve the support convention and load application point
- compare maximum displacement and moment with documented tolerance
- keep this as an independent load case from the cantilever fixture

### Prismatic Shaft Torsion

Reference form:

- shaft length: `L`
- torsional stiffness: `G J`
- applied torque: `T`
- twist magnitude: `T L / (G J)`

Qualification packet requirement:

- document positive torque and twist direction
- compare twist and reaction torque with documented tolerance
- do not reuse this case for warping torsion or non-circular sections unless a
  separate scope note is added

## Promotion Gate

The `beam-frame-classic` candidate may only move beyond this reference-note
stage when the evidence kit also contains:

- a multi-case regression artifact that runs at least two independent cases
- a force, moment, rotation, and torsion sign-convention note
- retained release provenance for the regression output

