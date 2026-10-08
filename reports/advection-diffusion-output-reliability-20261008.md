# Advection Diffusion Output Reliability

## Source Scope

October 8, 2026. Source macOS working-tree overlay on `f5a4247a`
(`daji 3.5.3` commit label); packaged metadata remains `3.5.0`.
This round changes the Rust transport solver and its verification surfaces,
not Agent-specific execution rules, RPC/result schemas or service routes.
Earlier service/normalization/result-readback edits remain in the working tree.
No version bump, commit, push or installed App rebuild is part of this round.

## Reproduced Defects

Before the fix, the initial 12-case output regression had one passing control
and 11 failing cases. These failures cover overlapping mechanisms, not 11
independent defects:

- Endpoint reversal changed the concentration solution and signed gradients/
  fluxes even though global coordinates, boundary values and velocity were equal.
- A representable `1e308` average overflowed; the returned infinite fields would
  serialize to JSON null instead of failing the calculation.
- A concentration difference could overflow even when its gradient was finite.
- Diffusion/advection coefficients and Peclet arithmetic lost range in
  intermediate products or denominators even when the final value was finite.
- Unrepresentable fluxes/Peclet and nonzero diffusion underflow returned success.
- Dense fixed-boundary replacement masked a non-finite assembled matrix.
- Output construction lacked the common cancellation checkpoints.

## Repairs and Local Controls

Signed element orientation is included in local advection matrices; gradients
use signed global coordinate separation. Path and dense fallback share one local
matrix builder. Partially and fully reversed meshes retain the same concentrations,
physical fluxes and Peclet values.

The ordinary normal-range product uses a cheap direct path. Exceptional products
normalize binary mantissas/exponents and restore scale once, including subnormals,
instead of adding an arbitrary clamp. Stable averaging/difference division avoids
avoidable overflow. Nonzero coefficients or physical outputs that round to zero
fail explicitly; non-finite local/assembled matrices and total flux also fail.
Fixed dense rows do not evaluate RHS terms belonging to discarded equations.

Sixteen output tests cover independent Galerkin concentration, path/dense and
mixed connectivity orientation, representable extreme controls, binary-scaled
free equations, smallest/largest constant concentrations, nonzero underflow,
matrix overflow before boundary elimination, and entry/chunk/final-scan
cancellation with a healthy next call. Ordinary closed-form, refinement,
review and 10,000-node numbering-independent path regressions remain selected.
Result builders use the existing shared cooperative checkpoints, not a new
transport-specific Agent watchdog or solver-control protocol.

A separate in-process actual solve -> diagnose -> quality -> objective -> decision
workflow test rejects advective/Peclet overflow in strict mode. Explicit recovery
marks the solver failed, skips all five dependent nodes, publishes no derived
result/quality/decision artifact and retains the independent branch's value 7.
A healthy reversed source-free model then restores the full decision chain with
independent middle concentration 0.62 and both signed fluxes 0.922. This is
workflow branch isolation, not remote process or installed service recovery.

Expanded fixture review caught two test-construction issues: one scale combination
had a genuinely unrepresentable physical flux, and inverse `powi` evaluation
zeroed a desired subnormal source. Controls now select representable fluxes and
construct exact binary subnormal sources without that intermediate. Neither
correction weakens an output rejection or solver tolerance.

## Actual Service Recovery

One isolated source Orchestra and two actual Rust Agents admit exactly nine
tasks through native Headless control:

1. Forward two-element source model: middle concentration 3.875, total fluxes
   -2.28125 and 1.71875, difference 4 and element Peclet number 0.125.
2. Reversed connectivity: the same independently derived physical fields.
3. Finite `1e308` constant concentration with velocity `1e-308`: mean `1e308`,
   signed flux 1 instead of a successful null field.
4. Advective flux overflow: terminal failure, no result or later project write.
5. Total flux overflow with individually finite component fluxes: same isolation.
6. Peclet overflow: same isolation.
7. Duplicate-edge dense assembly overflow with fixed zero concentrations:
   same isolation, not a successful zero field.
8. Actual native CLI/file upload repeats the advective overflow case with a
   disposable 8 MB metadata pad: immutable upload receipt, persisted failure
   report and structured nonzero exit, no automatic retry.
9. Actual native CLI healthy reversed chain on the same Agents after failures.

All five failed jobs retain `has_result=false` and a representability diagnostic.
Failed `job_wait` stops the batch before the guarded project action. Final job
count and summed Agent execution counters are nine; projects are unchanged and
both Agents return to accepting work. No deliberate replay is hidden in the
healthy continuation. Test directories/services are disposable and cleaned up.

## Verification and Limits

| Selected suite | Passed | Existing ignored |
| --- | ---: | ---: |
| Solver shared tridiagonal, postprocess and cooperative-control unit tests | 32 | 2 |
| Transport output, closed-form, reliability, refinement and review integration | 29 | 0 |
| Engine unit and thermo/transport research-chain integration | 637 + 7 | 1 |
| Complete native CLI Headless live/harness integration | 58 | 0 |

There are 763 distinct selected passing tests and three existing ignored tests.
Focused repeats, the profile's repeat and the previous round's 1778 selected
tests are not added to this total. The complete Headless suite includes the new
nine-task numerical-failure regression and repeats the prior service/graph
normalization chains. Its 451.75-second source debug run competed with heavy
local modal tests; this elapsed time is not a performance or scale measurement.

An additional 793-case full Solver unit trial was deliberately interrupted
with SIGINT while unrelated heavy modal regressions were still running. It
is not a completed passing suite and none of its partial passes are counted.
The relevant shared kernels above were rerun successfully after interruption.

All-target Clippy for Solver, Engine and CLI passes with warnings denied.
Native Rust format and diff whitespace checks pass. All 59 validation profile
definitions validate; the transport profile executes all five commands and its
retained ignored `tmp/advection-diffusion-output-validation.json` report verifies.
Topology, module/function matrix, extension standard, runtime API surface,
documentation inventory/book (26 HTML files), version contracts (224), and
800-source/2000-document organization checks pass with no tracked size debt.
Tensor self-test and actual checks pass with zero structural gaps; four maturity,
19 evidence-grade and 14 P0 gaps remain, so Daji readiness remains blocked.

The transport validation profile and reliability test index include the new
output suite. Coverage tensor claims stay `verified`, scoped separately to
local numerical validation and actual service failure/recovery contracts.
Historical qualification records and the earlier nine-task service report are
not rewritten or promoted by this follow-up.

This is bounded steady 1D transport validation, not general CFD qualification.
No remote/Linux/installed App test, scale/RSS/performance claim, cross-language
numerical acceptance, stabilization at high Peclet, arbitrary branching-junction
physics or general variable-coefficient convergence is established. Finite and
orientation-consistent fields are necessary but insufficient for scientific
accuracy. Central Galerkin high-Peclet oscillations remain an explicit limitation.
