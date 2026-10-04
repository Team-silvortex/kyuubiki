# Long Bending Modal Roundoff Diagnosis

## Findings

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was
checked on macOS ARM64 to investigate the unresolved 128-element bending
case. The sampled first mode still fails the original `1e-8` relative
residual gate after increasing the test-only refinement budget from four
to sixty-four steps. Independent wider-arithmetic recomputation agrees
with the production compensated residual for the retained f64 vectors.

The eight-step retained direction is already extremely close to the wide
reference, while its relative residual remains above the gate. Rounding
an independently resolved wide reference vector to f64 also raises its
residual above that gate. These observations identify representation
sensitivity in this fixture; they do not prove a universal precision floor
or that no alternative f64 vector or correction could satisfy the gate.
The production budget, operator arithmetic and result format are unchanged.

## Model And Oracle Scope

The fixture assembles a restrained straight Euler-Bernoulli bending chain
with two free dofs per unit-length element. Increasing from 100 to 128
elements lengthens the beam, rather than refining a fixed-length domain.
The 128-element case has 256 free dofs and is not a capacity-limit failure.
The diagnostic sweep isolates its first mode, not its complete spectrum.

The native Rust oracle is compiled only under tests. It independently
assembles integer physical stiffness entries, uses a banded LDL factor and
thirty-two inverse iterations, and carries arithmetic in two f64 components.
There is no production factor or matrix-product call in this reference and
no additional dependency. Its TwoSum and FMA product transforms follow
[Ogita, Rump and Oishi](https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf).
This is a bounded double-double numerical reference, not arbitrary precision
or an interval proof. Norms of the wide residual components are finally
evaluated with f64 hypot; the cancellation has already been evaluated wide.

For comparison with the actual normalized operator `A = D K D`, the oracle
uses the exact stored f64 inverse-mass factors `D`. Its physical inverse
uses the wide effective mass `1 / D^2`. It does not silently replace those
factors with a different square-root approximation or rational mass.
The oracle is checked against known sum/product/division tails, known
physical linear solutions at 1, 3, 80 and 128 elements, and existing
independent discrete roots at 80 and 100 elements.

## Iteration Budget Results

Each budget restarts from the same Jacobi seed and retains the best checked
pair. Relative residuals use `norm(Aq - value*q) / max(norm(Aq), value*norm(q))`.
The direction comparison removes an arbitrary scalar by wide projection;
it is not a comparison of raw vector normalization or sign.

| Test budget | Production residual | Wide recomputed residual | Relative direction error | Success |
| --- | --- | --- | --- | --- |
| 0 | 3.4726711047e-6 | 3.4726711047e-6 | 1.2546053361e-9 | No |
| 1 | 2.4968650305e-8 | 2.4968650298e-8 | 3.1633146874e-11 | No |
| 4 | 2.4968650305e-8 | 2.4968650298e-8 | 3.1633146874e-11 | No |
| 8 | 2.3268370073e-8 | 2.3268370070e-8 | 5.7032466859e-17 | No |
| 16 | 2.3268370073e-8 | 2.3268370070e-8 | 5.7032466859e-17 | No |
| 32 | 2.3268370073e-8 | 2.3268370070e-8 | 5.7032466859e-17 | No |
| 64 | 2.3268370073e-8 | 2.3268370070e-8 | 5.7032466859e-17 | No |

The wide reference first root rounds to `4.60497244109418627e-8`.
After the first correction, the retained root differs by about one f64
relative rounding unit, `1.11e-16`. Accurate frequency alone therefore
cannot substitute for the vector residual check. The observed stagnation
through budget 64 does not rule out every different algorithm or budget.

## Representation Results

The following comparisons round one binary-scaled wide reference direction
component by component, without a search for an optimal representable
direction. The production and independent wide residuals agree after that
rounding. These are internal normalized vectors, not published physical
unit shapes or serialized successful outputs.

| Elements | Wide reference residual | After vector and root rounding to f64 | Production residual on that f64 pair |
| --- | --- | --- | --- |
| 64 | 5.63838e-25 | 3.0690260593e-9 | 3.0690260608e-9 |
| 80 | 1.47655e-24 | 6.6937510003e-9 | 6.6937509961e-9 |
| 100 | 3.51100e-24 | 1.5793827478e-8 | 1.5793827477e-8 |
| 128 | 1.15808e-23 | 4.4712111611e-8 | 4.4712111618e-8 |

The naive rounded 100-element reference exceeding the gate does not
invalidate the independently validated production 100-element solve:
the production algorithm searches and polishes a different f64 vector.
The same distinction prevents treating the 128-element naive-rounding
result as a lower bound over every representable vector.

A separate comparison rounds each independently evaluated normalized
matrix entry once, while retaining the wide reference vector. The sampled
128-element residual becomes `8.5623365765e-9`. This measures a different
matrix representation; it is not the production normalized-entry rounding
order, a replacement for the physical operator, or a successful output.

## Regression And Next Decision

Five retained tests cover the budget sweep, vector/matrix rounding
comparisons, scalar arithmetic tails, banded solve recovery of known
solutions, and existing discrete roots. Every failed diagnostic refinement
must report budget exhaustion, not an unrelated failure masked by a residual.
The existing public and headless negative/replay tests still guard the
unresolved 128-element spectrum without returning partial modes.

For this case, compensated residual evaluation is not producing a false
failure, and merely allowing 64 corrections does not resolve it. Improving
the eigenvalue alone or solving wide and immediately rounding the vector
is also insufficient for the sampled representation. The next algorithm
decision must evaluate representable-vector correction or precision carried
through the relevant representation boundaries, including published shape
normalization and JSON readback. This remains an investigation, not approval
of an extended-precision runtime or a wider transport contract.

The tensor evidence is scoped to Solver numerical diagnosis. No benchmark,
successful 128-element multimode solve, live Agent, installed runtime or
industrial qualification claim is added. No tolerance was relaxed, ignored
test added, or production iteration budget increased.

## Validation Results

| Lane | Result |
| --- | --- |
| Full Solver suite including integration and doc harnesses | 1346 passed, 0 failed, 9 existing ignored; 186 result groups |
| Refinement target | 33 passed, 0 failed, 0 ignored; includes five new diagnosis tests |
| Headless modal operator target | 35 passed, 0 failed, 0 ignored |
| Strict Solver and CLI Clippy on all targets | Passed with warnings denied |
| Formatting and whitespace | Passed |
| Operator profile registration and self-test | Passed for 59 profiles; executed=false |
| Tensor structure and commands | Passed; 13 modules, 11 paradigms, zero structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development and shipping 3.4.4 |
| Source 800 and document 2000 organization audit | Passed; zero tracked debt |

Overlapping test lanes are not summed as coverage percentages. The existing
public and headless 128-element negative/replay cases passed, not a positive
128-element solve. The live TCP Agent lane was not run in this round.
Daji readiness still has four maturity gaps, sixteen evidence-grade gaps
and eleven P0 gaps. No version bump, package rebuild, reinstall, commit or
push was performed. Older reports retain their historical source fingerprints.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement::resolution_tests::conditioning --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement --locked --offline
cargo test -p kyuubiki-solver --locked --offline
```

The registered `modal-frame-sanity` refinement command includes these tests.
Profile registration alone is not execution of its live Agent lane.
This is bounded first-mode roundoff diagnosis, not a successful 128-element spectrum or general modal qualification.

## Source Fingerprints

SHA-256 values are relative to `workers/rust/crates/solver/src/`.
The production refinement fingerprint is unchanged from the preceding round.

| File | SHA-256 |
| --- | --- |
| `modal_frame_resolution_tests.rs` | `b614d5826229c66a7c0aed25c30a9feadf8fdad0a4300910a355db18d576060d` |
| `modal_frame_conditioning_tests.rs` | `166874a9315d05ebc179c46ee2c5ab413ba0964343b4e45df23acc75a93d9018` |
| `modal_frame_conditioning_reference.rs` | `3b3247f3a4051a88a30e5ad36da3ff39adcd51a80215af48fa35b0491982a6b3` |
| `modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
