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

The current shipping point in this line is `tamamono 1.4.0`, and these
expectations are meant to remain true across later `1.x` releases too.

## Current backend momentum

Recent operator work is following the `tamamono 1.x` rule in the right order:

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

That is the kind of operator growth this line should prefer: narrower, more
verified, and less speculative.

## Current reading path

1. [version-line.md](version-line.md)
   Formal version-line note, codename, and major-version policy.
2. [tamamono-minor-lines.md](tamamono-minor-lines.md)
   Suggested long-range grouping for the `1.x` minors.
3. [accuracy-plan.md](accuracy-plan.md)
   Accuracy roadmap, benchmark targets, and verification priorities.
4. [accuracy-baselines.md](accuracy-baselines.md)
   Concrete benchmark baselines already enforced in automation.
5. [operator-sdk.md](operator-sdk.md)
   Current extension-contract direction for growing operator capabilities
   without turning every family into a one-off vertical slice.
6. [workflow-graph.md](workflow-graph.md)
   Multi-operator composition direction for shader-like workflow growth.
   The first headless reference runner now exists for
   `heat_plane_quad_2d -> thermal_plane_quad_2d`, and the control plane now
   exposes a first built-in workflow catalog entry for
   `workflow.heat-to-thermo-quad-2d`.
7. [workflow-dataset.md](workflow-dataset.md)
   ONNX-like cross-operator data contract for workflow-carried values, with
   named datasets, shape semantics, and schema references shared across nodes.
8. [language-packs.md](language-packs.md)
   Local-first multilingual extension path for the Workbench UI, with a stable
   schema ready before remote delivery lands.
