# Missing Execution Observations And Original Record Recovery

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Scope: official Rust headless SDK original-ID inspection and real native Agent
recovery through the research example. Agent storage policy, execution engine
and numerical solvers are unchanged. This follows the
[RPC ambiguity work](headless-rpc-json-ambiguity-20261025.md).

## Reproduced Defect

SDK inspection checked a `not_found` report's schema, store/execution identity,
replay denial and null/absent `response`. It did not reject other execution
claims: the red protocol test accepted `not_found` with `resumable: true`.
Admission identity, top-level result/error or progress fields could likewise
accompany a successful missing-task observation.

This is a malformed-response acceptance defect, not evidence that the native
Agent emits contradictory reports. Existing completed-result gates still
rejected missing observations; no successful export bypass was observed.

## Correction

Both direct original-ID inspection and intent-bound inspection now use the
same missing-state validator. A `not_found` observation cannot carry non-null
`request_digest`, `request_id`, `process_instance_id`, `method`, `response`,
`result`, `error`, `event` or `progress`. Optional `resumable` and
`execution_started` must be absent, null or boolean false, never truthy strings,
numbers or containers. Existing required schema/identity/replay checks remain.

Neutral optional nulls and unrelated diagnostic metadata remain readable.
Neither the minimal report nor one with neutral fields can create an execution
handle or successful result. These observations do not establish historical
non-execution or authorize resubmission. No new API, retry mechanism, automatic
repair or persistence format is introduced.

## Regression Evidence

Three ordinary protocol tests cover both inspection methods, conflicting
admission/payload/authority fields, type errors, pinned lookup identity, explicit
replay denial, native minimal reports, neutral optional fields and metadata.

Two opt-in real-Agent tests cover the caller-visible recovery chain:

1. Submit a real modal study through the research example, retain its completed
   receipt, then displace only that test Agent's admission record. SDK and
   example inspection report `durable_execution_unavailable`; the example
   creates no export directory. Restart with the orphan receipt still present
   preserves refusal. Restore the exact admission, separately corrupt each
   record's checksum, and verify the same refusal without silent repair. Restore
   exact bytes and restart: the original receipt is unchanged, a fresh export
   passes independent physical checks and offline verification, local input and
   admission records are unchanged, and one stored execution remains.
2. Hold an identified real modal task at its solver checkpoint, confirm the
   active checkpoint, then force-terminate and restart the Agent. Its original
   admission reports `outcome_unknown`. Displacing that known admission yields
   unavailable, not absence. Restoring the exact admission recovers the same
   unknown state with null response and no resumability, result or active work.

An unrelated never-admitted ID remains observable as `not_found` during the
first test, without becoming a handle. The native failure code, RPC request ID,
replay denial and required-action details remain available to the SDK caller.

The first interrupted-fixture attempt did not set the job ID selected by the
existing fault hook, so the task finished and the checkpoint assertion failed.
After binding the input job ID to the hook, both live tests passed. The failed
fixture is not counted as interruption evidence. Fault files and record edits
exist only in disposable test directories, never installed services.

## Limits

Native missing-admission detection retains known IDs for the current process
and detects a remaining orphan receipt after restart. It cannot reconstruct
complete evidence deletion before restart. That existing policy is unchanged;
caller-retained history and no-replay rules remain essential. The interrupted
test displaces its record after the restarted process has scanned the original
admission; it does not claim detection after deleting every trace and restarting.

These tests use explicit exact-record restoration, not an SDK repair operation,
concurrent filesystem mutation safety, remote authentication, disk-full fault
qualification or physical power-loss durability.

## Validation

The focused protocol run passed all three new tests and 24 existing durable
receipt tests. Both new opt-in Agent/example tests passed after correcting the
interruption fixture.

- Full SDK all-target tests: 200 passed, zero failures; 36 opt-in tests were
  ignored in that invocation and executed separately through the profile below.
- The `modal-sdk-durable-research` profile executed `make test-sdk-modal-recovery`
  successfully (`executed=true`), including all 36 opt-in Agent/protocol tests.
- All-target Clippy with warnings denied, targeted Rust formatting and
  `git diff --check`: passed.
- Documentation book/inventory and project organization: passed, with the
  800-line source / 2000-line documentation limits and zero tracked debt.
- Tensor structure: passed with zero structural gaps. Overall daji status remains
  `blocked`, with 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

These overlapping runs are not additive and are not a coverage percentage.

No version bump, commit/push, packaging, installation or deployment is included.

## Terminal Receipt Loss Follow-Up

Date: 2026-10-26. Same local macOS source-built scope and package baseline.
This follow-up adds two real-Agent regressions, not a new implementation fix.
The existing SDK history gate and example publication gate held under these faults.

1. Submit and independently export a completed modal study. Keep the original
   submission and export unchanged, and create a separate caller-history fixture
   with that real completed report as its retained acknowledgement. Displace the
   native receipt while its admission remains. Both before and after forced Agent
   termination/restart, identity-only reads return `outcome_unknown` with the
   original identity, null response and no replay/resume authority. History-aware
   fetch rejects the lost terminal commitment. Online example inspection using
   either initial or completed admission fails, retaining only the raw diagnostic
   observation; neither `verified/` nor pending publication is created. With the
   Agent stopped, the intact original export still passes independent offline
   verification. Original submission/export and native admission bytes remain
   unchanged. Explicitly restoring the exact receipt and restarting permits the
   original completed read and a newly verified export, with one stored execution
   and zero active work or reservations.
2. Submit invalid material density to obtain a real failed solver receipt with
   structured diagnostics. Persist/read back the complete failure locally, then
   displace its native receipt. Live and restarted identity-only reads are unknown;
   history-aware reads reject the downgrade instead of silently losing the known
   cause. Explicit exact-byte restoration and restart recovers the same failure,
   including original request ID, code, message and details. Caller-retained bytes
   remain unchanged and no new execution is created.

These tests deliberately distinguish a diagnostic `receipt.json` from a verified
research result. Saving a conflicting observation for inspection is not a result
publication or an overwrite of earlier evidence. Offline success certifies only
the retained study's bounded consistency and physical reference checks, not the
current Agent store or scientific provenance. Terminal history remains explicitly
caller-owned; the SDK does not discover prior exports or persist a global cache.

Follow-up validation:

- Focused native missing-record suite: all four tests passed, including the two
  new terminal-loss regressions.
- Full SDK all-target tests: 200 passed, zero failures; 38 opt-in tests were
  ignored in that invocation, not counted as passes.
- The formal `modal-sdk-durable-research` profile passed with `executed=true`,
  including all 38 opt-in Agent/protocol integration tests. Its ordinary tests
  overlap the full SDK run and are not an additional coverage count.
- All-target Clippy with warnings denied, targeted Rust formatting and
  `git diff --check`: passed.
- Documentation book/inventory and project organization: passed; source limit
  800, documentation limit 2000, zero tracked debt.
- Tensor structure/command checks: passed; 13 modules, 11 paradigms, zero
  structural gaps. Overall daji status remains `blocked`, with 2 maturity gaps,
  23 evidence-grade gaps and 18 P0 gaps. Evidence grade was not promoted.

No production code change, version bump, commit/push, packaging, installation or
deployment was made in this follow-up. Tests use isolated temporary Agent stores
and remove their successful-run fixtures; no permanent backup policy was added.
