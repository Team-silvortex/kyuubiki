# Accuracy Baselines: v1.x Seed Set

This is the first concrete baseline set for `v1.x` numerical trust work.

These cases are intentionally small, hand-checkable, and fast. They exist to
turn the accuracy plan into something that will fail loudly if we regress core
mechanics.

## Current seed set

| Study | Case | Reference metrics | Tolerance | Automation |
| --- | --- | --- | --- | --- |
| `axial_bar_1d` | one-element tensile bar | tip displacement `4.761904761904762e-7`, max stress `100000`, reaction `-1000` | absolute tolerances from solver unit baseline | automated |
| `thermal_bar_1d` | restrained uniform temperature rise | max displacement `0`, max stress magnitude `100800000`, max axial force magnitude `1008000`, max temperature delta `40` | relative tolerance `1e-9` on force/stress magnitudes | automated |
| `spring_1d` | one-dimensional spring chain sample fixture | max displacement `0.09428571428571428`, max force `1200`, node displacements `(0.03428571428571429, 0.09428571428571428)`, element forces `(1200, 1199.9999999999998)` | absolute tolerances from solver unit baseline | automated |
| `spring_2d` | planar spring grid sample fixture | max displacement `0.06339734949589224`, max force `1120.754716981132`, node-2 `(ux, uy)=(0.0509433962264151, -0.03773584905660377)`, element forces `(1120.754716981132, -679.2452830188679, 112.06975399937737)` | absolute tolerances from solver unit baseline | automated |
| `spring_3d` | spatial spring cage sample fixture | max displacement `0.05955868626521211`, max force `803.0108273796119`, top-node `(ux, uy, uz)=(0.037134189113355795, 0.03445543981481482, -0.03132270383761861)`, loaded member forces `(-803.0108273796119, -474.11760144504234, -82.59674462242567)` | absolute tolerances from solver unit baseline | automated |
| `thermal_beam_1d` | free gradient beam sample fixture | max displacement `0.005184000000000001`, max rotation `0.004320000000000001`, max moment `7.275957614183426e-12`, max stress `6.614506921984932e-9`, max temperature gradient `45` | absolute tolerances from solver unit baseline | automated |
| `heat_bar_1d` | two-element conduction gradient with fixed end temperatures | max temperature `100`, max heat flux `1800`, middle-node temperature `60`, element gradient `-40` | absolute tolerances from solver unit baseline | automated |
| `heat_plane_triangle_2d` | two-triangle conduction patch with three fixed temperatures | max temperature `100`, max heat flux `3600`, node-1 temperature `60`, element gradients `(-40, -40)` and `(0, -80)` | absolute tolerances from solver unit baseline | automated |
| `heat_plane_quad_2d` | single quad conduction patch with three fixed temperatures | max temperature `100`, max heat flux `2846.0498941515416`, node-1 temperature `60`, gradient `(-20, -60)` | absolute tolerances from solver unit baseline | automated |
| `beam_1d` | tip-loaded cantilever beam | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `torsion_1d` | torsion shaft sample fixture | max rotation `0.026371308016877638`, max torque `2500`, max stress `20833333.333333332`, tip rotation `0.026371308016877638` | absolute tolerances from solver unit baseline | automated |
| `frame_2d` | tip-loaded cantilever frame member | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `frame_3d` | tip-loaded cantilever space-frame member | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `truss_2d` | three-bar triangular truss | max displacement `1.114463950892853e-6`, max stress `60092.52125773316`, tip `ux=2.380952380952381e-7`, tip `uy=-1.088733463909362e-6` | absolute tolerances from solver unit baseline | automated |
| `truss_3d` | space-frame pyramid sample fixture | max displacement `0.0000015799074540869988`, max stress `74386.37868140468`, top-node `(ux, uy, uz)=(2.897530666749509e-7, 2.897530666749509e-7, -0.0000015258420246488773)`, stressed members `(-74386.37868140468, -63387.6959669619, -63387.6959669619)` | absolute tolerances from solver unit baseline | automated |
| `plane_triangle_2d` | two-triangle square patch | max displacement `1.504347441414315e-6`, max stress `100000`, node-2 `ux=4.714285714285715e-7`, node-2 `uy=-1.428571428571429e-6` | absolute tolerances from solver unit baseline | automated |
| `plane_quad_2d` | single quad plate patch sample fixture | max displacement `5.333507749004975e-7`, max stress `126981.38527836032`, node-2 `(ux, uy)=(2.576145151695419e-7, -4.6700943316053366e-7)`, stress `(12500, -120000)`, shear `3048.7804878048746` | absolute tolerances from solver unit baseline | automated |
| `thermal_plane_triangle_2d` | fully restrained two-triangle thermoelastic patch | max displacement `0`, max stress `50149253.731343284`, max temperature delta `40`, element stress `-50149253.731343284` | absolute tolerances from solver unit baseline | automated |
| `thermal_plane_quad_2d` | fully restrained quad thermoelastic patch | max displacement `0`, max stress `34477611.940298505`, max temperature delta `30`, element stress `-34477611.940298505`, mechanical strain `-3.3e-4` | absolute tolerances from solver unit baseline | automated |
| `thermal_frame_2d` | heated portal frame sample fixture | max displacement `0.0010408174194986581`, max rotation `0.0006805479452054797`, max axial force `24164.383561644005`, max moment `42915.94520547945`, max stress `36971506.84931508`, max temperature delta `35`, max temperature gradient `30` | absolute tolerances from solver unit baseline | automated |
| `thermal_truss_2d` | heated triangular truss sample fixture | max displacement `4.801785714285713e-4`, max axial force `235.84952830143558`, max stress `23584.952830143557`, max temperature delta `40` | absolute tolerances from solver unit baseline | automated |
| `thermal_truss_3d` | restrained uniform temperature rise in a 3D truss member | max displacement `0`, max stress magnitude `100800000`, max axial force magnitude `1008000`, max temperature delta `40` | relative tolerance `1e-9` on force/stress magnitudes | automated |
| `thermal_frame_3d` | restrained thermal space frame with dual gradients | max displacement `0`, max axial force `1.764e6`, max moment `2688`, max stress `1.239e8`, max temperature delta `35`, max temperature gradient `30` | relative tolerance `1e-9` on force/moment/stress magnitudes | automated |

## Source of truth

These baselines are enforced in:

- [accuracy_baselines.rs](../workers/rust/crates/solver/tests/accuracy_baselines.rs)

The first qualification-oriented evidence packet for the 1D closed-form subset
is tracked in:

- [line-field-closed-form-baseline.json](../evidence/operator-qualification/line-field-closed-form-baseline.json)
- [line-field-closed-form-derivation.md](../evidence/operator-qualification/line-field-closed-form-derivation.md)
- [line-field-tolerance-policy.json](../evidence/operator-qualification/line-field-tolerance-policy.json)

Run them with:

```bash
cd workers/rust
cargo test -p kyuubiki-solver --test accuracy_baselines
```

For the qualification-oriented subset, retain a release evidence bundle with:

```bash
make capture-line-field-qualification-release-evidence
```

The moxi 2.0.0 retained bundle is checked in at
[line-field-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/line-field-closed-form-release-evidence.json)
and linked from the qualification release record. Its `promotion_summary`
cross-checks the retained review decision, release record, and four promoted
operator ids, so the four 1D closed-form line-field operators now carry
`qualification` evidence only after the retained packet approves them.

## Why these three first

They are the best first return for `v1.x` accuracy work:

- `axial_bar_1d`
  validates the most basic stiffness/force/reaction path
- `thermal_bar_1d`
  validates the simplest trustworthy thermo-mechanical response
- `beam_1d`
  validates flexural response and a more sensitive operator family

Together they cover:

- direct structural displacement/stress
- one-dimensional spring chain extension/force response
- planar spring extension/force distribution
- spatial spring extension/force distribution
- restrained thermal expansion
- free thermal bending from a through-depth temperature gradient
- pure heat conduction with fixed-temperature interpolation
- pure 2D conduction on a triangular patch
- pure 2D conduction on a quad patch
- bending kinematics and moment/stress response

The second wave extends that trust envelope to:

- frame bending with axial-capable line elements
- space-frame bending and restrained thermal space-frame response
- space-truss pyramid response with stable 3D displacement and axial stress paths
- heated triangular truss response in 2D
- triangulated truss load paths
- restrained thermal truss response in 3D
- constant-strain plane triangle response
- bilinear quad patch stress/displacement response
- restrained thermoelastic patch response in triangle elements
- restrained thermoelastic patch response in quad elements
- heated frame response in 2D with thermal curvature and restrained expansion

## Next baselines

After this seed set, the next best families are:

1. newly introduced families beyond the current seed set

## Related docs

- [accuracy-plan.md](accuracy-plan.md)
- [testing-and-ci.md](testing-and-ci.md)
