# Advection Diffusion Service Chain

Steady 1D advection-diffusion now uses the existing asynchronous service-job
lifecycle. The Rust solver and Agent RPC already implemented this operator;
this follow-up connects submission, discovery, workflows and Headless control
without coupling the engine to Orchestra. The initial service hookup did not
change the solver; the numerical hardening below is a separate follow-up.

## Public Entrypoints

| Surface | Entrypoint |
| --- | --- |
| Orchestra submission | `POST /api/v1/fem/advection-diffusion-bar-1d/jobs` |
| Native Rust Headless batch action | `solve_advection_diffusion_bar_1d` |
| Official Rust/Python/Elixir solve kind | `advection_diffusion_bar_1d` |
| Agent solver RPC | `solve_advection_diffusion_bar_1d` |
| Workflow operator | `solve.advection_diffusion_bar_1d` |
| Workflow input/output types | `model/advection_diffusion_bar_1d`, `result/advection_diffusion_bar_1d` |

The service returns a queued job receipt, not an immediate numerical result.
Follow it with `job_wait` and `result_fetch`. Ordinary and file-backed requests
share the [entity ID contract](graph-entity-input-normalization.md), including
rejection of ambiguous identifiers and nonobject nodes/elements.

The service catalog exposes the operator in the `transport` domain. Its local
baseline tag does not certify every Peclet regime, platform or industrial use.

## Small Research Input

The following model has unit length, area and diffusivity, velocity 0.5,
prescribed endpoint concentrations 2 and a middle nodal source 4. Concentration
units are caller-selected but must remain consistent with source and flux.
This dimensionless example is not a material-specific calibration.

```json
{
  "nodes": [
    {"x": 0.0, "fix_concentration": true, "concentration": 2.0},
    {"x": 0.5, "fix_concentration": false, "source": 4.0},
    {"x": 1.0, "fix_concentration": true, "concentration": 2.0}
  ],
  "elements": [
    {"node_i": 0, "node_j": 1, "area": 1.0, "diffusivity": 1.0, "velocity": 0.5},
    {"node_i": 1, "node_j": 2, "area": 1.0, "diffusivity": 1.0, "velocity": 0.5}
  ]
}
```

`fix_concentration` is required even when false. `concentration` and `source`
default to zero; `area`, `diffusivity` and `velocity` are required. Missing or
empty IDs generate positional labels. Diffusivity and area must be positive;
zero velocity and signed velocity are valid.

Expected concentration is `[2, 3, 2]`. Signed total fluxes are `[-0.75, 3.25]`;
their difference is the nodal source divided by area. Maximum absolute total
flux is 3.25 and maximum element Peclet number is 0.125. The live regression
checks these against an independent two-element Galerkin equation and element
flux formulas rather than comparing two executions alone.

## Saved Sources and Workflows

Create a model with kind `advection_diffusion_bar_1d` and the input above as its
payload. Use the returned immutable version ID with native
`solve_and_wait_from_model_version`. Only `model_version_id` is mandatory;
Orchestra selects the Agent for this supported direct service route.
`expected_model_source` remains available to bind research to the exact saved
content before submission. The combined output retains source, job, wait and
physical result receipts.

The former native batch contract incorrectly required explicit `endpoints`
even though its service executor could dispatch through Orchestra. Both saved
version solve actions now admit the service-routed form. Unsupported studies
that fall back to direct-mesh execution still require endpoints during executor
validation. This does not change the frontend/PWDT direct-mesh contract or
establish new explicit-endpoint desktop gateway support for this operator.

A workflow can connect input -> `solve.advection_diffusion_bar_1d` -> output.
The actual source-Agent regression checks all three nodes complete and the
`solve.result` artifact contains the expected signed reverse-velocity flux.
This does not add a task-IR execution-capability promotion or new RPC version.

## Results, Failures and Scope

File submissions use existing byte-verified model upload and
[native physical result readback](headless-result-artifact-readback.md).
Do not treat a file reference or a successful job status alone as numerical
evidence. Research metrics must select physical fields from completed runs.

The test creates, persists and reloads source-bound research evidence for the
middle concentration. Reloading a retained report does not submit a new solve.
Failure of an admitted model terminates `job_wait`, blocks downstream writes,
and does not automatically resubmit the calculation. Repair the model and
deliberately submit a new task; healthy tasks continue on the same Agents.

## Numerical Output Contract

Velocity and concentration gradient are signed along global x, not along the
arbitrary `node_i` -> `node_j` connectivity ordering. Reversing endpoints changes
neither the assembled physical equation nor signed physical flux; element
indices and endpoint labels still retain the caller's ordering.

The solver protects representable concentration averages, gradients, transport
coefficients and Peclet products against intermediate range loss. If a required
nonzero coefficient, gradient, flux or Peclet value cannot be represented, the
solve returns an explicit error rather than silently zeroing it or publishing
JSON nulls. A finite total flux must also be representable. Dense assembly is
checked before prescribed boundaries can overwrite invalid coefficients;
discarded equations at fixed nodes do not evaluate irrelevant RHS products.

Node/element result generation and maxima scans use the shared cooperative
cancellation mechanism. A cancellation returns an error before a successful
result; a later healthy solve uses a fresh scope. Default Galerkin result shape
is unchanged; the explicit upwind mode adds opt-in stabilization diagnostics.

The [numerical follow-up](../reports/advection-diffusion-output-reliability-20261008.md)
records a separate nine-admission actual source-Agent test: global-x analytic
checks, finite extreme outputs, five terminal numerical failures without results
or downstream writes, and healthy continuation on the same Agents.
An in-process research-chain regression also isolates a failed solve from its
diagnostic/quality/objective/decision descendants while retaining independent
work; a healthy reversed model restores the complete chain.
This is bounded steady 1D transport validation, not general CFD qualification.
Default central Galerkin advection remains unstabilized. An explicit
[conservative upwind mode](advection-diffusion-upwind.md) adds bounded uniform-path
and first-order refinement checks, with artificial diffusion reported separately.
Variable-coefficient continuum convergence, branching-junction physics and
multidimensional transport still need validation. Finite output is not accuracy.

Actual source macOS acceptance covers nine transport admissions and a separate
31-admission ten-family inline/file regression. The official Rust, Python and
Elixir SDK tests additionally verify HTTP and RPC mapping through mock servers.
Their mapping success is not cross-language numerical acceptance. No installed
App rebuild, remote test, scale benchmark, general CFD qualification, signed
execution provenance or algorithm accuracy claim is implied.

See the [verification report](../reports/advection-diffusion-service-chain-20261008.md)
and [remaining weaknesses](weakness-roadmap.md#roadmap-principle).
