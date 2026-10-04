# Modal Hybrid Proposal Comparison

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is bounded hybrid proposal comparison, not new runtime admission or general modal qualification.

The production modal pipeline already combines dense spectrum seeds, four
inverse corrections, residual smoothing and a bounded coordinate fit. This
follow-up checks whether a different local factor or candidate family resolves
the remaining physical-publication failure at extreme coordinate scales.
It does not. The tested alternatives therefore remain test-only; no fallback,
residual relaxation, mode format, dependency, version or installed application
is added. The six-mode failure is not reclassified as success.

## Candidate Methods

All comparisons start from the same best private candidate produced by the
actual four-sweep physical polisher. That search is shared with production,
not reimplemented as a different experiment. A reduced straight-beam fixture
uses the real rounded segment lengths, production local stiffness and element
masses. Separate closed-entry assembly validates accepted physical results.

| Method | Candidate policy |
| --- | --- |
| Gram | Existing automatic partition, projected pairs and Gram/LU fine fit |
| Local QR | Same automatic partition, power-of-two column scaling and Householder QR |
| Gram then local QR | Switch once on stagnation, retaining the best private candidate |
| Gram then global QR | Alternate tall QR with one mass-weighted amplitude coordinate anchored |
| Grid shortlist | Single-coordinate and correlated-pair neighboring-f64 proposals, ranked approximately and checked with the actual operator |
| Amplitude probes | 16 upward and 16 downward neighboring-f64 global scale factors around one |

The two switch experiments share one 80-check counter and four outer search
iterations. Switching does not reset either budget or expose the retained
private candidate. QR preparation, factor application and substitution are
cancellable; a prepared factor survives failed or cancelled substitutions.

The grid shortlist has at most 16 real residual checks per pass and four
passes, plus initial and final certificates: at most 66 callbacks. A fresh
final certificate and cancellation poll precede return. Approximate matrices
and scores only propose or rank candidates, never certify numerical acceptance.
Amplitude probes are a separate 32-candidate experiment, not extra attempts
hidden inside a production fit. These experiments are not all run together by
the runtime. Their cost is not covered by the earlier runtime payload/RSS
model, nor by an installed or remote performance qualification.

## Numerical Results

The 128-element physical mode is checked at segment lengths `1`, `1e14`
and `1e-10`, retaining the same checked eigenvalue in every comparison.
The unchanged residual gate is `1e-8`.

| Candidate family | Length 1 | Length 1e14 | Length 1e-10 |
| --- | --- | --- | --- |
| Gram, local QR and both QR-switch variants | Pass, about 9.20525e-9 | Pass, about 7.36295e-9 | Reject, about 1.740427e-8 |
| Grid shortlist | Reject, about 2.175706e-8 | Reject, about 2.012404e-8 | Reject, about 1.740255e-8 |
| Best of 32 amplitude probes | Above gate, about 2.276395e-8 | Above gate, about 2.291847e-8 | Above gate, about 2.142632e-8 |

The grid search slightly improves the tiny-coordinate residual but does not
meet the gate. No unsuccessful family produces a successful result envelope.
Negative outcomes are explicit assertions, not permissive tests that pass
merely because a failed solve returned no data.

For each accepted family/scale sample, a constructed complete modal result
record retains the unit shape norm, exact shape bits and eigenvalue through
typed JSON readback. A separately assembled physical operator checks the
restored shape against the actual request geometry. This is constructed
record validation inside a candidate experiment, not new public solver,
Headless, Agent or installed admission for QR/grid candidates.

A separate analytic least-squares sample has two independent, nearly
parallel columns whose rounded Gram system is rejected as singular. QR
recovers its known coefficients and independently recomputed products.
Well-conditioned analytic samples retain solutions under three row orders,
three binary column-scale pairs and two sign pairs, including opposite
column powers of `2^-400` and `2^400`. Rank loss, nonfinite/ragged inputs,
scaling loss and above-bound dimensions remain explicit failures.

## Interpretation

Normal equations can amplify conditioning sensitivity; QR is a reasonable
alternative candidate generator, not an automatic cure. This motivation is
supported by the [LAPACK least-squares guide](https://www.netlib.org/lapack/lug/node27.html)
and the normal-equation accuracy discussion in
[LAPACK Working Note 149](https://www.netlib.org/lapack/lawnspdf/lawn149.pdf).
The analytic rank sample confirms that distinction locally. It does not
establish that Gram conditioning causes the target modal failure: changing
the factor or using a nearly full global correction still fails that sample.

The measured results narrow the next investigation to candidate representation,
physical normalization/coordinate scaling and higher-precision proposal
arithmetic. They do not prove that no ordinary-f64 candidate can meet the
gate, that mixed algorithms are ineffective in general, or that a higher
precision algorithm will fix this sample. Such changes must retain independent
physical residuals, modal norms, spectral position and multimode orthogonality.

## Verification

All completed commands below exited successfully on the stated local source
line. The retained `hybrid_` unit-test filter passes nine tests. The full solver
regression passes 1,414 tests across 186 result groups, with zero failures and
nine pre-existing ignored tests. The new hybrid tests are included in that
total, not added to it. The Headless modal integration suite passes all 37
tests. Strict Clippy reports no warnings, and formatting checks pass.

Tensor structure and self-tests pass with 13 modules and 11 paradigms. The
broader readiness gaps remain unchanged: four maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps; the daji qualification status remains blocked. Operator
validation registration and self-tests pass for 59 profiles with
`executed=false`. Registry and tensor validators check registrations and
evidence anchors only; they do not execute numerical commands themselves.
The documentation book and inventory checks pass, including 26 HTML files.
Organization checks retain zero tracked line-limit debt at the source/doc
limits of 800/2,000 lines, and the changed files also pass the whitespace check.

Commands run from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib hybrid_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

Native validation commands run from the repository root:

```text
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor --self-test
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --self-test
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation
workers/rust/target/debug/kyuubiki-script-runner check-doc-book
workers/rust/target/debug/kyuubiki-script-runner check-doc-inventory
workers/rust/target/debug/kyuubiki-script-runner audit-project-organization
git diff --check
```

## Remaining Work

- Test higher-precision proposal arithmetic without using that same arithmetic
  as its only acceptance oracle, and retain ordinary-f64 publication checks.
- Separate errors introduced by physical normalization, unit transformation,
  dense proposal matrices and final vector serialization.
- Retain spectral-index and mass-orthogonality references before promoting
  any joint multimode hybrid recovery.
- Measure real work and memory before offering a runtime algorithm policy;
  neither the extra QR factor nor a sequence of independent experiments is free.
