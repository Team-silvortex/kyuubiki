# Bounded Modal Grid Portfolio

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only aggregate portfolio evidence, not runtime admission or general modal qualification.

The preceding fixed-grid comparison established individually certified f64
candidates but exposed coordinate-order dependence and lacked an aggregate
multi-fit budget. The new test-only entry tries three fixed order policies
under one budget and one publication boundary. It is not connected to public
Solver, Headless, Engine or Agent dispatch. No numerical tolerance, dependency,
public API, executable format, installed application or version is changed.

## Candidate And Failure Contract

The seed must satisfy the unchanged unit-norm contract, have resolved nonzero
coordinates and match bounded square directions and positive finite masses.
The largest `abs(shape[i]) * sqrt(mass[i])` selects the anchor; equal amplitudes
retain the lowest active index. This tie policy is deterministic, not an
assertion of invariance under arbitrary coordinate renumbering.

The fixed order sequence is reverse index, natural index, then ascending
grid-column norm with index tie breaking. Every attempt starts from the same
private immutable seed and frozen anchor, not from a previously rejected
candidate. Grid norms use scale-safe vector norms of the actual rounded
grid-scaled columns. All three orders are prepared before fitting.

Each fit retains four correction passes, original-column in-backsolve integer
quantization, maximum radius 4,194,304 and at most six actual certificates.
Only typed `Residual` and `UnitNorm` rejections permit another order. Operator
errors, malformed receipts, factor/range failures and cancellation propagate
immediately. An error containing residual/norm-gate words still remains an
error: there is no string-matching fallback classification.

An accepted fit releases its QR factor before one extra original-operator
certificate. The final shape must still satisfy unit norm and exact anchor
bits. Cancellation checkpoints precede the extra certificate and final return.
A failed extra certificate ends the portfolio; it cannot cause another attempt
or publish the earlier candidate. No renormalization or eigenvalue replacement
is performed. The caller's original physical certificate remains authoritative.

## Aggregate Resource Contract

| Boundary | Limit |
| --- | --- |
| Active coordinates | 2 through 256 |
| Coordinate-order attempts | At most 3 |
| Simultaneously retained QR factors | 1 |
| Correction passes per fit | At most 4 |
| Integer radius | 4,194,304 |
| Actual certificates for the complete portfolio | At most 19 |
| Modeled aggregate proposal work policy | 1,050,000,000 component visits |
| Modeled payload policy | 8 MiB |

The nineteen-certificate bound includes three worst-case six-certificate
fits plus one final publication certificate. A single monotonic counter is
charged before every callback. Attempt reservations never reset either
counter, and repeated calls after exhaustion remain exhausted.

For size `n`, per-fit payload/work retain the earlier formulas
`48*n*n + 2048*n` and `12*n*n*n + 2048*n*n`. The portfolio adds
`32*n*n + 256*n` to peak payload for shared input and order bookkeeping,
and `32*n*n + 128*n` to three per-fit work reservations for preflight/order
construction. At 256 coordinates the models give 5,832,704 bytes and
1,008,762,880 visits. Factors are prepared and released sequentially.

The aggregate work policy is explicitly distinct from the earlier per-fit
350,000,000 policy; it is not a hidden increase to a production solve budget.
These are static structural planning models, not measured allocations,
allocator audit, peak RSS, FLOP counts or a real-time deadline. They exclude
the caller's physical assembly/direction construction, diagnostics, independent
test reassembly and actual certificate work. Actual callback count is enforced,
but its numerical cost still depends on the caller's model.

## Independent Physical Results

The existing 128-element fixture supplies three segment lengths: `1`, `1e14`
and `1e-10`. Each is tested under identity, reversed and translation-then-
rotation numbering, and both positive/negative signs, eighteen combinations.
Mass and direction rows/columns follow the same permutation. Every callback
restores original physical coordinates before applying the original operator;
only the residual fed back to QR is remapped.

The original first eigenvalue is retained. Every final shape passes a separate
closed-entry physical reassembly, unit norm and complete constructed modal
result JSON readback. These records are candidate records, not public solve,
Headless or Agent output. The automatically chosen anchor maps to original
active index 252 in every sample and retains its exact original bits.

| Segment length | Identity, reverse order | Reversed numbering, natural order | Family numbering, reverse order |
| --- | --- | --- | --- |
| 1 | 7.794966084968067e-9 | 7.794966084968067e-9 | 2.8458950486887474e-9 |
| 1e14 | 8.109485283404323e-9 | 8.109485283404323e-9 | 3.4053948417862804e-9 |
| 1e-10 | 6.358817804510388e-9 | 6.358817804510388e-9 | 3.27235705951954e-9 |

All eighteen samples pass the unchanged `1e-8` physical gate and existing
strict `abs(norm - 1) < 1e-10` gate. Both signs have matching classifications
and residuals. Identity/family samples use one factor and four actual
certificates; reversed numbering rejects reverse order, then passes natural
order with two factors and seven certificates. The third order is exercised
by independent controls, not by an accepted beam sample in this run.

This resolves the sampled numbering failure through an explicit bounded
portfolio. It is not a proof that all permutations, anchor ties, geometries
or unit-shape seeds admit a candidate within these three policies.

## Failure And Recovery Controls

Seven tests cover the eighteen beam samples and the following independent
controls:

- A closed identity system recovers an exact one-ULP target while retaining the
  anchor. A larger representable target passes residual but fails unit norm;
  all three policies reject it without normalization or returning a shape.
- Injected monotonically decreasing receipts force all four passes of all
  three fits, observing exactly eighteen callbacks and three factors. These
  receipts test counter behavior, not physical validity.
- A separate injected receipt sequence first passes in the third policy and
  uses all nineteen callbacks, including the extra final certificate. It
  validates the successful budget boundary, not a new physical fixture.
- Direct counter tests prove the nineteen-certificate cap, aggregate work
  reservation and sticky exhaustion without resets.
- Operator faults, including numerical-gate words, and malformed initial
  receipts stop after one callback. A fresh invocation then succeeds.
- A failing, malformed or error-valued extra final certificate ends after
  three callbacks for an initially valid seed, with no further attempt.
- Invalid dimensions, masses, seeds, matrices, rank and tolerance fail before
  the physical callback.
- Cancellation at preflight, factorization, backsolve, the second attempt,
  pre-final certification and the final return yields no shape and permits a
  fresh valid invocation. The seed remains intact.

## Verification

| Check | Result |
| --- | --- |
| Focused portfolio tests | 7 passed, 0 failed, 0 ignored |
| Final complete `triangular_grid_` replay | 19 passed, 0 failed, 0 ignored |
| Full Solver regression | 1,464 passed, 0 failed, 9 original ignored; 186 result groups |
| Rust Headless modal integration | 37 passed, 0 failed, 0 ignored |
| Solver/CLI all-target strict Clippy | Passed with `-D warnings` |
| Workspace formatting and patch whitespace | Passed |
| Tensor/profile validator and self-tests | Passed; 59 profiles, registry-only `executed=false` |
| Documentation book/inventory | Passed; 26 HTML files, development/shipping 3.4.5 |
| Project organization | Passed; source limit 800, doc limit 2,000, tracked debt 0 |

The full run began before the type-only named-order-container cleanup and
the additional nineteen-certificate success assertion. Final replay of all
nineteen triangular-grid tests checks those final edits. Overlapping suites
and eighteen transformations of three beam fixtures are not additive physical
coverage. Native registry checks do not execute qualification. Tensor counts
remain 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps; qualification
remains blocked.

Commands from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib triangular_grid_portfolio_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all --check
```

Remaining work is broader geometry/numbering qualification, measured aggregate
time/memory, and a separately reviewed production integration with the actual
public physical publication path. The public tiny-coordinate failure remains
unchanged. No live Agent, installed/remote run, new benchmark, packaging or Git
submission is claimed; tensor maturity and release qualification gaps stay open.

Follow-on [canonical grid and cost evidence](modal-grid-canonical-cost-reliability-20261004.md)
retains the broader raw numbering failures and an alternative canonical entry,
plus isolated optimized candidate timings/RSS. Those measurements and expanded
cases are separate from this original run and do not enable production admission.
