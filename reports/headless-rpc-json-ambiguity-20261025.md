# Reject Ambiguous RPC Payloads Before Evidence Is Lost

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Scope: official Rust headless SDK direct-Agent response decoding. No changes to
Agent admission policy, execution storage, Engine contracts or Solver numerics.
This follows [explicit capacity refusal handling](headless-durable-admission-rejection-20261025.md).

## Reproduced Defect

Known outer envelope keys were typed and rejected duplicates, but `result`,
error `details` and retained progress documents decoded into ordinary JSON
values. Repeated keys were silently overwritten before identity, policy and
historical result checks could see the original contradiction.

Four raw loopback regressions failed before the fix:

- A result containing both failed and completed status was accepted as completed.
- A capacity report with both true and false `execution_started` values became
  a definite `DurableAdmissionRejected` decision using the last value.
- Nested repeated failure-detail fields were retained as a normal RPC error.
- Repeated optional progress fields were accepted rather than diagnosed as
  ambiguous JSON. The initial red fixture then hit EOF waiting for a terminal
  reply; the final test also supplies a valid terminal frame after the ambiguity.

These are controlled protocol faults, not a claim that the native serializer
normally emits duplicate fields or that a deployed transport was compromised.

## Implementation

A private recursive JSON visitor builds retained payload values using unique
decoded object keys. Repeated identical values and escaped versions of a key
are also rejected; identical keys in different objects remain valid. The map
entry API checks the destination map directly, without a separate key set or a
second copy of the result. Error messages do not echo the duplicate key/value.

The decoder applies it to complete results, structured error details and the
original retained progress frame. Typed envelope and progress validation stay
in place. Existing byte budgets, finite number parsing and parser depth limits
remain; this adds no exact heap or CPU budget. Numeric kinds, binary64 bits,
negative zero and subnormals are preserved, with no tolerance or normalization.

An ambiguous frame returns a JSON error, not a validated execution decision.
After dispatch, it cannot prove success, failure, absence or non-admission. The
caller keeps the original intent/receipt and may explicitly inspect the original
execution, without automatic resubmission or a new ID. Public interfaces and
wire schemas do not change.

This is not authentication or a global JSON policy. Ignored unknown terminal
envelope fields are not retained payload; external readers and other language
SDKs keep their own parsing responsibility. A `Value` supplied after another
parser discarded duplicate keys cannot reveal the original ambiguous bytes.
The research example's separate bounded file reader is unchanged.

## Test Evidence

Seven ordinary protocol tests cover nested/array/escaped/identical duplicates,
admission-flag contradictions, error details, optional progress data, valid
result/error/progress controls, signed zero, integer/float identity, integer
extrema, one-ULP values, minimum subnormal, maximum finite binary64, sibling key
scopes, parser depth and nonfinite number rejection.

One opt-in integration test computes a real four-mode heterogeneous cantilever
and checks its independent physical reference. A controlled response fixture
adds conflicting status or input-digest keys ahead of the original values in
that actual terminal receipt. A last-key-wins parse would preserve the original
typed history digest while hiding the bad bytes; the SDK now rejects both.

The original Agent is then read directly, forcibly terminated and restarted.
Its receipt remains identical and passes physical revalidation; one stored
execution and zero active executions remain. The fixture is not an on-network
capture or remote fault injection. No installed services are touched.

## Validation

- Focused raw-message and real-Agent run: all eight tests passed.
- Full SDK all-target tests: 197 passed, zero failures; 34 opt-in tests were
  ignored in that invocation and executed separately through the profile below.
- The `modal-sdk-durable-research` profile executed `make test-sdk-modal-recovery`
  successfully (`executed=true`), including all 34 opt-in Agent/protocol tests.
- All-target Clippy with warnings denied, targeted Rust formatting and
  `git diff --check`: passed.
- Documentation book/inventory and project organization: passed, with the
  800-line source / 2000-line documentation limits and zero tracked debt.
- Tensor structure: passed with zero structural gaps. Overall daji status remains
  `blocked`, with 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

These overlapping runs are not additive and are not a coverage percentage.

Scope remains bounded local development verification, not installed,
cross-language, remote-authenticated or host-power-loss qualification.
No version bump, commit/push, packaging or deployment is included.

## Native Record Follow-Up

Date: 2026-10-26. Same local macOS source-built scope and package baseline.
This follow-up changes the native Agent's record I/O boundary; the earlier
SDK-only scope above describes the original work, not this extension.

### Reproduction And Correction

The native record reader checked encoded length and SHA-256, then decoded
directly into typed records containing JSON values. Nested duplicate result or
failure-detail fields were silently collapsed before the outgoing RPC was
serialized, so even the SDK's unique-key parser and retained terminal history
could not detect the original ambiguity.

A native unit regression returned a completed result with `value: 2` from the
checksummed payload `{"value":1,"value":2}`. Two real-Agent regressions also
failed before the fix: a negative duplicate frequency preceding the original
modal frequency, and a conflicting failure reason preceding the original reason,
were normalized away. The fixture recomputes the unkeyed checksum only inside
its disposable store. This demonstrates malformed-record acceptance, not a
remote attacker or a native serializer normally producing these bytes.

A private native JSON preflight now recursively requires unique decoded keys
before the original typed decoder runs. The preflight traverses the same bounded
byte buffer, retaining object keys only rather than a second full result tree.
Typed decoding consumes those original bytes, preserving its numeric semantics.
This is an additional parse, not a claimed performance improvement. Existing
file-size, parser-depth, checksum and file-ownership gates remain in force.
Unknown fields also undergo uniqueness checking before a typed decoder can
ignore them; unique unknown fields kept their previous schema behavior in that
change. The terminal-envelope follow-up below tightens that separate boundary.

Record preparation uses the same gate, so future custom serializers cannot
publish bytes that the restart reader would reject. There is no new dependency,
public SDK API, protocol field, Engine coupling or change to Solver arithmetic.
Rejected reads preserve `durable_execution_unavailable`, original request
diagnostics and no-replay policy; inspection does not rewrite evidence.

### Evidence And Limits

Four native tests cover nested, escaped, identical and ignored-field duplicates;
result and failure-detail payloads; sibling scopes and JSON-looking strings;
negative zero, integer extrema, minimum subnormal and one-ULP binary64 values;
trailing data, invalid UTF-8, nonfinite/out-of-range numbers, depth and byte
limits; and ambiguous custom serialization rejected before file creation.

Two real-Agent tests run actual completed and failed modal work. They replace
only the disposable native receipt with the checksummed ambiguous version.
Live and restarted SDK reads now receive the native error, and the research
example creates no export directory. An unrelated absent ID remains readable;
bad bytes and caller records remain unchanged. Explicit restoration of the exact
original native receipt recovers the same result or failure diagnostics. The
successful case exports independently verified modes and rechecks them offline
with the Agent stopped; one stored execution and no active work remain.

These tests do not provide record authentication, signatures, remote access
control, host-power-loss durability or a global JSON policy. An attacker with
write access can still replace a record with different unambiguous data and a
matching unkeyed checksum; ownership and caller-retained commitments remain
separate protections. No production data or installed services are modified.

Follow-up validation:

- Native store suite: 48 tests passed, including the four new regressions.
- Both previously failing real-Agent regressions passed using the rebuilt
  development Agent, including stopped-Agent offline verification of the
  restored successful study.
- Formal `modal-sdk-durable-research`: passed with `executed=true`, now running
  the native store suite and all 42 opt-in Agent/protocol integration tests.
- Full SDK all-target tests: 200 passed, zero failures; 42 opt-in tests were
  ignored in that invocation and executed separately through the formal profile.
  These overlapping runs are not additive or a coverage percentage.
- Native CLI and standalone SDK all-target Clippy with warnings denied, targeted
  Rust formatting and `git diff --check`: passed.
- Documentation book/inventory and project organization: passed; source limit
  800, documentation limit 2000, zero tracked debt.
- Tensor structure/command checks: passed, zero structural gaps; 2 maturity,
  23 evidence-grade and 18 P0 gaps remain, with overall daji status `blocked`.
  No evidence grade was promoted by this bounded local regression.

No version bump, commit/push, application installation or deployment was made.

## Native Terminal Shape Follow-Up

Date: 2026-10-26. Local macOS source-built Agent and official Rust SDK; package
baseline remains 3.7.1. The latest version-named Git checkpoint when work began
was `5fa2f509` (`daji 3.7.5`), not a package/artifact version update.

Two native negative controls reproduced silent normalization of correctly
checksummed, uniquely keyed but invalid terminal records. The shared
`RpcResponse` and `RpcError` decoders ignored unknown fields: adding a progress
event to a completed/failed response, or a contradictory result to its error
envelope, still returned an apparently normal original receipt. The SDK could
not detect fields already discarded by the Agent. Missing RPC version was
already rejected and is not a newly fixed defect.

A private persisted-response decoder now requires closed v1 envelopes:

- Response fields: `rpc_version`, `id`, `ok`, `result`, `error`.
- Error fields: `code`, `message`, `details`.
- Unknown fields are rejected even when null. Existing required-field,
  identity, success/failure and byte/checksum/unique-key gates remain in force.
- Actual result and error-detail values remain open JSON; reserved-looking
  keys inside those payloads are legal and do not become envelope claims.

The decoder constructs typed payloads directly, without cloning a full result
tree. It does not alter shared RPC types, serialized record bytes, operator or
engine contracts, solver arithmetic, SDK public APIs, or add a dependency.
Record preparation reads back through the same decoder. Future disk-envelope
extensions require an explicit contract update rather than silent field loss.

Three native tests cover response/error envelope additions, unknown nulls,
live/restarted rejection, unchanged stored bytes, duplicate lookup without an
execution reservation, and open payload controls. Valid result/detail fields
retain signed zero, subnormals, one-ULP binary64 values and integer extrema.

Two added opt-in SDK tests exercise three real-Agent fault scenarios: successful
and failed modal response envelopes containing progress fields, and a failure
error envelope containing result/replay claims. Existing duplicate-key cases
reuse the same bounded harness and remain separately tested.

For each case, the test computes an actual original task, retains caller intent
and admission, modifies only its disposable native receipt and recomputes the
unkeyed checksum. Live and restarted readback, inspection and duplicate
submission reject the malformed receipt. The example creates no export;
admission-only discovery still finds one original task. No bad bytes or caller
records are rewritten and no extra execution is admitted. Exact original-byte
restoration recovers the original outcome without recomputation. Successful
recovery also exports independently checked modes and verifies the export
offline with the Agent stopped; failure recovery preserves original diagnostics.

This is malformed-record fault coverage, not a claim that the production
serializer emits such fields or a deployed adversary caused them. Checksums are
not signatures, same-shaped hostile replacements remain outside this gate,
and local process restart is not host-power-loss qualification. No automatic
repair, retry, version bump, commit/push, installation or deployment is included.

Terminal-shape validation:

- Native store suite: 58 passed, including all three new regressions.
- Selected CLI/Agent tests: 378 passed, covering the CLI unit tests and durable
  synchronous/asynchronous execution, panic, recovery, response contracts/limits,
  lifecycle, task IR and Mesh publication/cancellation/missing-admission chains.
  This was not a rerun of every CLI integration target.
- SDK all-target tests: 206 passed; 48 opt-in tests were ignored by that ordinary
  invocation and executed through the formal profile below.
- Formal `modal-sdk-durable-research`: `ok=true`, `executed=true`, including all
  48 opt-in integration tests. The four focused native-record SDK tests also
  passed independently. These overlapping runs are not additive coverage.
- CLI and SDK all-target Clippy with warnings denied, targeted Rust formatting
  and `git diff --check`: passed.
- Documentation book/inventory and project organization: passed; source limit
  800, documentation limit 2000, zero tracked debt.
- Tensor structure/command checks: passed across 13 modules and 11 paradigms,
  with zero structural gaps. The 2 maturity, 23 evidence-grade and 18 P0 gaps
  remain; overall daji status is still `blocked`. No evidence grade was raised.

Validation is local macOS development-binary evidence, not Linux/Windows,
installed-application or host-power-loss qualification.

## Native Admission Identity Follow-Up

Date: 2026-10-26. Same local source-built scope and unchanged package version.

Three failing native controls exposed a weaker admission boundary than the SDK:
readback checked the execution filename/schema but not the remaining identity
fields; an internal new admission could reach its reservation callback with an
invalid request/process identity; and duplicate lookup compared the digest but
not the independently retained method. Matching checksummed admission/receipt
pairs therefore did not guarantee a well-formed recovery observation. Ordinary
RPC request-envelope validation already rejects invalid transport request IDs;
the new reservation guard is defense for the store's internal API, not a newly
demonstrated bypass of the public RPC request gate.

`Record::validate` now checks the native schema/execution ID, 64-character
lowercase hexadecimal digest, nonblank control-free identity strings bounded to
256 UTF-8 bytes, and the durable method family (`solve_` with a nonempty suffix,
or `run_operator_task_ir`). New work validates before reserving execution or
writing its admission. Existing lookup, duplicate submission, completion checks
and returned index records share the record-read gate. Duplicates separately
compare the retained method and request digest. No string is trimmed or repaired.

Four native tests cover malformed identities in admission-only, completed and
failed stores; coordinated invalid admission/receipt pairs; live and restarted
lookup/index/duplicate rejection with unchanged bytes; a reservation callback
that must never run for invalid new identities; method conflict despite matching
digest; and exact restoration. Positive controls preserve 256-byte and Unicode
identities and TaskIR. Unrelated missing-ID lookup remains available, and valid
new work is admitted after explicit restoration and restart.

A fifth real-Agent record test exercises empty process identity, malformed digest
and a non-durable method across successful and failed modal executions. Native
RPC errors, rather than only SDK-side validation failures, reach inspection,
duplicate submission and listing before and after forced process termination.
The example creates no export while the native identity is invalid. Restoring
the exact original pair recovers the original outcome and index; successful
results export and verify offline with the Agent stopped, while failures retain
their original diagnostic details. Caller evidence is unchanged and only the
original execution remains stored.

This checks bounded metadata, not method availability in the operator registry,
or the digest's relationship to original inputs absent from the native store.
Well-shaped substituted records still require caller-retained commitments and
independent physical validation. Index content validation covers only returned
page entries; discovery is neither a full-store audit nor completion proof.
There is no auto-repair, retry, new dependency, solver/engine coupling, version
bump, commit/push, installed-service change or deployment.

Admission-identity validation:

- Before the fix: three native regressions failed and the positive identity
  control passed. After the fix: all 62 native store tests passed.
- Selected CLI/Agent regression: 382 passed, zero failures, covering CLI unit
  tests and the durable/Mesh integration targets from the preceding follow-up.
  This is not an all-target CLI or complete project test claim.
- SDK all-target regression: 206 passed; the 49 opt-in tests ignored by this
  normal invocation were run explicitly by the formal profile.
- Formal `modal-sdk-durable-research`: `ok=true`, `executed=true`, including all
  49 opt-in tests. All five focused native-record live tests also passed in a
  separate run. Counts overlap and must not be added as unique coverage.
- CLI/SDK all-target Clippy with warnings denied, targeted Rust formatting,
  documentation book/inventory, project organization and `git diff --check`
  passed. All modified sources remain below 800 lines; docs below 2000.
- Tensor structure/command checks passed with the unchanged 2 maturity,
  23 evidence-grade and 18 P0 gaps. Overall daji status remains `blocked`;
  this bounded local evidence does not promote a production qualification.

No Linux/Windows, installed-application or host-power-loss verification was run
for this follow-up.

## Native Store Identity Bootstrap Follow-Up

Date: 2026-10-26. Local source-built Agent and official Rust SDK, unchanged package
version. This intentionally retires the populated-store empty-lock compatibility
behavior documented in the earlier Mesh owner-fence report.

Two failing native controls reproduced automatic ownership replacement after an
empty or missing `owner.lock`. Existing admissions and receipts did not prevent
generation of a new random store ID, and startup could also remove unowned pending
files. Pinned SDK callers already rejected the replacement ID, but the native
store could expose original tasks under the new identity to newly discovering or
unfenced callers. This was not recovery of original ownership.

Identity initialization now requires a directory containing no entries except
an optional empty `owner.lock`. The missing-file path checks before creating a
lock, then initialization checks again under the exclusive owner lock before
generating identity or running pending cleanup. Any retained entry blocks it;
no record contents need to be loaded. Existing nonempty identity validation,
file permissions and exclusive locking remain unchanged. Valid-owned stores keep
their prior pending cleanup and recovery behavior. An empty legacy lock can still
initialize in an otherwise empty directory, not over old work.

Native controls cover absent/empty owner files with admission-only, completed and
failed records; repeated rejected startup without record writes or owner creation;
orphan/pending/foreign entries preserved; empty-store initialization and lock
exclusion; and exact owner restoration followed by original-task lookup, duplicate
suppression and fresh explicit work. Existing interrupted-publication alias tests
continue to exercise cleanup under a valid retained identity.

Two real-Agent tests cover six cases: absent/empty owners with completed, failed
or solver-interrupted modal executions. The child process exits on each damaged
startup attempt, lookup cannot return a replacement observation, and owner/task
bytes remain as found. Restoring the original owner recovers the exact original
store ID. Completed work exports and passes offline verification with the Agent
stopped; failed diagnostics are unchanged; killed in-flight work stays unknown
without a new receipt or replay. The test fixture now reaps an exited startup
child before a later explicit restart, without changing production lifecycle.

This is not authenticated storage, protection from concurrent hostile filesystem
mutation, or host-power-loss qualification. A wholly deleted store remains
indistinguishable from a fresh installation; caller pins and retained evidence
remain necessary. No automatic migration, repair, retry, backups, version bump,
commit/push, installed-service changes or deployment were performed.

Owner-bootstrap validation:

- Before the fix: two native negative controls failed; four existing/positive
  identity tests passed. After the fix: all 64 native store tests passed.
- The six focused SDK native-record-loss tests passed, including both new
  owner-loss tests. Their first live run correctly observed startup rejection
  but used the wrong SDK error variant for a refused connection; the assertion
  now checks `SdkError::Io(ConnectionRefused)`. SDK transport behavior was not
  changed to accommodate the test.
- Selected CLI/Agent regression: 387 passed, zero failures (315 CLI unit tests
  and 72 durable/Mesh integration tests, including live Mesh store binding).
  This is not a complete project or all-target CLI runtime qualification.
- SDK all-target regression: 206 passed; 51 opt-in tests ignored by that ordinary
  invocation were executed by `modal-sdk-durable-research`, which passed with
  `ok=true` and `executed=true`. Overlapping counts are not unique coverage.
- CLI/SDK all-target Clippy with warnings denied, targeted Rust formatting,
  documentation book/inventory, project organization and `git diff --check`
  passed; source/document limits remain 800/2000 with zero tracked debt.
- Tensor structure/command checks passed. The 2 maturity, 23 evidence-grade and
  18 P0 gaps remain; overall daji status stays `blocked`. No grade promotion.

This round did not run Linux/Windows, installed-application or physical-power-loss
verification. Disposable failed-test directories were removed after diagnosis;
no user data or application state was cleaned.
