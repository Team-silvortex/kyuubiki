# Offline Modal Research Readback

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [export publication](headless-modal-export-publication-20261025.md).
This change adds an example-local consumer and test-harness repair, not a new
Solver algorithm, Engine storage policy, installed application or release.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Missing Consumer Boundary

Publication already checked a result before exposing `verified/result.json`
and `verified/verification.json`. Subsequent consumption requirements were
documented, but the example had no executable offline verification entry.
The new `verify <submission-dir> <export-dir>` command closes that example
workflow without introducing a generic SDK command router.

The command reads the retained input and typed intent, reconciles the original
execution receipt, checks any present admission, compares the published result
bytes against their SHA-256 and parsed content against the completed receipt,
and validates the verification report's identity and qualification claims.
It then reruns the independent physical reference checker.

It does not read runtime endpoint environment configuration, connect to an
Agent, submit work, create output directories, repair records or promote a
pending export. Only a fresh JSON verdict is printed to standard output.
The original endpoint must still be syntactically valid retained metadata;
offline verification does not establish its present identity or availability.

Historical numerical diagnostics must have the expected array shapes and
finite nonnegative entries. They are not physical authority: roots and
residuals are recomputed. A difference sets `historical_diagnostics_match`
to false in the new verdict instead of silently reusing the old diagnostics
or demanding bit-identical floating-point arithmetic across toolchains.
`passed` describes the new physical check, not a digital signature over the
old report. The saved report is never rewritten.

## Real-Agent Evidence

Two opt-in tests submit real modal work through the actual example executable,
wait for completed original receipts, export, then kill the test-owned Agent.
They invoke `verify` with an intentionally invalid endpoint environment value,
so any attempt to parse online configuration would fail the positive control.
Snapshots compare both paths and bytes before and after every verification.

- A valid export rechecks successfully with the Agent stopped. Missing admission
  is recoverable through the saved intent and completed receipt.
- Deliberately changed historical diagnostics produce freshly recomputed values
  and an explicit mismatch flag, without modifying the historical evidence.
- Five required evidence files are each deleted and truncated independently;
  all ten cases fail rather than recreating files or trusting partial state.
- Changed input with both stale and recomputed byte hashes fails against the
  prepared input intent; a changed typed digest also fails.
- Changed results fail with stale hashes, with recomputed hashes, and even with
  a matching changed receipt when independent total-mass validation rejects them.
- Unknown status, wrong store/request identity, false pass/orthogonality flags,
  wrong scope, invented material certification, malformed recovery metadata and
  malformed diagnostic arrays fail.
- Present malformed or foreign admission is not ignored. Another execution's
  otherwise valid export cannot be consumed as the original submission.
- Pending-only and missing export directories fail without promotion or repair.
  The original untouched export still passes after all negative controls.
- Restarted Agents retain exactly the original one/two executions. No new solve
  is authorized by offline verification or a failed verification.

## Harness Findings

The first live run exposed the fault relay reading a newly accepted socket
before request bytes arrived. A deterministic regression forces the accepted
socket nonblocking, delays request bytes, and reproduced `WouldBlock` before
the repair. The relay now explicitly sets blocking mode while keeping its
existing three-second I/O timeout. The regression also waits within a fixed
deadline for the nonblocking listener to accept the test connection.

The new live fixture initially queried the store identity just after submitting
work. Agent descriptions intentionally report `store_busy` without identity
while the store lock is held. The fixture now records the identity before
submission instead of interpreting the busy descriptor as a lost store.
No production retry or busy-state behavior was changed.

An initial live attempt also returned `InvalidInput` on a separate case; no
independent root cause was established for that error. Passing later runs do
not establish a universal socket-stability guarantee.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The SDK all-target run passed 150 tests, with zero failures and 16 live tests
ignored by default. All 16 opt-in real-Agent tests passed separately through
the focused target and executable `modal-sdk-durable-research` validation
profile (`executed: true`, `ok: true`).
The ordinary test count includes the existing no-op publication-child fixture
and the new socket regression, not extra research journeys. Overlapping test
counts must not be added. Clippy passed with warnings denied.

Documentation book/inventory and project organization checks passed, with no
tracked size debt under the 800-line source/2,000-line documentation limits.

## Limits

This is a caller-owned local-file check of the two explicit synthetic model
orderings. It does not authenticate jointly forged input/intent/receipt/result
sets, consult current remote ledger state, defend against hostile concurrent
filesystem mutation, prove power-loss durability, qualify other languages or
establish general heterogeneous modal production reliability. Independent
physics tolerances are unchanged. Tensor evidence stays `verified`, not
`qualified`; no unrelated release gates are closed by this bounded extension.
