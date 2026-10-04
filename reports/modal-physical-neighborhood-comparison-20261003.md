# Modal All Coordinate Neighborhood Comparison

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only all-coordinate neighborhood diagnosis, not new runtime admission or general modal qualification.

Removing fine-coordinate elimination and searching groups of up to four
physical coordinates produces only a small improvement in the unresolved
tiny-coordinate long-beam fixture. It does not reach the unchanged `1e-8`
physical residual gate. The neighborhood cannot replace continuous block
correction: from the physical-polisher seed alone all sampled scales reject.
Already-qualified block-fit candidates remain unchanged and are freshly
revalidated. Production algorithms, tolerances, formats, dependencies and
version remain unchanged.

## Candidate Search And Acceptance

The experiment reuses retained-wide physical directions for
`D * (K - lambda M)`, with `D = diag(1 / sqrt(mass))`, and keeps every physical
coordinate available. It has no fine/coarse partition, projected directions
or linear correction factor. A cached double-double Gram table and fresh
residual dots rank simultaneous next-up/next-down changes.

For a candidate delta, the score difference is
`2 * delta' * A' * r + delta' * A' * A * delta`.
The constant residual square is not recomputed for each candidate. This is a
proposal model, not an actual-operator certificate or interval bound.
An independent binary-entry example checks the accepted move against closed
products rather than reusing the cached score.

Width one uses singles. Width two adds the existing bounded correlation-pair
graph. Width three or four adds sorted sliding windows and, for each graph
root with enough neighbors, a group containing its strongest neighbors.
There are at most `7*n` groups and at most 80 nontrivial combinations per
group. Each width defines its own deterministic nomination policy; this is
not exhaustive search over all coordinate subsets or wider ULP radii.

Four passes are allowed. Each family runs forward and reverse. Each such
sweep starts from the retained candidate and its fresh actual residual.
A new candidate is retained only if the true physical relative residual
strictly decreases. That certificate also replaces the predicted residual
before the next sweep. One check counter covers initial, sweep and final
certificates, with at most 26 callbacks inside this search. Preparing the
input and producing a prior block-fit seed are outside that callback count.

## Long Beam Results

All fixtures have 128 elements and 256 active bending coordinates. They use
the actual rounded segment lengths, physical stiffness and mass, the same
production first eigenvalue, and the same four-sweep physical-polisher seed.

The following residuals start directly from that physical-polisher seed.
Every entry rejects at `1e-8`.

| Maximum group width | Segment length 1 | Segment length 1e14 | Segment length 1e-10 |
| --- | --- | --- | --- |
| One | 2.1756674980185087e-8 | 2.0124784181695625e-8 | 1.7403597139757606e-8 |
| Two | 2.175451839714863e-8 | 2.0119241200366236e-8 | 1.7399966774554802e-8 |
| Three | 2.175451839714863e-8 | 2.0119241200366236e-8 | 1.739996605185335e-8 |
| Four | 2.175451839714863e-8 | 2.0119239147117738e-8 | 1.739996605185335e-8 |

A second comparison starts from the best candidate actually checked by the
existing automatic block fit. Its unit-coordinate residual is
`9.205252983779463e-9`, and its large-coordinate residual is
`7.362945994602734e-9`. Those seeds already pass, so all four widths leave
their exact coordinates unchanged; each requires two fresh certificates.
They also pass unit-shape norm checks, complete constructed result JSON
bit-preserving readback and independently reassembled physical residuals.
These are local candidate records, not a new public solver route.

The tiny-coordinate block seed starts at `1.7404270550916884e-8`. All four
widths reach the same respective tiny-coordinate values shown above and
still reject. Expected failures assert both their error category and their
residual band, and confirm that the final failure reports the best true
certificate observed within that search. No failed candidate is published.

These observations do not establish an f64 residual floor or prove that no
admissible shape exists. The small local improvement narrows this proposal
family; a wider or nonlocal selection policy remains a separate hypothesis.

## Independent Barrier And Recovery Checks

A five-coordinate closed-entry system isolates a four-coordinate move.
Its four coupled columns have entries drawn from zero, plus/minus one and
one eighth. The final residual starts at minus half an ULP. Changing all four
coupled coordinates upward by one ULP makes every residual entry exactly
zero; singles, pairs and triples cannot reduce the initial score.
All neighboring combinations involving at most three of the four coupled
coordinates are also enumerated against those independent closed products.

Widths one through three retain relative residual one and reject. Width four
recovers the known exact rounded coordinates and zero residual, across three
sampled coordinate permutations and both signs of the seed. This validates a
real simultaneous-search effect, not general basis invariance or physical
modal qualification.

Control tests force all 26 certificate calls and verify their monotonic
counter. An initially passing candidate with a failing or malformed final
certificate rejects. A proposal that predicts improvement but receives a
worse actual certificate is discarded. Operator errors propagate, malformed
input dimensions/ranges fail before the callback, and failed searches leave the
borrowed seed intact. Preparation and cached search cancellation cover their
available checkpoints, including final publication validation, with fresh
control replay after each cancellation.

A separate callback trace accepts one coordinate move, then supplies a new
zero residual while its relative certificate remains above the gate. Every
later candidate retains that first move without repeating the old correction.
This tests residual reanchoring directly, not just the check count. Malformed
initial certificates and an operator failure after the initial check also
reject without affecting the borrowed seed. Synthetic control traces are not
physical acceptance evidence.

## Resource Limits And Verification

Dimensions remain 2 through 256; width is 1 through 4. Nonzero input heads
for directions, seeds and certificates must remain within `1e-50..=1e50`,
with finite bounded high/low components. Private predicted residuals retain
the existing wider component guard. These conservative ranges bound this
test experiment, not all valid modal requests.

At 256 coordinates a separate model allows 5,242,880 bytes of numeric and
container payload and 218,103,808 component visits, below ceilings of 8 MiB
and 250,000,000. The model covers direction/Gram/graph storage, cached score
work and bounded sweeps; it excludes caller-owned physical models, shared
physical-direction construction and actual certificate computation. The
callback cap is checked separately. These are structural models, not a wall
time benchmark, instruction count, allocator audit or measured peak RSS.

Six retained `physical_neighborhood_` tests pass. Strict all-target Solver/CLI
Clippy passes with no warnings.

Full Solver regression passes 1,437 tests with zero failures and nine
pre-existing ignored tests across 186 result groups; the six new tests are
included in that total. The strengthened six-test filter also passes after
adding the closed-product enumeration and residual-reanchoring assertions.
Headless modal integration passes all 37 tests. Workspace formatting checks
and native tensor, operator-validation, documentation and organization audits
pass; tensor and operator-validation self-tests also pass. Operator-validation
checks verify the registry without executing its commands. The tensor retains
four maturity gaps, 16 evidence-grade gaps and 11 P0 gaps, with daji
qualification still blocked. No broader readiness gap is promoted by this
test-only diagnosis.

Commands from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib physical_neighborhood_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

No live Agent, installed/remote qualification, packaging, version change or
Git submission is claimed. Source/document limits remain 800/2,000 lines.
