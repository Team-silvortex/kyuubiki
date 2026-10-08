# Headless Result Artifact Readback Verification

## Source Scope

October 8, 2026. This verifies a macOS source working-tree overlay on commit
`f5a4247a` (`daji 3.5.3` commit label). Packaged source metadata remains `3.5.0`.
The preceding axial-bar normalization work is retained in the same working
tree. No installed App rebuild, version bump, commit or push belongs to this
round. Historical test-side download reports retain their original scope.

## Implemented Boundary

Native Rust Headless `result_fetch` resolves supported immutable result files
after completed-job identity and requested association gates. Both preferred
embedded results and separate result reads produce actual physical JSON under
the existing `result` key. A small `result_artifact_readback` receipt retains
the normalized original raw-byte descriptor and effective policy. Inline values
remain unchanged; reference-only reads do not invent verification.

The download defaults to 64 MiB and permits an explicit limit up to 512 MiB.
The network deadline defaults to 600 seconds and can be reduced. Malformed
policy values stop before action I/O, including before a combined saved solve's
submission. Combined solves forward all three read policy fields.

Descriptor checks require matching lowercase SHA-256 ID/digest, positive integer
size, exact schema/media and boolean immutability. Download paths are derived
only from the ID on the configured control plane, not from descriptor URLs.
Redirects, wrong status/media, compression, ambiguous framing, oversized headers,
unsupported chunk trailers, truncation and trailing bytes reject. Fragmented
Content-Length and chunked bodies use bounded stream buffers. Owned temporary
files are hashed during writing, verified before JSON decoding, closed and
removed on normal/error return. Nested reference content is not followed.

Failures use the declared public category `result_artifact_readback_failed`,
stage `result_fetch`, non-retryable with strategy `none`. A failed read cannot
authorize a later batch write or another computation. Explicit recovery targets
only the same acknowledged completed job.

## Actual Research Chain

The new live test owns a temporary real Orchestra, isolated database/artifact
store and two Rust Agents configured for that control plane. An approximately
8 MB padded four-element GPa-only bar model selects model/result file transport.
The SDK directly returns displacement and checks it against independent
`FL/(EA)` without a test-side result downloader.

A read-only test proxy changes a numeric byte in the actual stored result body,
preserving its HTTP length. SDK and native CLI both reject the raw digest
mismatch before a guarded project write. The CLI's nonzero failure report
survives file reload. Actual job/project state stays unchanged.

Explicit healthy rereads recover the original completed job through preferred,
separate and reference-only modes. A native research spec extracts
`tip_displacement` in metres from the physical result, not file size. A retained
CLI run report rebuilds the same research evidence through the Rust SDK. Both
Agents return to accepting state. Exactly one calculation/job occurs throughout;
there is no automatic replay or replacement task.

The preceding upload identity and axial-bar parity tests now require SDK-native
physical output instead of falling back to the old test-side downloader. Their
upload substitution, admission counts and downstream refusal assertions remain.

## Verification

Selected regression suites pass **1609 tests**, with one existing Engine test
ignored. Counts exclude focused repeats and repeated qualification executions.
This is selected test breadth, not complete repository code coverage.

| Suite | Passed |
| --- | ---: |
| Native Headless SDK | 527 unit and 25 integration |
| CLI | 16 unit and 96 selected integration, including 55 live/harness tests |
| Engine | 637 unit; one existing ignored |
| Protocol | 120 unit and seven integration |
| KCore | Six unit and 12 integration |
| Standalone Rust SDK | 106 integration |
| Solver accuracy/bar reliability | 32 integration |
| Elixir normalizer and HTTP/model/result-artifact APIs | 25 |

Thirteen new unit tests cover verified physical values, original byte identity,
both result routes, no-I/O policy rejection, malformed descriptors, reference-only
mode, unchanged inline output, unsampled corruption, status/media/framing limits,
fragmented chunked decoding, JSON-object/nested-reference checks, slow body reads
against one total deadline, spool ownership/cleanup, combined-policy propagation
and public receipt schema fields. The existing failure receipt schema test now
also exercises the new category. The old reference preservation test explicitly
uses valid reference-only mode rather than accepting malformed descriptors.

All-target Clippy for native SDK, CLI, Protocol and KCore passes with warnings
denied; Rust format checks pass. The Headless qualification contract now requires
543 tests (527 SDK unit plus 16 CLI boundary tests), with eight new readback
anchors. The existing Protocol qualification report remains valid for 120 unit
tests, 61 RPC methods and five TaskIR examples. Qualification reports are local
ignored small JSON files, not newly published build artifacts.

Both local qualification reports pass report verification. Topology, module/function
matrix, extension standard, runtime API surface, book (26 HTML files), documentation
inventory and version checks pass. Organization has zero tracked debt at the
800-line source and 2000-line document limits. The tensor self-test and actual
check pass with zero structural gaps, four maturity gaps, 19 evidence-grade gaps
and 14 P0 gaps; Daji readiness remains blocked. The new proof is contract-only.

## Limits

Read byte limits are not a total RSS bound: parsed JSON and retained previews can
expand, and decoding is not preemptively interrupted by a CPU timer. Only small
physical results are accepted in this live chain; padded input transport is not
million-node numerical or throughput qualification. Unix private file mode and
normal/error cleanup do not establish Windows acceptance, hostile filesystem
isolation or power-loss cleanup.

Digest equality proves consistency with the service-provided descriptor, not a
signed execution identity or independent scientific qualification. Solver method
labels are retained, not cryptographically tied to execution. Other solver
normalization, imported-report provenance, Python/Elixir consumers, installed
Apps, remote acceptance and large-result capacity remain separate work. Coverage
tensor evidence is limited to native SDK workflow/validation contract scope.
