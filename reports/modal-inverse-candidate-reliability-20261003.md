# Modal Inverse Candidate Reliability

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. The 128-element bending probe exposed a regression within
the existing four-step inverse refinement: its first-mode true relative
residual was about `2.496865e-8` after one correction, but rose to
`3.737118e-8` after two and ended at `2.638575e-8` after four. The old code
overwrote the retained eigenpair even when a correction made it worse.
A new regression failed against that behavior before the fix.

The first attempted fix rejected every uphill search step. That was too
restrictive: six public mass-coordinate tests failed, including spatial
paired bending and coordinate scaling that previously passed. That approach
was withdrawn rather than relaxing the original `1e-8` refinement gate.

The implemented fix separates the private search iterate from the best
checked eigenpair. Search may traverse temporary residual regressions within
its existing budget, while only an improvement replaces the retained pair.
The unresolved 128-element fixture still returns failure, not a supported
or qualified spectrum. This is a bounded recovery fix, not an expanded
numerical capability claim.

## Implementation Boundary

Inverse correction, orthogonalization, Rayleigh evaluation and residual
smoothing run on private candidate vectors. A failed or cancelled candidate
does not partially overwrite the retained value and vector. When previous
modes require deflation, the baseline is the fully checked projected seed:
an unconstrained seed is not a valid comparator for that subspace.

The smoothing helper returns the already checked residual of its selected
pair. The caller uses it to retain an improvement without another matrix
product. The helper's early exits return the unchanged seed residual. Tests
compare the returned bits with an independently recomputed operator residual
for sampled improved and unchanged candidates; this arithmetic check is
distinct from an independent physical reference.

The search still permits at most four inverse corrections and one Richardson
smoothing candidate per correction. The prepared inverse is reused within
the request. Tolerances, normalization, spectral-neighborhood rejection,
final spectrum validation and published physical-shape validation are
unchanged. There is no extra iteration, dependency or cross-request cache.
Retaining the best pair adds bounded vector-copy workspace, not a history
of search snapshots. No throughput or wall-clock improvement is claimed.

All numerical logic remains in Solver. Engine and Agent continue to execute
their existing routes rather than implementing another refinement algorithm.
A failed public solve discards its internal candidates; the retained pair
is not a resumable result or partial successful spectrum.

## Regression Coverage

The 128-element kernel fixture compares budgets zero through four. Its
retained true residual cannot worsen as the budget grows, while the first
correction must still improve the original seed by more than a factor of
100. Equal-residual retention preserves both value and vector. The fixture
also requires explicit budget exhaustion, exactly one preparation and dense
factorization for nonzero budgets, and one smoothing attempt per correction.

Cancellation after inverse substitution now preserves the complete seed
pair rather than only the unprocessed vector tail. A further regression
reaches a fully corrected private candidate and cancels during smoothing's
scan, compensated dot and update stages after 64 components. Each call
fails within the operation and a fresh call passes the original gate.
A zero-direction correction also fails without overwriting the seed and
does not poison a fresh valid request.

Existing public tests remain necessary: repeated modes, mass orthogonality,
node/member ordering, coordinate scaling, low/high/complete bending spectra,
physical-shape residuals and error/replay exercise the successful search
trajectory. In particular, they prevent best-pair retention from becoming
the rejected monotonic-search restriction. These lanes overlap and must
not be counted as independent coverage percentages.

## Validation Results

| Lane | Result |
| --- | --- |
| Full Solver suite including integration and doc harnesses | 1337 passed, 0 failed, 9 existing ignored; 186 result groups |
| Refinement target | 24 passed, 0 failed, 0 ignored; includes three new regressions |
| Public mass-coordinate target within the full suite | 30 passed, 0 failed, 0 ignored |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Headless modal operator target | 35 passed, 0 failed, 0 ignored |
| Strict Solver and CLI Clippy on all targets | Passed with warnings denied |
| Formatting and whitespace | Passed |
| Operator profile registration and self-test | Passed for 59 profiles; executed=false |
| Tensor structure and commands | Passed; 13 modules, 11 paradigms, zero structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development and shipping 3.4.4 |
| Source 800 and document 2000 organization audit | Passed; zero tracked debt |

The original 128-element public and headless failure/replay cases pass;
passing a rejection test does not count as a successful 128-element solve.
The successful public lanes retain physical residuals, repeated directions,
mass orthogonality and coordinate/order invariance. No ignored test was added.

Live TCP Agent replay is not verified in this round. The sandboxed attempt
returned one passing harness test and twelve permission failures because
local TCP listeners were not allowed. Two subsequent approval review
attempts ended in transport errors before execution; no elevated Agent test
ran. These are environment limitations, not numerical qualification or
evidence that Agent recovery passed. The full executed modal profile was
not rerun because it includes that same blocked TCP target. Earlier executed
profile counts belong to their historical source overlays.

Daji readiness remains blocked with four maturity gaps, sixteen
evidence-grade gaps and eleven P0 gaps. The new tensor claim is scoped to
Solver numerical validation and recovery, not live Agent execution or
broader operational qualification. The focused, full-suite and headless
lanes overlap and must not be summed as a coverage percentage.

## Reproduction

From `workers/rust`, run the focused target:

```sh
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement --locked --offline -- --nocapture
```

From the repository root, run the native modal profile and validate its
saved execution report:

```sh
CARGO_NET_OFFLINE=true ./scripts/kyuubiki check-operator-validation --execute --profile modal-frame-sanity --out tmp/modal-inverse-candidate.json
CARGO_NET_OFFLINE=true ./scripts/kyuubiki check-operator-validation --in tmp/modal-inverse-candidate.json --profile modal-frame-sanity
```

The executed profile requires local TCP listener permission for its Agent
lane. Registration checks alone do not substitute for execution. Local raw
logs remain temporary artifacts rather than retained qualification.

## Source Fingerprints

SHA-256 values are relative to `workers/rust/crates/solver/src/`:

| File | SHA-256 |
| --- | --- |
| `modal_frame_refinement.rs` | `d2cdd54661b665f2c22007d5f838c41743175c5e9c7a5b4a49d1c60e4fef5624` |
| `modal_frame_resolution_tests.rs` | `f662a195aa5cabec1f6210ef6fde546c43c82dca0c6290a2964081e95d6ef0a8` |
| `modal_frame_refinement_tests.rs` | `023cc9c9bf5e3ab425fce082d4b2f1fee68217b15dd7a37af5549a0b759e711d` |
| `modal_frame_refinement_control_tests.rs` | `8b44ac0ed8ad4d2a9e1ccda6292a0088ef6f49d2342881df5433bc8005326447` |
| `modal_frame_polish_tests.rs` | `a787b547a279ccc8e08b267825b3c4deb6e185cf9008a60f0d08eea3d0d5a813` |

## Remaining Boundaries

This is bounded inverse-candidate retention verification, not general modal or Agent qualification.
The best 128-element first-mode residual remains above the original gate;
no numerical tolerance was relaxed to convert it to success. Private search
can still exhaust its budget or fail explicitly on an invalid direction.
Cooperative safe points do not certify wall-clock allocation latency.
This round adds no installed-package, remote Linux, million-DOF bending or
industrial correlation claim. Source metadata remains 3.4.4, with no package
rebuild, reinstall, commit or push in this round. Earlier reports retain
their historical validation counts and source fingerprints.
