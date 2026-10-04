# Bounded Single-Mode Runtime Admission

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope

This is bounded single-mode runtime admission, not general modal or Agent qualification.

The preceding feasibility packets established separately checked ordinary-f64
candidates, including four-mode synthetic subspaces. This follow-up promotes
only the coordinate corrector and automatic partition into a bounded runtime
path. Joint multimode admission remains test-only. No format, dependency,
version, installed application or remote server is changed in this run.

## Runtime Boundaries

- Non-tridiagonal single-mode requests with at most 256 active DOFs use the
  complete dense spectrum, including the existing nonpositive-root rejection.
  The former eligibility bound was 128 DOFs; samples that formerly used a
  `1e-6` sparse gate in this range now require the dense `1e-8` gate.
- Recognized tridiagonal paths and larger sparse requests are unchanged;
  their own tolerances are not globally relabeled as `1e-8`.
- The existing inverse refinement keeps its four-step bound and retained
  best pair. Only an unresolved single pair may enter coordinate correction.
- The corrected eigenvalue is unchanged by the coordinate fit. The existing
  seed-relative root-neighborhood gate still applies; it is not a general
  spectral-index certificate.
- Correction uses rank/grid-weighted fine-column selection, one Gram factor,
  projected correlation pairs and private representable coordinate proposals.
  All acceptance checks use the actual compensated sparse operator, not the
  approximate proposal matrix.
- Physical publication retains its ordinary four-sweep correction first.
  If unresolved, one separately bounded physical-coordinate fit may follow.
  Its final real-operator certificate and cancellation poll precede assignment.
  The best ordinary private candidate is retained when a sweep regresses.
- Tests and runtime now share the same corrector/partition implementation.
  The former test-only reference modules retain their test registrations as
  imports, rather than a second evolving numerical implementation.

## Resource Contract

The preflight uses the actual operator dimension, not the borrowed vector's
claimed length. Invalid dimensions, mismatched data and ineligible operators
are rejected before the helper materializes a dense matrix.

| Boundary | Limit |
| --- | --- |
| Active DOFs | 2 through 256 |
| Outer coordinate iterations | 4 |
| Fine corrections per inner pass | 4 |
| Partners nominated per coarse column | 4 |
| Union of pair proposals | At most 4 times the coarse-column count |
| Single-coordinate trials | 3, forward and reverse |
| Pair-coordinate trials | 9, forward and reverse |
| Real residual callbacks inside one fit | Enforced cap of 80 |
| Additional final residual certificate | 1 per correction |
| Modeled additional payload policy | At most 8 MiB |
| Modeled component-visit policy | At most 350,000,000 |

For dimension `n`, the conservative payload model is `64*n*n + 512*n` bytes;
the component-visit model is `16*n*n*n + 1024*n*n`. At `n=256`, these give
4,325,376 bytes and 335,544,320 modeled visits. The dimension/loop/partner/
checked-product limits are enforced. The two formula estimates are static
planning bounds, not measured allocations, FLOP counts, peak RSS, a real-time
deadline or a bound on the caller's entire dense eigenspectrum. The existing
dense eigensolver has its own preparation/sweep controls. A single solve can
need both normalized and physical fits, each with its own local budget.

The cap is not increased after numerical failure. The checked-product counter
remains exhausted after repeated calls. Preparation/search/final-validation
phases expose distinct cooperative cancellation stages. Dense substitutions,
rank reorthogonalization, projection and trial-vector updates retain inner
cancellation checks. This is cooperative cancellation, not process preemption.

## Numerical And Recovery Results

The normalized 128-element first mode previously failed after four inverse
corrections. The shared runtime correction now reaches approximately
`7.5374773863e-9`, retaining the same checked eigenvalue bits. Instrumentation
observes one inverse factor, one Gram factor, 11 fit residual callbacks and one
final certificate. This is one fixture, not a worst-case cost measurement.

The full public 2D solve, normalization, publication and typed JSON readback
pass for eight samples: 80 and 100 elements at segment lengths `1`, `1e14`
and `1e-10`; 128 elements at lengths `1` and `1e14`. Independent reassembly
uses the actual rounded request geometry and no production assembly/product
helper. Unit-length cases additionally use the previous independent unit-beam
reference. This avoids mistaking a rounded decimal coordinate conversion for
an exact unit-geometry residual certificate.

Reversed node/member numbering passes for all three unit-length models,
retaining the first root and physical direction. Shape components retain
their exact bits through JSON readback. Root restraints, inactive axial
components, participation norm and frequency/period consistency are checked.
Two spatial single-mode requests at 40 and 64 segments also retain independent
physical residuals after full public JSON readback, with planar/spatial first
roots cross-checked. This is not an arbitrary 3D coupling or unique-direction
certificate for repeated clusters.

The 128-element `1e-10` coordinate case remains a negative reference. Its
physical corrector stalls at about `1.7404270551e-8` and returns an explicit
physical roundoff-recovery error on owned and borrowed public routes. It does
not fall back to the former `1e-6` gate. The 128-element six-mode request also
retains its original four-inverse-step failure; multimode correction is not
enabled. Fresh valid calls replay after both failures.

Cancellation is injected at all three new stages before normalized admission,
and separately after normalized admission inside physical publication. Neither
boundary returns a partial result. Fresh control scopes replay the same result.
Invalid certificates, operator/vector mismatches, dimensions above the bound
and exhausted counters retain explicit negative tests.

## Headless And Sparse Boundaries

Two new Rust Headless plan-to-Engine tests carry the actual 128-element models
through the existing bridge manifest. JSON readback is independently rechecked.
The physical tiny-coordinate rejection and six-mode rejection survive this
route. All three new phase cancellations return errors and then replay valid
results. This is the native SDK plan/Engine bridge, not live Agent RPC, a
source-detached installed SDK or Python/Elixir execution.

Sparse-only fixtures are increased beyond the new complete-check threshold,
rather than deleting their path assertions: connected row-pair fixtures use
258 DOFs, the repeated spatial subspace fixture uses 288, and the rotated
transverse-versus-axial fixture uses 258. Their independent references and
cancellation checks are retained. The Agent's explicitly enabled fault-stage
parser accepts all three new stages; zero/one-step phase boundaries cannot be
hidden behind its previous three-step minimum. Parser/admission tests are not
live fault-injection qualification.

## Verification

Final local source-state runs completed successfully:

| Check | Result |
| --- | --- |
| Full `kyuubiki-solver` tests | 1,405 passed, 0 failed, 9 pre-existing ignored; 186 result groups including doc tests |
| Full `modal_spectrum_operator` Rust Headless tests | 37 passed, 0 failed |
| Agent fault-stage admission unit tests | 4 passed, 0 failed; no live Agent execution |
| Solver and CLI all-target Clippy with `-D warnings` | Passed |
| Workspace formatting and diff whitespace | Passed |
| Tensor validator and its self-test | Passed; broader maturity/evidence gaps remain |
| Operator profile registry and its self-test | 59 profiles; passed, `executed=false` |
| Documentation book and inventory | Passed; 26 HTML files, development/shipping 3.4.4 |
| Project organization | Passed; source limit 800, document limit 2,000, tracked debt 0 |

The full solver run includes all six new resource/admission unit tests, all six
new public-route tests, the shared-kernel feasibility suites and the enlarged
sparse-path fixtures. Preliminary runs exposed sparse-path fixtures below the
new 256-DOF boundary; the fixtures were enlarged and the full final run was
repeated. No path assertion or residual gate was removed to obtain a pass.
The nine ignored tests are not counted as successful execution.

Numerical commands run from `workers/rust`:

```text
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo test -p kyuubiki-cli --bin kyuubiki-cli agent_fault_injection --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

Profile/tensor validators only check registrations and evidence anchors; they
do not execute the listed numerical commands themselves. These local runs do
not qualify the full validation-profile matrix, installed/remote execution,
general multimode recovery or an industrial performance envelope.

## Remaining Work

- Qualify bounded joint multimode admission against orthogonality, repeated
  subspaces and spectral-index references before enabling that runtime path.
- Resolve or explicitly scope the rejected tiny-coordinate physical sample
  without changing the original residual gate or adding hidden retries.
- Measure paired execution time, work counters and actual peak memory; the
  static payload/work model is not performance evidence.
- Retain live Agent TaskIR, installed and remote execution for the new path.
- Extend independent references beyond the sampled beam geometries and
  coordinate transformations before claiming general industrial reliability.
