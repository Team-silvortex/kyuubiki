# Release Notes: v0.9.0

`v0.9.0` is the release where Kyuubiki stops feeling like a promising shell
around a few FEM studies and starts reading like a broader engineering
workstation:

- a wider solver family
- a clearer desktop operator story through `Kyuubiki Hub`
- a more structured Workbench information architecture
- a stronger local development loop through managed hot-reload flows

## Highlights

### Broader operator family

Kyuubiki now supports a much wider study family than the earlier truss- and
plane-heavy baseline.

Newly integrated or expanded solver/operator lines now include:

- `thermal_bar_1d`
- `spring_1d`
- `spring_2d`
- `spring_3d`
- `beam_1d`
- `torsion_1d`
- `frame_2d`
- `plane_quad_2d`

These are not just protocol placeholders. Across the stack they now have:

- Rust protocol coverage
- solver kernels
- Rust agent RPC support
- control-plane orchestration routes
- direct-mesh request/result support
- Workbench-facing result payloads

### Family-aware Workbench

Workbench now treats study growth as a first-class information-architecture
problem rather than letting the UI flatten into one long list.

The frontend now groups study entry and sample discovery by family:

- `Axial & Springs`
- `Beams & Frames`
- `Trusses`
- `Planes`

That family structure is now visible across:

- `Study` type selection
- `Library > Samples`
- `Model > Tree`
- `Inspector > Report`

The goal is simple: new solvers can keep landing without turning the editor
into a crowded switchboard.

### Stronger operator-facing analysis surfaces

Several operator families now have more specific result-review paths instead of
only generic tables and legends.

Examples now present in Workbench include:

- plane-field switching for `von Mises / principal stress / max shear`
- hotspot ranking and direct viewport focus for plane studies
- frame result fields for stress, bending, combined stress, and moment
- frame member-end force tables with sorting and export
- spring-family hotspot ranking and force-table export
- thermal-bar review with temperature-driven stress and axial-force summaries

### Hub becomes a real operator entrypoint

`Kyuubiki Hub` is now much closer to a day-to-day desktop shell instead of only
an early launcher surface.

Hub now covers:

- desktop readiness and staging checks
- workload-library management
- local and remote workload-catalog sync
- bundle inspect / validate / normalize / unpack / pack / diff
- assistant-guided onboarding
- assistant action audit with security-event mirroring
- runtime watch surfaces for hot-loop and stack logs
- managed hot-reload loop control for local, cloud, and distributed dev shapes

### Hot-reload becomes repo-level

`v0.9.0` now has a much more coherent development loop:

- `make hot-local`
- `make hot-cloud`
- `make hot-distributed`

These keep the native HMR flows where they already exist and fill in the
missing restart-on-change paths for the non-Phoenix Elixir control plane and
Rust solver agents.

Hub can also observe and manage those hot-loop paths directly.

## What changed by layer

### Frontend / Workbench

- family-aware study grouping
- family-grouped sample library
- new study integration for `thermal_bar_1d`
- expanded result switching, hotspot ranking, and export for line and plane
  studies
- stronger result-aware tree/report surfaces for frame and spring families

### Control plane

- broader HTTP solve coverage for the growing solver family
- richer agent metadata and capability-aware routing
- workload-catalog endpoints for Hub-driven distribution
- stronger security-event integration across assistant/operator flows

### Rust data plane

- broader solver kernel coverage across line, frame, spring, torsion, thermal,
  and plane operators
- matching RPC method growth inside the Rust agent
- stronger reusable protocol-first engine boundaries

### Desktop / Hub

- Hub-managed workload library
- runtime watch and hot-log observation
- desktop release readiness/status walls
- local workload attach and open-in-workbench flow
- assistant onboarding with audit and security mirroring

## Upgrade notes

- Existing `.kyuubiki` project flows remain compatible, but the project format
  now behaves more like a standardized engine bundle with asset catalogs and
  references.
- Desktop release flow now assumes Hub is part of the normal operator path, not
  just installer/workbench binaries.
- Workbench study selection now expects users to think in solver families first,
  which is intentional and should improve discoverability as more operators
  arrive.

## Suggested validation for v0.9.0

For a broad pre-release confidence pass, start with:

- `make test-web`
- `make test-rust`
- `make test-frontend`
- `make test-sdk`
- `make test-integration`
- `make test-hub-gui`
- `make test-installer-gui`
- `make test-workbench-gui`
- `make desktop-status PLATFORM=all`

Current validation note:

- `make test-integration` is now a stronger first-pass gate because it includes
  API smoke, cluster smoke, direct-mesh smoke, and split Workbench UI smoke for
  representative `Mechanical`, `Thermal`, and `Thermo-mechanical` samples.

Then confirm the current release scaffolding path through:

- [packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
- [desktop-release-checklist.md](/Users/Shared/chroot/dev/kyuubiki/docs/desktop-release-checklist.md)
