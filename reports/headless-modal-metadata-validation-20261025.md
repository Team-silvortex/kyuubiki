# Modal Research Result Metadata Validation

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [retained result binding](headless-result-content-binding-20261025.md).
This strengthens the research example's independent validation, not Solver
numerics, generic SDK semantics, Agent execution policy or installed software.

## Reproduced Gap

The reference checked eigenvalues, shapes, residuals, mass orthogonality, Hz,
periods and mass, but ignored several other fields in the published result:
input echo, angular frequency, declared participation norm and frequency extrema.
Such fields are useful for attribution, display and downstream selection; an
acceptable eigensolution alone does not validate them.

Two red regressions used a real native Agent result and test-owned copies.
Removing its input echo still allowed publication into `verified`, and offline
verification accepted matching copied receipt/admission/result files with an
updated byte hash. Both tests failed on the old code before the repair. The
actual Agent ledger was never modified. This does not demonstrate that the
production solver emitted a bad result; it demonstrates a validator blind spot.

## Repair And Boundaries

The example's small `result_contract` module is called by the shared physical
reference, so direct checks, pre/post-staging validation and offline verification
all apply the same rules:

- The returned input must match the fully explicit supported input, including
  node/member order, geometry and material identifiers.
- Each mode's finite positive angular frequency must agree with the square
  root of its eigenvalue and its Hz value.
- The declared finite positive `participation_norm` must agree with the
  Euclidean norm recomputed from its finite shape components using `hypot`.
- Finite positive minimum/maximum frequencies must agree with extrema computed
  over all returned modes; missing and inverted summaries are rejected.

Derived numeric relations use a relative tolerance of `1e-12`, consistent with
the existing frequency-conversion check. Original modal spectrum/residual/unit
norm tolerances are unchanged. The result-content gate still uses exact typed
identity rather than this physical/derived-value tolerance. Input echo here is
semantic equality of this fixed, fully specified model, not a new raw-byte hash
or a general normalization rule for arbitrary solver inputs.

`participation_norm` is the published shape's Euclidean norm, not effective
modal mass or an engineering participation factor. The helper alone does not
establish a correct eigenproblem, constrained displacement, spectral ordering
or unit shape; those remain mandatory checks in the surrounding reference.
No production eigensolver/helper is imported by this independent checker.
Physical rules remain example-local rather than being added to generic SDK or
Engine/Agent code. No output schema or historical diagnostic policy changes.

## Regression Evidence

A shared test-only fault table creates 38 cases: missing/null/text/negative/wrong
values for input echo, frequency bounds, first/last-mode angular frequency and
declared norm, plus wrong material identity, wrong geometry and inverted bounds.

The publication regression rejects every case before creating a verified or
pending directory, preserves the raw diagnostic receipt and confirms that the
real Agent receipt is unchanged. The offline regression stops the Agent first,
then deliberately keeps copied admission/receipt/result contents and hashes
mutually consistent. All 38 cases fail at the metadata check, with file paths
and bytes unchanged. The original export remains accepted.

Five ordinary tests use manufactured metadata, explicitly not physical solver
evidence. They cover both supported input orderings, representable rounding,
every mode position, derived norms, empty/invalid data, extreme finite values
whose derived norm or conversion is unrepresentable, and summaries recomputed
from all modes rather than assumed first/last positions. Existing real tests
still cover both model orderings, independent modal physics, process restart,
lost replies, corruption and original-result binding.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target passed all 22 opt-in live Agent tests and its regular digest,
receipt, transport, file and metadata checks. The full SDK suite passed 172
tests, zero failures, with 22 live tests ignored by default and executed
separately. Counts include the existing no-op child fixture; overlapping runs
are not additive. All-target Clippy passed with warnings denied.

The native `modal-sdk-durable-research` profile passed with `executed=true`.
Documentation book/inventory, targeted Rust formatting, project organization
and `git diff --check` passed. Source/document limits remain 800/2000 lines with
zero tracked organization debt. Tensor structure passed with zero structural
gaps; two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps remain.

## Limits

This remains a bounded source-built local macOS synthetic reference. It does
not qualify arbitrary materials/geometries, installed runtimes, remote or
cross-language workflows, authenticity or power-loss recovery. Tensor evidence
remains `verified`, not `qualified`; general readiness gaps are not closed by
this repair. No installation, package bump, Git commit or push is included.
