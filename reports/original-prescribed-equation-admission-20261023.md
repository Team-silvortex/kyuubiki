# Original Prescribed-Equation Admission

Date: 2026-10-23

Version line: daji 3.6.x, after `520e3371` (`daji 3.6.9`).

## Contract

The new admission step checks the complete expanded vector against the assembled
pre-elimination equations, rather than trusting only a reduced-system residual.

- Every fixed value must equal its prescription, independent of the residual
  tolerance. Numerically equal signed zeros remain equal.
- Every free equation must satisfy componentwise backward error in the original
  assembled matrix and RHS. Row scales cannot be borrowed from other equations.
- Fixed equations are reaction equations: their residual is not required to be
  zero. Fixed loads and all expanded values must nevertheless remain finite.
- Prescribed inputs are exact data and receive no unknown-rounding allowance.
  Valid quantized free responses retain the existing row-local allowance.
- Original row overflow/underflow uses the existing compensated range evaluator.
  Irreversible alignment and unrepresentable raw residuals fail closed.

Unit negative controls show why the ordinary unconstrained residual gate is not
enough. A prescribed zero beside a very large coefficient can lend an invalid
rounding allowance to a wrong free response if treated as a rounded unknown.
The new gate rejects both ordinary and range versions of that construction.
It also rejects swapped free-DOF reconstruction and an inconsistent original
load after the reduced system itself has successfully solved.

## Integration

`PrescribedSystemValidator` borrows the original sparse matrix and stores one
validated prescribed-value index. The sparse reducer and validator share matrix
structure validation and duplicate/index checks. Each validation checks complete
dimensions, finite RHS/solution, exact fixed values and all free rows.

The gate is wired into all eleven current static call sites of the shared
prescribed sparse reducer: heat/electrostatic/acoustic/magnetic bars, triangle
and quadrilateral heat/electrostatic/magnetic planes, and quadrilateral electric
conduction. Heat planes are checked after contact-balance refinement. Plane
fields are checked in their existing reference-shifted coordinates before
restoring the absolute output reference; this avoids common-mode cancellation.

The prescribed chain driver now scans all free equations before publishing the
full vector, not just the boundary rows of each solved segment. It reuses the
same row evaluator and the existing chain tolerance. Earlier reduced-segment
and original boundary checks remain in place.

The non-path advection-diffusion dense fallback now retains a sparse copy of its
assembled original coefficients and the original RHS for this gate. Its existing
512-node limit is unchanged. No dense conversion was added to a sparse backend.

Transient heat retains the original effective matrix and reuses its validator
across steps. A separate full RHS buffer is updated from the previous state,
independently of prescribed-value elimination. The expanded current temperatures
are checked before any computed history frame is captured. The initial condition
is not incorrectly treated as a steady-state equilibrium requirement.

## Cancellation and Cost

Stages `constraint_validate` and `constraint_validate_row` append as IDs 86 and
87; IDs through `constraint_reduce_row` (85) are unchanged. Full-vector and row
completion scans check cancellation in existing 64-step batches. Wide ordinary
and compensated row passes check every 1,024 entries. Interruption returns no
successful partial result; the original matrix and validator remain reusable.

Sparse preparation adds one O(n+nnz) matrix check and an O(n) fixed-value index.
Each validation is O(n+nnz), using constant extra row state; it does not copy the
matrix, solution, or a whole residual vector. Chain validation uses fixed-size
row storage and its already allocated fixed mask. Transient heat additionally
keeps one O(n) full RHS buffer, allocated once rather than once per step.
The bounded dense transport fallback keeps O(nnz) original coefficients.
No runtime or peak-RSS improvement is claimed; the added verification has a cost.

## Added Coverage

Fifteen tests were added: seven sparse unit tests, four chain unit tests, one
stage-contract test and three consumer integration tests.

- Wrong original loads and swapped expanded free values after reduced success.
- Exact small/large prescribed identity, repeated values, signed zeros, fixed
  reactions, empty/all-fixed systems, malformed dimensions and hidden nonfinite
  data, invalid indices, unordered/duplicate columns and conflicting constraints.
- Fixed-zero rounding exclusion in ordinary and range paths, retained valid
  quantized unknowns, rejected erased loads and range/alignment failures.
- A 2,049-entry original sparse row, cancellation during vector validation,
  ordinary/range scans, the second range pass and final completion, then replay.
- Interior chain equations, fixed identity, malformed lengths and cancellation
  during the complete chain scan and final completion, then replay.
- Real magnetic path/junction dispatch and workflow cancellation at the new gate,
  followed by clean execution and identical workflow/direct artifacts.
- Per-step transient heat checks, interruption in the third step at admission
  entry and completion, and successful borrowed/owned replay with matching history.
- A real four-arm non-path transport network, cancellation in dense-fallback
  admission and a clean closed-form response with borrowed/owned parity.

## Validation

Final source validation used pinned Rust 1.88.0, offline locked dependencies,
local macOS AArch64 and the existing remote Linux x86_64 test copy:

| Scope | macOS passed | Linux passed |
| --- | ---: | ---: |
| Solver library, excluding `modal_frame_spectrum::` | 727 | 727 |
| Engine library | 711 | 711 |
| 67 solver/engine/CLI integration targets | 338 | 338 |
| Total | 1,776 | 1,776 |

Each platform had zero final failures, ten ignored library tests and 292 filtered
modal-spectrum tests. The fifteen new tests are included once in these totals.
The integration set includes 10,000-node prepared/transient cases, 52 live Agent
cancellation tests and the affected field/contact/workflow consumers.

Local solver/engine all-target Clippy passed with warnings denied; workspace
formatting and whitespace checks passed. Source/configuration files remain
within 800 lines and this report within 2,000 lines. Existing build roots were
reused. No retained numerical dump, package archive or service deployment was
created; only affected source/test files were synchronized to the Linux copy.

Native operator metadata validation passed for 80 profiles with `executed=false`;
registered-command validation is separate from the numerical runs above.
Coverage-tensor structure and command checks passed across 13 modules and
11 paradigms, with zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps. Overall readiness remains `blocked`; this registration does
not promote all constrained formulations to verified status.

An exploratory broad `original_` filter also matched long modal comparisons and
was interrupted; that partial run is not counted. The final bounded library
commands explicitly exclude `modal_frame_spectrum::`. An initial integration
assertion incorrectly expected the observer scope to retain a step-prefixed
cancellation string; it was corrected to verify the third invocation and the
existing standardized cancellation stage, without changing cancellation policy.

## Reproduction

From the repository root with the pinned toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib constraint_validation -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib original_constraint_stages -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --test magnetostatic_prescribed_workflow \
  --test transient_heat_bar_reliability --test advection_diffusion_bar_reliability \
  original_ --no-fail-fast -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
```

The broad integration scope is the same 67-target set in the
[sparse reduction report](sparse-prescribed-reduction-reliability-20261023.md#reproduction).
The three new consumer tests are inside its existing targets.

## Remaining Limits

This is a componentwise backward-error gate, not a forward-error certificate or
condition-number bound. Small responses beside large prescribed contributions
may still have poor relative accuracy while satisfying backward error. The gate
does not reconstruct exact real-valued assembly that f64 has already rounded.

The new context is available for further consumers, but homogeneous mechanical
reduction, nonlinear tangent increments, eigen/buckling formulations and other
independent constraint implementations were not globally migrated in this change.
They need formulation-specific admission rather than a blanket steady-state
equilibrium test. This work does not qualify physical models or durable restart.

This is bounded original prescribed-equation admission, not general forward accuracy, coverage of all constrained formulations, physical qualification or durable restart.

Follow-up: [homogeneous static mechanical admission](homogeneous-mechanical-equation-admission-20261023.md)
connects the shared coordinate-zero mechanical reconstruction sites. The limits
above describe this report's original change, not the later follow-up's scope.
