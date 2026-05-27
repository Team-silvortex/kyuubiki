# Accuracy Plan: v1.x

This document defines the accuracy-validation path for `v1.x`.

The goal is simple:

- `v0.9.0` proved that Kyuubiki is broadly usable
- `v1.x` should prove that its supported simulation results are trustworthy

This is not a performance plan and not a UI plan. It is the verification plan
for numerical confidence.

## Principles

For `v1.x`, a study should not be treated as fully trustworthy only because:

- it runs
- it has a sample
- it has a report/export path

It should also have:

- an explicit benchmark case
- expected reference metrics
- documented tolerances
- a clear automation status

## Validation levels

### Level A: hand-checkable sanity cases

Use very small problems where we can cross-check:

- displacement
- reaction
- force / moment / torque
- stress
- temperature / heat flux

These are the fastest way to catch sign mistakes, load mapping mistakes, and
field bookkeeping mistakes.

### Level B: canonical engineering benchmarks

Use standard textbook or widely recognized FEM verification cases for:

- truss
- beam
- frame
- plane stress / strain
- pure heat conduction
- thermo-mechanical response

These validate more than a single formula. They validate the operator family.

### Level C: release samples as golden paths

Every officially supported sample should also have a numerical expectation, not
only a UI expectation.

For example:

- expected max displacement
- expected max stress
- expected max heat flux
- expected max axial force

These can tolerate small solver or mesh differences, but they should still
define a trusted range.

## Accuracy status labels

Use these labels for `v1.x` planning:

- `verified`
  benchmark case exists, tolerances are defined, and the case is automated
- `partially verified`
  benchmark case exists and is checked manually or by unit tests, but the full
  regression path is not yet automated
- `unverified`
  feature exists but there is not yet a trustworthy benchmark path

## Suggested tolerance policy

These are planning defaults, not hard law. Adjust per family when needed.

- displacement-like quantities:
  target relative error `<= 1e-4` for closed-form cases
- force / reaction / moment / torque:
  target relative error `<= 1e-4`
- stress-like quantities:
  target relative error `<= 1e-3`
- temperature and heat-flux quantities:
  target relative error `<= 1e-4`
- sequential thermal -> thermo-mechanical bridge:
  verify both:
  - mapped temperature field consistency
  - resulting structural response tolerance

For singular or mesh-sensitive families, define case-specific tolerances rather
than pretending one global number is honest.

## Family plan

| Study family | Current `v0.9.0` support | Accuracy target for `v1.x` | Benchmark shape | Automation target |
| --- | --- | --- | --- | --- |
| `axial_bar_1d` | fully supported | `verified` | closed-form tensile bar and restrained bar | unit test + release fixture |
| `thermal_bar_1d` | fully supported | `verified` | restrained thermal expansion bar | unit test + workflow fixture + orchestrated smoke |
| `spring_1d` | fully supported | `verified` | 1D chain with hand-checkable extension and force | unit test |
| `spring_2d` | minimal | `verified` | small planar spring grid | unit test + direct-mesh + orchestrated fixture |
| `spring_3d` | minimal | `verified` | small spatial spring cage | unit test + direct-mesh + orchestrated fixture |
| `beam_1d` | fully supported | `verified` | cantilever and distributed-load beam | unit test + sample fixture |
| `torsion_1d` | fully supported | `verified` | shaft twist and shear-stress case | unit test + sample fixture + orchestrated smoke |
| `frame_2d` | fully supported | `verified` | portal frame and restrained thermal frame | unit test + sample fixture + orchestrated smoke |
| `frame_3d` | backend operator shipping | `verified` | cantilever space frame with bending and rotation response | unit test + sample fixture + orchestrated smoke |
| `truss_2d` | fully supported | `verified` | small planar truss benchmark | unit test + sample fixture + orchestrated smoke |
| `truss_3d` | fully supported | `verified` | stable 3D truss benchmark with known response | sample fixture + orchestrated smoke |
| `plane_triangle_2d` | fully supported | `verified` | constant-strain patch and cantilever plate | unit test + sample fixture + orchestrated smoke |
| `plane_quad_2d` | fully supported | `verified` | quad patch and cantilever plate | unit test + sample fixture + orchestrated smoke |
| `heat_bar_1d` | shipping in thermal domain | `verified` | 1D conduction gradient and source case | unit test + sample fixture + orchestrated smoke |
| `heat_plane_triangle_2d` | shipping in thermal domain | `verified` | triangular conduction patch | unit test + sample fixture + orchestrated smoke |
| `heat_plane_quad_2d` | shipping in thermal domain | `verified` | quad conduction patch | unit test + sample fixture + orchestrated smoke |
| `thermal_truss_2d / 3d` | shipping in thermo-mechanical domain | `verified` | restrained thermal truss response | unit test + sample fixture + orchestrated smoke |
| `thermal_beam_1d` | shipping in thermo-mechanical domain | `verified` | restrained gradient beam | unit test + sample fixture + orchestrated smoke |
| `thermal_frame_2d / 3d` | shipping in thermo-mechanical domain | `verified` | restrained thermal frame and restrained space-frame thermal response | unit test + sample fixture + orchestrated smoke |
| `thermal_plane_triangle_2d / quad_2d` | shipping in thermo-mechanical domain | `verified` | restrained thermoelastic plane patch | unit test + workflow fixture + orchestrated smoke |

## Required benchmark artifacts

For each `verified` or `partially verified` family, add:

1. `benchmark case`
   a named case with stable geometry, loads, and material
2. `reference metrics`
   the specific numbers we compare against
3. `tolerance`
   documented near the case, not assumed from memory
4. `automation status`
   one of:
   - unit test
   - integration fixture
   - workflow smoke
   - manual only

## Where to automate

Use the smallest honest layer first:

- solver correctness:
  Rust solver tests in
  [workers/rust/crates/solver/src/lib.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/solver/src/lib.rs)
- protocol / result-shape integrity:
  Rust protocol and CLI tests in
  [workers/rust/crates/protocol/src/lib.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/protocol/src/lib.rs)
  and
  [workers/rust/crates/cli/src/main.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/cli/src/main.rs)
- sample and workflow regression:
  release smoke and Workbench UI smoke in
  [docs/release-archive-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-archive-0.9.0.md)
  and
  [tests/integration](/Users/Shared/chroot/dev/kyuubiki/tests/integration)
- performance-only baselines:
  benchmark tooling in
  [workers/rust/benchmarks/README.md](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/README.md)

## First 1.x priorities

Prioritize these first:

1. `axial_bar_1d`
2. `thermal_bar_1d`
3. `beam_1d`
4. `frame_2d`
5. `truss_2d`
6. `plane_triangle_2d`
7. `plane_quad_2d`
8. `heat_bar_1d`
9. `heat_plane_quad_2d`
10. `heat -> thermo-mechanical` bridge workflows

This ordering matches the current support matrix and gives `v1.x` the best
accuracy-to-effort return.

The first concrete baseline seed set is tracked in:

- [accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)

## Exit rule for 1.x accuracy

Treat a family as accuracy-ready for `v1.x` when:

- its benchmark case exists
- its reference metrics are documented
- its tolerances are explicit
- its automation status is at least `partially verified`
- its results do not contradict the support claim made for the family

## Related docs

- [release-archive-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-archive-0.9.0.md)
- [testing-and-ci.md](/Users/Shared/chroot/dev/kyuubiki/docs/testing-and-ci.md)
- [accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)
- [workers/rust/benchmarks/README.md](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/README.md)
