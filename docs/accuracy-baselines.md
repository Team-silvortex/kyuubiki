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
| `heat_bar_1d` | two-element conduction gradient with fixed end temperatures | max temperature `100`, max heat flux `1800`, middle-node temperature `60`, element gradient `-40` | absolute tolerances from solver unit baseline | automated |
| `beam_1d` | tip-loaded cantilever beam | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `frame_2d` | tip-loaded cantilever frame member | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `frame_3d` | tip-loaded cantilever space-frame member | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `truss_2d` | three-bar triangular truss | max displacement `1.114463950892853e-6`, max stress `60092.52125773316`, tip `ux=2.380952380952381e-7`, tip `uy=-1.088733463909362e-6` | absolute tolerances from solver unit baseline | automated |
| `plane_triangle_2d` | two-triangle square patch | max displacement `1.504347441414315e-6`, max stress `100000`, node-2 `ux=4.714285714285715e-7`, node-2 `uy=-1.428571428571429e-6` | absolute tolerances from solver unit baseline | automated |
| `thermal_truss_3d` | restrained uniform temperature rise in a 3D truss member | max displacement `0`, max stress magnitude `100800000`, max axial force magnitude `1008000`, max temperature delta `40` | relative tolerance `1e-9` on force/stress magnitudes | automated |
| `thermal_frame_3d` | restrained thermal space frame with dual gradients | max displacement `0`, max axial force `1.764e6`, max moment `2688`, max stress `1.239e8`, max temperature delta `35`, max temperature gradient `30` | relative tolerance `1e-9` on force/moment/stress magnitudes | automated |

## Source of truth

These baselines are enforced in:

- [accuracy_baselines.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/solver/tests/accuracy_baselines.rs)

Run them with:

```bash
cd /Users/Shared/chroot/dev/kyuubiki/workers/rust
cargo test -p kyuubiki-solver --test accuracy_baselines
```

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
- restrained thermal expansion
- pure heat conduction with fixed-temperature interpolation
- bending kinematics and moment/stress response

The second wave extends that trust envelope to:

- frame bending with axial-capable line elements
- space-frame bending and restrained thermal space-frame response
- triangulated truss load paths
- restrained thermal truss response in 3D
- constant-strain plane triangle response

## Next baselines

After this seed set, the next best families are:

1. `plane_quad_2d`
2. `heat_plane_quad_2d`
3. `thermal_truss_2d`
4. `thermal_plane_triangle_2d`

## Related docs

- [accuracy-plan.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-plan.md)
- [testing-and-ci.md](/Users/Shared/chroot/dev/kyuubiki/docs/testing-and-ci.md)
