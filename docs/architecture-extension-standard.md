# Architecture Extension Standard

This is the standard flow for adding new architecture surface area to Kyuubiki.
It applies to modules, function paradigms, service surfaces, evidence lanes,
and shared contract families.

The machine-readable source is
`config/architecture/module-extension-standard.json`.

## Core Rule

Nothing enters the architecture map as prose only.

Every extension must have:

- ownership in `config/architecture/module-topology.json`
- functional coverage in `config/architecture/module-function-coverage-matrix.json`
- evidence depth in `config/architecture/module-function-coverage-tensor.json`
- a runnable gate or contract evidence
- documentation of what the extension does not own

If a required cell is marked `covered` but has no runnable test command and no
contract evidence, the tensor reports `weak_evidence`.

## Adding A Module

1. Add a stable module ID to `module-topology.json`.
2. Declare exactly one architecture layer.
3. Declare repository-relative `owned_paths`.
4. Declare upstream `depends_on` modules.
5. Attach benchmark lanes and security lanes.
6. Add risk tags that describe how the module can fail.
7. Add required paradigms and cell statuses in the matrix.
8. Add tensor lane mappings and contract evidence until required covered cells
   are not left as unexplained `thin` evidence.
9. Run `make check-module-function-coverage-tensor`.
10. Add prose ownership and non-ownership notes.

Do not create a top-level module for an internal service face. Use a service
surface when the code and ownership remain inside an existing module.

## Adding A Function Paradigm

1. Add the paradigm to `module-function-coverage-matrix.json`.
2. Add it to `required_by_module` only where the module truly owns that
   capability.
3. Add benchmark and security lane mappings in
   `module-function-coverage-tensor.json`.
4. Ensure every required covered cell has runnable evidence or contract
   evidence.
5. Prefer a `strong` maturity coordinate before using the paradigm in release
   claims. If the coordinate is intentionally `medium` or `thin`, document the
   next hardening gate.

## Adding A Service Surface

1. Attach `service_surfaces` to the owning module in `module-topology.json`.
2. Do not give the service surface its own `owned_paths`.
3. Bind shared runtime API families when the surface exposes contracts.
4. Add readiness or report evidence when the surface is deployable.

This is the pattern used by `central-web-service`: it is part of
`orchestra-control-plane`, not a separate product shell.

## Adding An Evidence Lane

1. Declare the lane under `benchmark_lanes` or `security_lanes`.
2. Add at least one command under `lane_test_plan`.
3. Map paradigms to the lane in the tensor.
4. Regenerate the tensor and confirm it changes the intended cells.

Evidence lanes should be concrete. A lane that cannot point to a command is a
planning note, not evidence.

## Adding A Contract Family

1. Add repository-relative source files.
2. Add schema or text anchors to the appropriate checker config.
3. Attach client surfaces and service surfaces when the contract crosses
   modules.
4. Add tensor contract evidence if the contract proves a required paradigm.
5. Run the family checker and `make architecture-check`.

## Required Gates

Run the smallest relevant gate while editing:

- `make check-module-topology`
- `make check-module-function-matrix`
- `make check-module-function-coverage-tensor`
- `make check-contracts-runtime-api-surface`
- `make check-central-store-contract`

Run `make architecture-check` before treating the extension as integrated.
