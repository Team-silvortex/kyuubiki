# Prescribed-Value Consistency Across Chain and Sparse Solvers

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[sparse reduction repair](sparse-prescribed-reduction-reliability-20261023.md).

## Reproduced difference

The cross-backend conflict test failed before this repair. On a three-node
chain with zero loads, assigning a fixed DOF first `0` and then the smallest
positive subnormal `q` returned success and kept `q`. The sparse reducer
rejected the same assignments as conflicting.

The chain entry used `abs(previous - value) > 1e-12`. That absolute tolerance
could treat distinct input data as identical, including adjacent normal f64
values near `1`. Which assignment appeared last could change the effective
boundary condition. The sparse entry already required numerical equality.

## Policy and implementation

Distinct finite assignments to the same DOF are conflicts; signed zeros compare numerically equal.

The chain's duplicate comparison now uses finite numerical equality, matching
the sparse entry. Finite-value and index validation still precede assignment.
Equal repeats remain accepted. Different DOFs retain independent values and
can differ by a single floating-point interval or a subnormal quantum.

This is a private input-contract correction. Callers that previously relied on
near-equal duplicate assignments being overwritten must resolve the conflict
before solving. Solution tolerances and numerical range policies are unchanged.
The implementation changes one comparison and adds no runtime allocation,
lookup table, model conversion or new solver stage.

## Added regression coverage

Five cross-backend unit tests exercise the actual path assembly/solve entry
and the independently assembled sparse reduction/solve entry:

- Six magnitude pairs, both assignment orders: zero/quantum, opposite quantum
  signs, adjacent values at the minimum normal boundary, adjacent values near
  positive/negative one, and adjacent values near the largest finite f64.
- Equal repeats versus one assignment at zero, tiny, normal and large values;
  both signed-zero orders and largest-finite all-fixed controls.
- Separate DOFs with adjacent normal values or a two-quantum boundary
  difference, checking the retained endpoints and resulting solutions.
- NaN, positive/negative infinity and invalid indices in repeated assignments.
- 128 identical assignments, cancellation at indexing entry, step 64 and
  completion, followed by clean reuse through each entry.

This checks duplicate-input identity for nonempty test systems. Backend range
capabilities, empty-system behavior and physical accuracy remain separate
contracts. The high-level operator models derive unique constraints from
nodes; these tests directly exercise the shared internal entry points used by
those models rather than inventing a new public duplicate-assignment field.

## Validation

Completed on the final source with Rust 1.88.0 and offline locked dependencies:

| Scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions per platform |
| --- | ---: | ---: | --- |
| Solver library | 694 | 694 | 9 ignored; 292 modal-spectrum tests filtered out |
| Engine library | 711 | 711 | 1 ignored |
| 10 related integration targets | 32 | 32 | None |
| Total | 1,437 | 1,437 | 10 ignored; 292 filtered out |

The five new tests are included above. Repeated focused runs are not added
again. This round ran the related integration subset listed below; it did not
repeat all 67 targets from the preceding sparse-reduction report. The long
modal-spectrum matrix and opt-in benchmarks were not run.

Local solver/engine all-target Clippy with warnings denied, workspace formatting
and patch-whitespace checks passed. Affected source/configuration files remain
within 800 lines. Existing local and remote caches were reused; no new build
root, numerical dump or application archive was created.

Native operator-validation metadata passed for 77 profiles with
`executed=false`. Tensor structure/command checks passed for 13 modules and
11 paradigms, with 0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps. Overall readiness remains `blocked`. These registration checks
are separate from the numerical executions above.

## Reproduction

From the repository root with the pinned toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib prescribed_consistency -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine \
  --test magnetostatic_prescribed_range --test magnetostatic_prescribed_workflow \
  --test heat_bar_closed_form --test electrostatic_bar_closed_form \
  --test acoustic_bar_closed_form --test acoustic_bar_reliability \
  --test magnetostatic_bar_reliability --test advection_diffusion_bar_reliability \
  --test transient_heat_bar_reliability --test thermal_bar_1d_closed_form \
  --no-fail-fast -- --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
```
