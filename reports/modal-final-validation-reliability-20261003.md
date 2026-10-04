# Modal Final Validation Reliability

## Scope And Findings

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. The preceding vector-control work covered modal refinement,
but final spectrum norms and published-shape input scans and norms still
ran without internal cancellation checks. Six initial regression tests failed
before those checks were added; the sampled legacy residual-bit baseline
already passed. This report records final-validation cancellation and replay,
not an expansion of the supported numerical spectrum.

The 128-element planar bending request for six modes still fails its original
refinement gate. Its public, headless and live Agent failure/replay regressions
remain required. It is not counted as a successful or qualified solve.

## Implementation Boundary

`modal_frame_spectrum.rs` uses the existing cancellable scaled norm for the
final eigenvector norm, normalized residual norm and applied-vector norm.
`modal_published_shape.rs` checks finite shape/product values and positive
mass in 64-row blocks, then uses that same norm for its applied product,
mass target and physical residual. Each norm retains the original accumulator
and term order rather than combining separately rounded block norms.

The appended diagnostic stages are `modal_spectrum_norm`, `modal_shape_scan`
and `modal_shape_norm`. Existing numeric stage identifiers are not renumbered.
Checks occur at entry, every 64 components and the final partial block.
Cancellation propagates from the operation itself before its enclosing
control scope returns. It does not yield a partial numerical norm or spectrum.

All numerical changes remain in Solver. Engine and Agent orchestrate the
execution; they do not implement a second eigensolver or physical validator.
Residual tolerances, the four-step refinement and shape-correction budgets,
coordinate relaxation, mass weighting and existing shape-normalization
arithmetic remain unchanged. No new dependency or retry mechanism is added.

Shape corrections remain private until their recomputed physical residual
passes the original gate. Cancellation during input checks or any of the
three final norm passes leaves the caller's seed unchanged, including after
a correction has already produced an improved candidate. Public recovery
also discards earlier completed modes if validation of a later shape fails.

Agent fault injection admits the new stages only through its existing
explicit hold-path, supported-method and exact-job-marker controls. Tests
use these controls to reach a numerical safe point reliably. Normal Agent
startup does not enable a hold or create another runtime service.

## Regression And Recovery

The spectrum-control target verifies all nine final norm passes for three
129-DOF diagonal modes. Each selected pass cancels after 64 components;
a fresh call returns the exact expected eigenvalues and component bits.
The trace covers each pass at steps `0, 64, 128, 129`, including later modes.

Published-shape tests cancel both before and after correction, independently
selecting its input scan and each of its three norm passes. They also check
that cancellation at step 64 precedes invalid mass at the tail, and that
invalid data still fails on a fresh uncancelled call. Sampled relative
residuals and residual-vector bits match the legacy arithmetic at lengths
`1, 63, 64, 65, 127, 128, 129, 4097` and scales `1, 1e-200, 1e200`. This bit
comparison is an arithmetic-regression baseline, not an independent physics
oracle; the public replay lanes below provide independent physical checks.

The public 100-element bending fixtures cover planar 20-mode and spatial
six-mode results. Both cancel in each new stage and replay with independent
physical residuals, discrete root references, restrained zeros, unit shape
norms and mass orthogonality. Late-cancellation cases reach the third physical
norm pass of the second shape on both owned and borrowed routes. The call
returns an error rather than exposing the earlier successful mode. Replay
retains the spatial paired bending directions.

The 129-segment axial fixtures cover uniform and heterogeneous planar and
spatial chains at common mass/stiffness scales `2^-600, 1, 2^600`. They cancel
in each new stage without dense refinement or factorization. Fresh owned and
borrowed solves agree, while independently assembled physical residuals
remain below `1e-9`. The first version of the route guard incorrectly treated
`ModalSweep` as exclusively dense; that stage also marks tridiagonal
validation. The corrected guard rejects actual dense refinement/factor stages
instead. No production solver change was made to satisfy that test.

Live local TCP tests cancel planar and spatial TaskIRs in all three stages.
Each returns a cancellation error with no result and a non-resumable
checkpoint, releases its execution slot and accepts a fresh TaskIR on the
same connection. Replay passes independent physical checks. The hold marker
remains present, demonstrating exact-job rather than global holds. The
shared test helper now also uses that same connection for existing vector
and sparse-product cancellation replays.

These lanes overlap with the modal profile and must not be added as distinct
coverage. They do not establish installed, remote or industrial qualification.

## Validation Results

| Lane | Result |
| --- | --- |
| Full Solver suite including integration and doc harnesses | 1334 passed, 0 failed, 9 existing ignored; 186 result groups |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Final spectrum-control target | 2 passed, 0 failed, 0 ignored |
| Published-shape target including new control regressions | 14 passed, 0 failed, 0 ignored |
| Public mass-coordinate target | 30 passed, 0 failed, 0 ignored |
| Public axial-chain target | 20 passed, 0 failed, 0 ignored |
| Live local TCP Agent target | 13 passed, 0 failed, 0 ignored |
| Executed modal profile | 31 commands, 307 passed, 0 failed, 0 ignored |
| Strict Solver and CLI Clippy on all targets | Passed with warnings denied |
| Formatting and whitespace | Passed |
| Documentation book and inventory | Passed; 26 HTML files, development and shipping 3.4.4 |
| Source 800 and document 2000 organization audit | Passed; zero tracked debt |

The executed modal profile has `executed=true` and `ok=true`; its saved-report
gate also passes. Fifteen new tests span kernel controls, public recovery and
live Agent cancellation. No ignored test was added. This does not claim
execution of the entire CLI test suite, an installed package or remote Linux.

Tensor structural and command checks pass with 13 modules, 11 paradigms and
no structural gaps. Daji readiness remains blocked: four maturity gaps,
sixteen evidence-grade gaps and eleven P0 gaps remain. These scoped verified
claims do not clear broader operational or industrial qualification gaps.

## Reproduction

Run the native validation entrypoint from the repository root:

```sh
CARGO_NET_OFFLINE=true ./scripts/kyuubiki check-operator-validation --execute --profile modal-frame-sanity --out tmp/modal-final-validation.json
CARGO_NET_OFFLINE=true ./scripts/kyuubiki check-operator-validation --in tmp/modal-final-validation.json --profile modal-frame-sanity
```

Local JSON and raw logs are ignored execution artifacts, not new retained
qualification. Separate tensor claims link bounded Solver numerical/recovery
checks and development-Agent execution/recovery. Earlier reports keep their
original source fingerprints and validation counts.

## Source Fingerprints

SHA-256 values are relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_frame_spectrum.rs` | `fd6f27b8514ae88a293c670f2da1ec6f056282fd94cbae2510813fbb20264b2a` |
| `solver/src/modal_frame_spectrum_control_tests.rs` | `9679f3b13881c7c873afc2b455f95be884a4d7eb5d8d74c27a05940ec2c637bf` |
| `solver/src/modal_published_shape.rs` | `cfa762a36f753d72a30b32219e938c4e942f195851f1223a594b1324652fcd0a` |
| `solver/src/modal_published_shape_control_tests.rs` | `3f5ba4bc574be620a8c2d5d59dbb99de526c794086cebfcdd863a0bfbee1fab0` |
| `solver/src/solver_control.rs` | `cade7b1bc7502f8064cc626571a0c14a506d84fca0902587c6fb22d19a0b91f1` |
| `solver/tests/modal_mass_scaling_reliability/control.rs` | `bf906cf686b21e4a7e13cc4ce44d1c4cca3a287a4b96f3913ee0eff3eb77a664` |
| `solver/tests/modal_chain_reliability/final_validation.rs` | `3fc3df41ef48be415b3e90201b61f8b5ef3966fa8af00fd1396fa429c8a3a036` |
| `cli/src/agent_fault_injection.rs` | `79ff8de378a9b2d5c7c40af3c33dbc54174d7d4b21a617aafa39a40109727ebc` |
| `cli/tests/agent_modal_live.rs` | `e684280fe423d01808f624ce9197adac141c7b7207a45791010b5b6f34d3c2de` |

## Remaining Boundaries

This is bounded final-modal-validation verification, not general modal or Agent qualification.
Cooperative checks bound components between polls, not wall-clock latency
during allocation, callback execution or other unmodified solver phases.
This round adds no million-DOF bending, throughput, installed desktop or
remote Linux claim. Source metadata remains 3.4.4; no desktop package has
been rebuilt or reinstalled, and no commit or push is part of this round.
