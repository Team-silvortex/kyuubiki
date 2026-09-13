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
- an evidence grade target and paradigm priority weight
- explicit platform, backend, language and recovery qualification requirements
  where the capability makes those claims
- a runnable gate or contract evidence
- documentation of what the extension does not own

If a required cell is marked `covered` but has no runnable test command and no
contract evidence, the tensor reports `weak_evidence`.

A registered lane is a plan, not proof that it ran. Grades from `exercised`
upward require scoped `proven` claims. Tensor v5 takes the weakest required
dimension and also requires each explicit qualification scope to pass.
New modules must review advisory grade and scope gaps even when the structural
command exits successfully.

Each `qualification_requirements` entry declares `module_id`, `paradigm`,
`dimension`, `scope`, `target`, `acceptance`, `basis` and named `claims`.
Use an empty claim list for an open obligation. The dimension must be required
by that exact coordinate; references to unknown or differently scoped claims
are rejected. Review the actual retained report before binding it: schema and
text-anchor checks do not verify platform semantics, freshness or test success.
Rationale files are not proof, and a registered scenario list is not automatically
a complete inventory of every supported platform or feature.

## Adding A Module

1. Add a stable module ID to `module-topology.json`.
2. Declare exactly one architecture layer.
3. Declare repository-relative `owned_paths`.
4. Declare upstream `depends_on` modules.
5. Attach benchmark lanes and security lanes.
6. Add risk tags that describe how the module can fail.
7. Add required paradigms and cell statuses in the matrix.
8. Add tensor lane mappings, target grades, and scoped contract evidence until
   required covered cells are not left as unexplained `thin` evidence.
9. Register scenario acceptance requirements and explicitly bind only reviewed proof.
10. Run `make check-module-function-coverage-tensor`.
11. Add prose ownership and non-ownership notes.

Do not create a top-level module for an internal service face. Use a service
surface when the code and ownership remain inside an existing module.

A shared source layer that is consumed by independently packaged product shells
is a module when it owns its own synchronization, validation, and automation
contract. `desktop-shared-ui` is the reference: it does not merge the three
desktop applications, but it owns their common source assets and mirror gate.

## Adding A Function Paradigm

1. Add the paradigm to `module-function-coverage-matrix.json`.
2. Add it to `required_by_module` only where the module truly owns that
   capability.
3. Add benchmark and security lane mappings in
   `module-function-coverage-tensor.json`.
4. Declare the paradigm's evidence grade target and priority weight.
5. Ensure every required covered cell has runnable evidence or contract
   evidence.
6. Require `strong` dimension presence, target grade, and satisfied scenario
   requirements before using the paradigm in release claims. For each gap,
   document the next executable acceptance step.

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

Lane presence does not raise a coordinate's grade. Add an explicit graded
claim only after the execution evidence is retained and reviewed. A Rust
installed journey does not qualify Python/Elixir packages; Linux/macOS success
does not qualify Windows; a database snapshot does not qualify whole-generation
recovery with external artifacts.

## Adding A Contract Family

1. Add repository-relative source files.
2. Add schema or text anchors to the appropriate checker config.
3. Attach client surfaces and service surfaces when the contract crosses
   modules.
4. Add tensor contract evidence if the contract proves a required paradigm.
5. Add an explicit claim grade only when the contract is paired with proof at
   that strength; a schema alone remains `declared`.
6. Use `evidence_includes` for large scoped evidence bundles so the main tensor
   stays readable and under the project file-line guard.
7. Run the family checker and `make architecture-check`.

## Required Gates

Run the smallest relevant gate while editing:

- `make check-module-topology`
- `make check-module-function-matrix`
- `make check-module-function-coverage-tensor`
- `make check-contracts-runtime-api-surface`
- `make check-central-store-contract`

Run `make architecture-check` before treating the extension as integrated.
