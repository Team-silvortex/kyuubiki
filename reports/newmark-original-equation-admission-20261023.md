# Original Newmark-Increment Equation Admission

Date: 2026-10-23

Version line: daji 3.6.x, after `0a65f2e8`, extending the uncommitted
[original mechanical equation gate](homogeneous-mechanical-equation-admission-20261023.md).

## Gap and Reproduction

The transient spring solver checked the reduced Newmark linear solve, but did
not independently check the complete restored velocity increment against the
pre-elimination effective matrix and full current RHS. Correct reduced algebra
therefore did not protect against a faulty free-DOF gather/scatter map.

Before connecting the gate, two internal map fault-injection tests expected an
error but received successful states. These tests deliberately replace an
internal mapping; they do not demonstrate that normal validated user requests
currently generate those faulty maps. The fixed-component control now uses a
coupled chain so that the bad map actually writes a nonzero fixed increment.
Both malformed-map controls fail admission after the change; a clean call using
the original map succeeds afterward.

## Contract

For average-acceleration Newmark in the existing velocity-increment formulation:

```text
(K + 2 C / h + 4 M / h^2) delta_v = (4 / h) M a_n - 2 K v_n
```

The shared `PrescribedSystemValidator` borrows the full effective matrix and
indexes homogeneous supports once, independently of the reduced free map. After
every reduced solve, the increment is scattered into the existing `next_v`
buffer. Before converting that buffer into the next physical velocity:

- Complete dimensions, finite RHS/unknowns and exactly zero fixed increments
  are required.
- Every free original effective equation must pass the existing componentwise
  backward-error policy, including the shared subnormal/range controls.
- Fixed equations can contain reactions; they are not required to balance
  without those reactions.
- Failure or cooperative cancellation returns before displacement/velocity
  updates, acceleration recovery, energy metrics or history capture for that
  step. No partial successful result is returned.

The full RHS is the current pre-elimination increment RHS already assembled by
the integrator, not the physical applied-force vector by itself. Confusing the
two would incorrectly test a dynamic increment as a static displacement.

The gate retains one O(n) constraint index and adds an O(n+nnz) scan per step.
The existing `next_v` allocation is reused; there is no second complete increment
vector, matrix copy, dense conversion or factorization. This is extra validation
work, not a claimed performance improvement. Sampling never changes its frequency.
The integrator's coefficients, reduced solver, arithmetic used to update valid
states and existing tolerances are unchanged. No engine protocol or task format
change is needed; the engine remains a consumer of the operator.

## Added Coverage

Eight new tests consist of three solver unit tests and five engine integrations:

- Internal permuted gather/scatter and nonzero fixed-component map controls,
  followed by correct-map reuse.
- Signed coefficients/loads at binary scales `2^-300`, `1`, `2^300`, checked
  against simple one-step rational closed forms, plus borrowed/owned parity.
- Serialized dispatch and workflow output equality; entry and completion counts
  prove all seven increments are checked although only three frames are saved.
- Cancellation at entry, the complete-input scan and completion of unsaved step
  three, then identical replay. The observer checks the actual transient step,
  not a step prefix in an error that the outer cancellation scope may replace.
- A 1,030-node star, mostly restrained, reaches a genuinely wide original free
  row. Both direct and workflow calls cancel at its 1,024-entry checkpoint in
  unsaved step two and replay cleanly. Narrow rows intentionally do not emit
  per-row checkpoints; no production event was added just for a test.
- Loaded all-fixed models accept reactions and retain zero physical states.
- A heterogeneous damped chain independently checks midpoint kinematics and
  discrete work/energy/dissipation balance; sampled workflows preserve final
  states, peaks, elements and retained frames from full-history execution.

The 1,030-node star is a cancellation fixture, not a broad large-scale dynamics
qualification. Existing 10,000-node sparse-chain, static preload, rigid motion,
continuous-solution convergence, energy and state-restart fixtures are rerun
separately below.

## Validation

Pinned Rust 1.88.0, offline locked dependencies and existing build roots:

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed |
| --- | ---: | ---: |
| Solver library excluding `modal_frame_spectrum::` | 755 | 755 |
| Engine library | 711 | 711 |
| Nine solver/engine integration targets | 69 | 69 |
| Non-overlapping total | 1,535 | 1,535 |

Each platform has zero final failures, ten ignored library tests and 292 filtered
modal tests. The eight new cases are included once; focused runs are not added
to the total. Local solver/engine all-target Clippy passes with warnings denied;
workspace formatting passes. Linux used the existing server test copy and build
cache. Only the three source/test files and current workspace dependency lock
were synchronized; no server deployment, credentials, SSH configuration or
service lifecycle was changed.

Native operator metadata validation passes for 84 profiles with `executed=false`.
The new profile was also run with `--execute`: all three registered commands
pass. Those repeated cases are not added to the table. Coverage-tensor structure
and command checks pass across 13 modules and 11 paradigms, with zero structural
gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall readiness
remains `blocked`; this component claim does not qualify the full dynamic solver.
Whitespace checks pass, source/configuration files stay below 800 lines and this
report stays below the 2,000-line documentation limit.

The initial integration-test draft borrowed data into a `'static` observer and
did not compile. The corrected observer owns its counters. A later draft looked
for a row event on a narrow matrix; that event is intentionally absent, so the
row-cancellation control was moved to a real wide-row model rather than changing
runtime checkpoint policy. Final tests pass on both platforms after those test
corrections. No new package, backup tree, build root, version change, staging
operation or commit was created for this operator work.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib newmark_original_constraints -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib \
  --test transient_spring_constraint_admission --test transient_spring_reliability \
  --test transient_spring_balance --test dynamic_spring_closed_form \
  --test mechanical_constraint_admission --test thermal_frame_constraint_admission \
  --test magnetostatic_prescribed_workflow --test transient_heat_bar_reliability \
  --test advection_diffusion_bar_reliability --no-fail-fast \
  -- --skip modal_frame_spectrum:: --quiet
```

## Limits

This admits the restored increment in the assembled f64 effective system. It is
not an independent assembly implementation, a forward-error or time-integration
error bound, or a general physical certificate for the final dynamic state.
Nonlinear tangents, harmonic/complex, modal/buckling and other formulations need
their own original-equation contracts. Existing shared range rejection remains
conservative rather than arbitrary precision. Cooperative cancellation and
same-input replay do not establish process-kill, power-loss or durable recovery.

This is bounded original Newmark-increment equation admission, not general dynamic qualification, forward accuracy, a physical reference certificate or durable restart.
