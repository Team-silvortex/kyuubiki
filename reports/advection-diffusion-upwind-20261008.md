# Explicit Upwind Verification

Date: 2026-10-08. Scope: current source macOS build of steady 1D
advection-diffusion and its native Headless -> Orchestra -> Rust Agent chain.
No installed application, remote scale run or historical qualification
promotion is included.

## Numerical Change

The default central Galerkin discretization remains unchanged. A two-element
unit model with diffusivity/area 1, velocity 20 and endpoint concentrations
0/1 still gives middle concentration -2 under Galerkin. This retained regression
shows why a finite answer alone is not a physical acceptance criterion.

Explicit `scheme: "upwind"` assembles conservative upstream endpoint flows.
For the same source-free model, the middle concentration is 1/12. A source of
4 changes it to 1/4; physical total fluxes are 2 and 11, while numerical fluxes
are -1/2 and 7/2. The numerical jump, not the physical flux jump, equals 4.
Artificial diffusivity is 5 and is reported, not hidden in the material input.

The implementation keeps physical flux fields and adds optional stabilization
diagnostics plus maximum absolute numerical flux. Missing/default scheme
preserves the old JSON shape. Typed Rust callers must initialize the new
request field and optional result fields in struct literals. Solver-specific
execution remains outside Orchestra and the Agent's generic task machinery.

## Independent Numerical Checks

Eight tests in
`workers/rust/crates/solver/tests/advection_diffusion_bar_upwind.rs` cover:

1. Uniform paths at 2, 8 and 64 elements and seven signed velocities, including
   +/-1000. Concentrations are bounded/monotone and match an independently
   derived discrete recurrence; diagnostics agree with explicit flux formulas.
2. The original unstabilized Galerkin result and omission of optional JSON
   fields, without an implicit high-Peclet algorithm switch.
3. Nodal source balance with separate physical and numerical fluxes.
4. Layered area/diffusivity/velocity coefficients with independently computed
   middle concentration 2/7 and area-weighted source balance, in path and dense
   assembly with reversed endpoint connectivity.
5. Continuum exponential-profile refinement at 16/32/64/128 elements; each
   error reduction ratio lies between 1.8 and 2.2, consistent with first order.
6. The zero-velocity diffusion limit with zero artificial transport.
7. Strict scheme decoding and the legacy default.
8. Cancellation during node/element generation, node summary and the third
   element-summary pass for numerical flux; a fresh call then succeeds.

The numerical reference is derived from the discrete equations, not from a
second call to the same solver. NIST's
[FiPy numerical schemes reference](https://pages.nist.gov/fipy/en/stable/numerical/scheme.html)
supports the upstream-weighting/diffusion tradeoff and central coefficient
restriction. FiPy's face Peclet convention is twice the reported element Peclet
here. No external FiPy simulation was executed.

First-order upwind adds numerical diffusion; it is not a sharp-layer accuracy guarantee.
The bounded concentration check is scoped to uniform source-free paths with
Dirichlet 0/1 boundaries. Layered tests establish discrete conservative balance,
not general variable-coefficient continuum convergence or arbitrary junction
conditions. Near-zero numerical flux uses a scaled absolute tolerance, not a
claim of relative accuracy for exponentially small quantities.

## Actual Service Execution

`real_upwind_service_chain_preserves_scheme_flux_metrics_and_failure_recovery`
uses two disposable source Agents and an isolated Orchestra server:

| Admission | Checked outcome |
| --- | --- |
| 1 | Positive velocity, bounded concentration and explicit flux diagnostics |
| 2 | Negative velocity with signed conservative flux |
| 3 | Reversed endpoints and nonzero nodal source |
| 4 | Native CLI immutable file upload, physical readback and saved/reloaded numerical-flux research metric |
| 5 | Workflow input -> solve -> output, all three nodes completed |
| 6 | Invalid file-backed scheme, typed Agent failure with no result |
| 7 | Upwind physical-output overflow, terminal numerical failure with no result |
| 8 | Healthy native CLI calculation after the failures on the same Agents |

An additional invalid inline scheme fails before submission: the job and
Agent execution counters stay at 5. Both admitted failures stop later project
writes. The final job count and sum of Agent execution counters are exactly 8;
both Agents return to accepting state. The persisted metric regenerates without
another computation. Temporary large padding is generated only during tests
and removed by fixture owners; no large dataset is added to the repository.

Three typed Rust protocol tests also check the shared ten-case scheme fixture,
legacy result decoding and round-trip preservation of separate diagnostics.
Two Elixir unit tests consume the same fixture and check conflicting internal
atom/string keys. The API regression verifies invalid schemes create no jobs
and valid upwind survives dispatch; that mapping test uses a fake Agent and is
not itself numerical evidence.

## Verification Results

| Suite | Result |
| --- | --- |
| Six focused transport integration suites | 37 passed |
| Protocol library | 128 passed |
| Engine library | 637 passed, 1 pre-existing ignored |
| Thermo/transport workflow integrity integration | 7 passed |
| Native CLI library | 16 passed |
| Full actual-service Headless integration suite | 59 passed in 153.54 seconds |
| Elixir scheme, API and graph tests | 10 passed |
| Native Headless qualification | 544 passed across two suites |

Clippy checks Protocol, Solver, Engine, CLI and Benchmark across all targets
with warnings denied. The current transport validation profile now includes
six commands and the upwind conservation/refinement invariants. New evidence
is recorded at verified contract/numerical/recovery scope in the coverage
tensor, rather than rewriting historical qualification records.

Operator validation was executed and its generated report verified. Protocol
qualification passed with 128 library tests, 61 RPC methods and five TaskIR
examples; its generated report also passed verification. Coverage-tensor
self-tests and the current audit pass with zero structural gaps, while retaining
4 maturity gaps, 19 evidence-grade gaps and 14 P0 gaps; overall daji readiness
remains blocked. Topology, module/function matrix, extension standard, four
runtime API contract families, documentation inventory, 26 HTML book files,
224 exact version contracts and the organization audit pass. Source files stay
within 800 lines and documents within 2000, with zero tracked organization debt.

The old transport/output reports remain historical scoped evidence, not
claims that the new scheme was already covered by their earlier runs.

## Remaining Boundaries

Default Galerkin can still oscillate at high Peclet number. Upwind does not
change that default or eliminate boundary-layer smearing. Users must inspect
mesh refinement and artificial diffusivity. For varying area/velocity, the
conservative upwind balance differs from the old `v*C_x` formulation; choosing
the intended physical equation remains explicit.

Multidimensional/transient transport, nonlinear reactions, material calibration,
installed GUI controls, remote performance, signed execution provenance and
general industrial CFD qualification remain outside this acceptance scope.
No release metadata, Git commit/push or installed App was changed.

Contract and examples: [explicit upwind transport](../docs/advection-diffusion-upwind.md).
