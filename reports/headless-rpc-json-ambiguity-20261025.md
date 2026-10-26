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
ignore them; unique unknown fields keep their previous schema behavior.

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
