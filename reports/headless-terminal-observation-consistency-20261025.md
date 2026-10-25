# Retained Terminal Observation Consistency

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [bounded evidence reads](headless-modal-evidence-boundaries-20261025.md).
This change extends the official Rust headless SDK's caller-retained recovery
contract and applies it to the modal example. No Solver numerical algorithm,
Engine/Agent execution policy, installation or package version is changed.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Reproduced Gap

The previous example checked that saved admission and observed receipt had the
same execution handle. It did not reconcile their terminal outcome or payload.
A real native Agent result was exported, then a test-owned copy of admission
was replaced with a syntactically valid failed receipt using the same identity.
Offline verification incorrectly accepted the successful exported result. The
new regression failed before the repair. No actual Agent ledger was modified.

Identity equality cannot establish outcome consistency: an immutable terminal
receipt must not change from failure to success, replace its payload, or lose
its known result merely because a later record has the same IDs.

## SDK Contract

`DurableExecutionHandle::validate_followup(retained, observed)` independently
validates both observations against the handle. If the retained state is
completed or failed, the later state and known v1 terminal contents must agree.
This includes result values or failure code, message and opaque details.

`SolverRpcClient::fetch_durable_agent_execution_after(retained)` derives the
original handle, performs one read-only fetch, then applies that reconciliation.
It introduces no retry, submission, persistent cache or filesystem policy.
The existing handle-only fetch remains available and does not invent history
that was never stored in its identity-only handle.

The internal comparison reuses the typed encoder under a separate local domain.
It preserves integer values, low binary64 bits and signed zero, sorts object
keys, and has a bounded nesting depth. It walks borrowed payloads rather than
cloning or serializing entire result buffers. Request digest bytes and shared
SDK/Agent golden vectors are unchanged. This is not a new wire hash or file
format. Comparison remains proportional to the retained/observed payload size;
no performance benchmark claim is made here.

Absent and null optional wire fields normalize together. Unrelated observation
metadata is not part of the v1 terminal commitment. Both receipts must still
pass the existing supported protocol and terminal-shape checks.

Running and outcome-unknown records have no committed terminal payload. A later
valid original terminal receipt can be reconciled without inferring replay or
changing the original input. This is terminal-evidence consistency, not a total
ordering for all possible nonterminal observations. A consistent failed receipt
remains readable as failure evidence; `completed_result` still returns its
original solver error rather than qualifying it as a successful result.

## Example Integration And Tests

The example applies the SDK comparison between present `admission.json` and
the original observed `receipt.json` in both online `inspect` and offline
`verify`. Error context names both files. Online failure can retain the raw
receipt for diagnostics but never creates a verified export. Offline failure
does not modify any evidence. Missing acknowledgement reconciliation is unchanged.

The live regression exercises two conflicts against a real completed result:

- A failed retained admission contradicts a successful terminal receipt.
- A retained completed admission contains the opposite sign of one mode shape.
  That alternate shape independently passes the physical checker, but it is
  still not the same immutable terminal payload. Both online and offline paths
  reject the conflict; the Agent's original receipt remains unchanged.

Matching completed observations and a nonterminal unknown observation followed
by the valid original terminal result remain accepted. These controls mutate
only test-owned copies, not the actual Agent's outcome. The retained store has
one execution; no conflict resolution submits another solve.

Six SDK regressions cover terminal changes/downgrades, original failure details,
signed zero/low-bit/integer precision, optional-null normalization, nonterminal
identity binding, depth limits, invalid identity before network access and
history-aware read-only fetch. Existing real modal recovery uses the new fetch
after Agent restart; the real timeout/failure case checks the new fetch too.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target passed all 19 opt-in live Agent tests on macOS, along with
the shared request-digest, receipt, intent, file-reader and transport checks.
The full SDK run passed 162 tests, zero failures, with 19 live tests ignored by
default and separately executed. Clippy passed with warnings denied. Counts
include the existing no-op child fixture; overlapping runs are not additive.

The native `modal-sdk-durable-research` validation profile also passed with
`executed=true`. Documentation book/inventory checks, targeted Rust formatting,
project organization and `git diff --check` passed. Source/document limits remain
800/2000 lines, with zero tracked organization debt. The tensor structure check
passed without structural gaps; its two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps remain. This repair does not remove those broader readiness gates.

## Limits

Applications must retain and supply the full observation; an identity handle
alone cannot reveal previous outcome changes. Consistency is not authenticity,
an independent physical qualification, remote chronology proof or protection
against coherently forged history. The existing local-file, power-loss and
general heterogeneous modal qualification limits remain. Tensor evidence stays
`verified`, not `qualified`. No Git commit or push is part of this turn.
