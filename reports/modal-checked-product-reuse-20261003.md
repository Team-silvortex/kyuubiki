# Modal Checked Product Reuse

## Scope And Findings

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. Dense modal refinement recomputed the same physical-operator
product and relative residual at loop entry even after both had been checked
for the preceding private candidate. This follow-up removes that duplicate
work without changing the eigensolver arithmetic or its acceptance gates.

Two test-only numerical experiments were also evaluated and removed. Four
forward/reverse coordinate sweeps reduced the sampled 128-element first-mode
relative residual from about `2.496865e-8` to `2.100842e-8`; four two-direction
residual minimizations ended at about `2.396165e-8`. Neither met the original
`1e-8` gate. These exploratory results do not prove a floating-point floor,
validate the other modes or justify expanding the supported spectrum.
Neither experimental algorithm remains in production or the test suite.

## Implementation Boundary

The private `CheckedResidual` holds the actual operator product and relative
residual belonging to the current search iterate. The smoothing helper
returns the product of the selected candidate when it improves, or the
original product when it does not. Ownership moves the product into the
next iteration without another product, rescan or norm reduction.

Deflating a new mode against earlier directions changes the vector and
invalidates its original product. That branch still recomputes the product,
Rayleigh value and residual before using the projected seed. Each loop
still polls `ModalIteration` before accepting convergence or starting the
next correction. Cancellation, invalid directions and budget exhaustion
retain their explicit failure behavior; no state survives the request.

Final spectrum validation and published physical-shape validation still
perform their own operator products and residual checks. Reuse applies only
to an unchanged private iterate within Solver, not to those independent
boundaries or to an Engine, Agent, SDK or cross-request result cache.
Residual gates, factor reuse, four inverse corrections and one Richardson
candidate per correction are unchanged.

## Work Counts And Regression Coverage

For the unresolved 128-element kernel fixture, the observer verifies one
initial product plus three products per inverse correction. Budgets zero
through four therefore use `1, 4, 7, 10, 13` modal products. The previous
control flow used two initial products and four per correction, or
`2, 6, 10, 14, 18`; that baseline count is derived from the previous loop,
not a retained wall-clock benchmark. The four-step case eliminates five
duplicate products and their residual scans, not five inverse solves.

A new diagonal work-count test failed against the previous implementation
before reuse was added. Existing accurate-seed behavior remains unchanged:
one validation product and no prepared inverse. Sampled helper products and
relative residuals match independently recomputed bits for improved,
converged, zero-step and fully evaluated rejected candidates. The rejection
lane covers near-analytic two-by-two eigendirections at common scales
`1, 1e200, 1e-200`, and requires reaching an actual evaluated rejection.

A two-mode diagonal test verifies product refresh after deflation, correct
roots and independently evaluated residuals and orthogonality. Another test
reaches a checked converged state, cancels at the next iteration boundary
before acceptance, and requires a fresh request to recompute the same pair.
The 128-element fixture also retains its preparation, smoothing-count,
best-pair and explicit budget-exhaustion assertions.

These checks establish bounded work reduction, not whole-solver throughput,
memory peaks, release-build timings or million-DOF performance. The public
and headless physical-reference lanes remain necessary.

## Validation Results

| Lane | Result |
| --- | --- |
| Full Solver suite including integration and doc harnesses | 1341 passed, 0 failed, 9 existing ignored; 186 result groups |
| Refinement target | 28 passed, 0 failed, 0 ignored; includes four new regressions |
| Public mass-coordinate target within the full suite | 30 passed, 0 failed, 0 ignored |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Headless modal operator target | 35 passed, 0 failed, 0 ignored |
| Strict Solver and CLI Clippy on all targets | Passed with warnings denied |
| Formatting and whitespace | Passed |
| Operator profile registration and self-test | Passed for 59 profiles; executed=false |
| Tensor structure and commands | Passed; 13 modules, 11 paradigms, zero structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development and shipping 3.4.4 |
| Source 800 and document 2000 organization audit | Passed; zero tracked debt |

Final-validation safe-point tests remain separate from private product reuse.
The public and headless lanes still reject the unresolved 128-element
spectrum and accept a fresh supported request after failure. No ignored
test was added, and overlapping lanes are not summed as coverage percentages.

The live TCP Agent lane was not rerun following the preceding turn's local
listener permission failures and approval transport errors. This round
therefore makes no current Agent transport or full executed modal-profile
claim. Daji readiness remains blocked with four maturity gaps, sixteen
evidence-grade gaps and eleven P0 gaps. The new tensor claim is scoped to
Solver numerical validation and recovery, not general performance or
operational qualification.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
```

The registered native `modal-frame-sanity` profile includes the new regressions
through its existing refinement command. Registration is not execution;
its live Agent lane requires local TCP listener permission.

## Source Fingerprints

SHA-256 values are relative to `workers/rust/crates/solver/src/`:

| File | SHA-256 |
| --- | --- |
| `modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
| `modal_frame_resolution_tests.rs` | `2f3bcd9bd7e21c0d1d9bc78563f0dfe98bfd8612459d644dc22420266efa0c61` |
| `modal_frame_refinement_tests.rs` | `080c45b6a38b900a52f0f2a2f3c15e5cf620719b1b194ffac608ac01366b859c` |
| `modal_frame_refinement_control_tests.rs` | `f972c511ef3b8cc4c51698965791cc4f967ef48101b1218677dc1f4be2c24c98` |
| `modal_frame_polish_tests.rs` | `1bc85c0135d113246bda3be8ac9938fb56229703febbe90fa3ac81cd2e64b98f` |

## Remaining Boundaries

This is bounded checked-product reuse verification, not general modal or Agent qualification.
The 128-element request still rejects its unresolved spectrum. No tolerance
was relaxed, experimental candidate retained or industrial correlation
claim added. Source metadata remains 3.4.4, with no package rebuild,
reinstall, commit or push in this round. Earlier reports keep their original
source fingerprints and validation counts.
