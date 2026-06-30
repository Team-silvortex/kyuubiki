# tamamono 1.x

This is the single entrypoint for the current Kyuubiki product line.

Use it when you want the shortest answer to:

- what `tamamono 1.x` means
- what this line is optimizing for
- where to go next inside the current `1.x` documentation set

## What defines this line

`tamamono 1.x` is the point where Kyuubiki stops defining progress mainly by
new operator families and starts defining it by industrial qualities:

- numerical trust
- repeatable validation
- bug fixing and consistency
- smoother operator and modeling experience

## What not to expect

This line should not grow by default through feature-count inflation.

The default posture is:

- keep the major version at `1`
- improve confidence before widening scope
- only add new capability when it strengthens the industrial baseline

The current published release snapshot in this line is `tamamono 1.13.0`.

The current shipping point now sits inside the wider `1.13.x` to `1.20.x`
industrialization range, where the main job is still hardening trust,
contracts, and operator-visible runtime behavior rather than inflating feature
count.

The immediate hardening focus is:

- benchmark-backed accuracy claims instead of anecdotal confidence
- clearer task, failure, and recovery semantics across runtime surfaces
- stronger workflow and asset contracts instead of ad hoc payload growth
- more explicit Installer-side remote deployment and runtime-control behavior

These expectations are meant to remain true across later `1.x` releases too.

## Current backend momentum

Recent operator work and runtime-control work are following the `tamamono 1.x`
rule in the right order:

- add the solver and protocol path
- add agent/runtime support
- add sample-backed orchestrated smoke
- then decide whether wider UI exposure is worth it

The current example is the now-verified `frame_3d` / `thermal_frame_3d` / `thermal_truss_3d` backend line:

- Rust solver support exists
- protocol and engine paths exist
- agent RPC handling is wired through
- formal accuracy baselines exist
- official-sample orchestrated API smoke exists for all three studies

That is the kind of growth this line should prefer: narrower, more verified,
less speculative, and easier to explain across Workbench, Installer, SDK, and
agent surfaces.

## Current reading path

1. [version-line.md](version-line.md)
   Formal version-line note, codename, and major-version policy.
2. [tamamono-minor-lines.md](tamamono-minor-lines.md)
   Suggested long-range grouping for the `1.x` minors.
3. [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
   Trust-gate checklist for deciding whether the line is ready to become a
   credible `2.0` early-commercial product.
4. [accuracy-plan.md](accuracy-plan.md)
   Accuracy roadmap, benchmark targets, and verification priorities.
5. [material-research-roadmap.md](material-research-roadmap.md)
   Reliability roadmap for turning material studies from runnable prototypes
   into reproducible screening, review, and qualification-oriented workflows.
6. [accuracy-baselines.md](accuracy-baselines.md)
   Concrete benchmark baselines already enforced in automation.
7. [operator-sdk.md](operator-sdk.md)
   Current extension-contract direction for growing operator capabilities
   without turning every family into a one-off vertical slice.
8. [workflow-graph.md](workflow-graph.md)
   Multi-operator composition direction for shader-like workflow growth.
   The first headless reference runner now exists for
   `heat_plane_quad_2d -> thermal_plane_quad_2d`, and the control plane now
   exposes a first built-in workflow catalog entry for
   `workflow.heat-to-thermo-quad-2d`.
9. [workflow-dataset.md](workflow-dataset.md)
   ONNX-like cross-operator data contract for workflow-carried values, with
   named datasets, shape semantics, and schema references shared across nodes.
10. [installer-remote-control.md](installer-remote-control.md)
   Installer-owned remote deployment and runtime-control surface, including
   workflow snapshots, certificate alignment, and mesh-oriented operator
   guidance.
11. [language-packs.md](language-packs.md)
   Local-first multilingual extension path for the Workbench UI, with a stable
   schema ready before remote delivery lands.
