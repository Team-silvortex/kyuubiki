# Explicit Upwind Transport

Steady 1D advection-diffusion supports an opt-in conservative first-order
upwind scheme for advection-dominated models. It keeps the default Galerkin
algorithm unchanged and exposes artificial diffusion, so a bounded result is
not mistaken for an accurately resolved boundary layer.

## Selecting the Scheme

Add `"scheme": "upwind"` to the model payload. The existing service route,
Headless action, Agent RPC and workflow operator remain unchanged; see the
[transport service chain](advection-diffusion-service-chain.md).

```json
{
  "scheme": "upwind",
  "nodes": [
    {"x": 0.0, "fix_concentration": true, "concentration": 0.0},
    {"x": 0.5, "fix_concentration": false, "source": 4.0},
    {"x": 1.0, "fix_concentration": true, "concentration": 1.0}
  ],
  "elements": [
    {"node_i": 0, "node_j": 1, "area": 1.0, "diffusivity": 1.0, "velocity": 20.0},
    {"node_i": 1, "node_j": 2, "area": 1.0, "diffusivity": 1.0, "velocity": 20.0}
  ]
}
```

Missing `scheme` or explicit `"galerkin"` selects the old algorithm. There is
no automatic switch based on Peclet number. Null, unknown names, wrong case
and nonstring values reject instead of silently selecting an algorithm.
The [shared fixture](../schemas/examples.advection-diffusion-scheme.json)
checks these choices in typed Rust and Elixir service normalization.

Legacy JSON requests and results remain readable. Galerkin serialization omits
the default scheme and new optional diagnostic fields. Rust struct-literal
callers must add `scheme: Default::default()` or select
`AdvectionDiffusionBar1dScheme::Upwind`; result struct literals also need the new
optional `stabilization` and `max_numerical_flux` fields. This is a Rust source
change, not a new RPC version or a wire-format migration requirement.

## Flux and Conservation

All signs are along global x, independent of endpoint connectivity ordering.
The existing physical quantities keep their original meaning:

```text
h                  = abs(x_j - x_i)
gradient           = (C_j - C_i) / (x_j - x_i)
diffusive_flux     = -D * gradient
advective_flux     = v * (C_i + C_j) / 2
total_flux         = diffusive_flux + advective_flux
peclet_number      = abs(v) * h / (2 * D)
```

Upwind adds an element `stabilization` object and top-level
`max_numerical_flux`, the maximum absolute numerical flux density:

```text
artificial_diffusivity = abs(v) * h / 2
stabilization_flux     = -artificial_diffusivity * gradient
numerical_flux         = diffusive_flux + v * C_upstream
                       = total_flux + stabilization_flux
```

`C_upstream` uses the physical flow direction, not the storage order. The
solver computes numerical flux from the upstream expression to avoid an
unnecessary cancellation between artificial diffusion and central advection.
Every published diagnostic must be finite and representable; a finite
numerical flux cannot excuse an unrepresentable physical diagnostic.

For the example, concentration is `[0, 0.25, 1]`, physical total fluxes are
`[2, 11]`, and numerical fluxes are `[-0.5, 3.5]`. Artificial diffusivity is
5 in both elements. The numerical flux jump is the nodal source 4; the physical
flux jump is not the conservative balance of this stabilized discretization.
For unequal areas, use **area times numerical flux** when checking nodal balance.

Upwind assembles conservative endpoint flows. With constant area and velocity,
it approximates the same steady transport equation as Galerkin. If area or
velocity varies, its balance is `(A * q)' = nodal load`; that is not generally
identical to the old Galerkin `v * C_x` formulation. Choose the intended equation
explicitly rather than assuming the two schemes are interchangeable.

## Research and Execution

The same model travels through inline submission, immutable file upload and
workflow input -> solve -> output, without solver-specific logic in the Agent
or an Orchestra engine dependency. Completed physical results include the
scheme and diagnostics. For a native solve -> wait -> fetch batch, a numerical
flux metric can use:

```text
/steps/2/result_preview/result/elements/0/stabilization/numerical_flux
```

The live regression persists and reloads that research evidence without another
solve. Select physical `total_flux` for physical reporting and numerical flux
for discretized conservation; never silently substitute one for the other.

Invalid inline scheme choices reject before job admission. Invalid file-backed
choices fail typed Agent decoding after admission. Numerical failures publish
no result and stop dependent writes. Healthy tasks then run on the same Agents
without replaying failed calculations. Cooperative solver cancellation and
finite-output guards are shared with the default path.

## Validation and Limits

Uniform source-free paths are checked at 2, 8 and 64 elements with velocities
`-1000, -20, -1, 0, 1, 20, 1000`. Their bounded monotone concentrations match
an independently derived discrete recurrence. A moderate-velocity continuous
exponential solution checks first-order error reduction at 16, 32, 64 and 128
elements. Layered coefficients check area-weighted nodal balance in both path
and dense assembly, including reversed connectivity.

First-order upwind adds numerical diffusion; it is not a sharp-layer accuracy guarantee.
Refine the mesh and compare physical observables, not only solver success or
monotonicity. NIST's [FiPy numerical schemes reference](https://pages.nist.gov/fipy/en/stable/numerical/scheme.html)
explains the central-scheme coefficient restriction and the diffusion tradeoff
of upstream weighting. Its face Peclet convention is `abs(v)*h/D`, twice this
operator's reported element value. It is a method reference, not an external
FiPy execution cross-check.

Evidence is bounded to source macOS tests of this steady 1D operator. It does
not establish multidimensional CFD, transient transport, nonlinear reactions,
arbitrary junction conditions, material calibration, installed App acceptance
or remote/large-scale qualification. Historical operator qualification records
are unchanged; the new tests extend the current validation profile only.

See the [verification report](../reports/advection-diffusion-upwind-20261008.md).

Continue with the [stabilized research chain](advection-diffusion-research-chain.md)
to preserve these diagnostics through configured quality and objective decisions,
including native service execution, corrupt-evidence isolation and explicit recovery.
