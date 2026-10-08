# Headless Model Artifact Identity Follow-Up

## Reproduced Defect

Native large-model upload handling previously accepted any string `artifact_id`.
A full valid-looking descriptor for different content was forwarded into the
solver request. The new mismatched-content regression failed against the old
implementation and passes with the prepared/sent/acknowledged content gate.

The repair hashes prepared file bytes and actual transmitted chunks, checks exact
typed schema/digest/identity/length/media/immutability, strips unverified reply
extensions, and retains `model_artifact_upload` in successful outcomes. Inline
submissions and previews explicitly retain null. Combined saved solves preserve
the small descriptor without cloning full result arrays.

## Real Chain

The source-built macOS experiment owns a real Elixir Orchestra, isolated SQLite
and artifact directories, and two native Rust Agents. The test first prepares a
healthy uploaded bar model. A test-only relay then forwards another upload
unchanged and replaces its real reply with the first model's existing same-size
reference. Native SDK and actual CLI reject it at `artifact_upload`; both leave
the job/project baseline unchanged, perform no extra calculation and never reach
the guarded downstream write. The CLI persists the same failed receipt it emits.

A separate explicit healthy native-SI run completes and retains the actual
upload descriptor through binding and JSON reload. The two healthy bar results
are downloaded separately as raw result-artifact bytes, verified against their
declared digest/size, and checked using independent `FL/(EA)` and load-ratio
relations. Retained research evidence observes result-file byte count only. The
approximately 8 MB fixture uses padding and four physical elements; it is not a
million-node or remote qualification experiment.

## Newly Exposed Work

The first healthy experiment failed because its Agents had no control-plane
download address. The owned harness now explicitly configures that address while
leaving installed runtime configuration untouched.

The next failure exposed an actual normalization mismatch: GPa-only axial-bar
inputs work inline but artifact references bypass `FemModelNormalizer` and the
native Agent requires `youngs_modulus` in SI. This is still open, not fixed by
receipt hashing. Healthy tests use consistent SI/GPa forms. Artifact-backed
result readback is also not automatic in the SDK; the physical check above uses
an explicit test-side bounded HTTP download, not a new SDK capability. Both
acceptance boundaries are recorded in the weakness roadmap.

## Verification

The selected regression run passed **887 tests**: native Headless SDK 513 unit
and 25 integration tests; selected CLI suites 104 (including 53 real-chain/harness
tests); Protocol 114 unit and seven integration tests; KCore six unit and 12
integration tests; standalone Rust SDK 106 integration tests. Counts exclude
repeated focused experiments, zero-test binaries, and repeated qualification
execution. This is selected-suite breadth, not whole-repository code coverage.

All-target Clippy for native SDK/CLI/KCore/Protocol passed with warnings denied.
The Headless qualification contract now requires 529 tests (513 SDK unit plus
16 CLI boundary tests); its source-bound report is retained locally and checked
separately. Artifact submission and receipt tests are in dedicated test files,
with original test names registered under their new module paths.

Tensor checks retain zero structural gaps, four maturity gaps, 19 evidence-grade
gaps and 14 P0 gaps; Daji readiness remains blocked. Topology, matrix, extension,
API surface and documentation checks pass without promoting operational proof.
No installed-App rebuild, remote test, release-version change, Git commit or push
is part of this round.

## Limits

This adds verified contract evidence to `sdk-headless` workflow, validation and
Headless coordinates only. It does not promote their operational/numerical
qualification, authenticate an Agent's execution, bind normalized solver request
bytes or certify any result solely from a digest. An acknowledged upload may
remain in managed server storage after a later rejection. Unknown-outcome upload
failures are not converted into safe automatic retries. Unit tests also cover
temporary-content replacement, exact numeric/string content, all required reply
fields, extension stripping and halting before solver/downstream POST.
