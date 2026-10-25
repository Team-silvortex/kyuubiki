# Modal Evidence Read Boundaries

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [offline readback](headless-modal-offline-readback-20261025.md).
Changes are confined to the official Rust SDK modal research example, tests
and evidence documentation. No Solver algorithm or Engine/Agent policy changed.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Reproduced Defect

The example used `fs::read` for optional `admission.json` and interpreted every
`NotFound` as absence. A real native Agent execution was exported, then the
test replaced its admission with a dangling symbolic link. Offline verification
incorrectly succeeded by falling back to the input intent and terminal receipt.
The regression failed before the repair, demonstrating that a present invalid
record could violate the documented no-downgrade rule.

The original input, actual Agent receipt and published result were unchanged.
This is an evidence-consumption defect, not an incorrect numerical solve.
Inspection also found unbounded file reads and ordinary JSON parsing that
silently kept the last value of duplicate object keys.

## Bounded Reader

Input, submission intent, optional admission, retained receipt, published
result/report and staged publication readback now use one example-local reader.
It validates fixed local filenames and non-symlink immediate containers, checks
leaf metadata without following links, and requires a regular file. Only a
missing leaf in a valid container yields optional absence. Errors opening an
already observed file remain errors, even if it disappeared in the meantime.

Encoded files are limited to 8 MiB. The reader checks size before opening,
rechecks the opened file and caps the read at the limit plus one detection byte;
it does not preallocate from an untrusted reported size. One byte buffer feeds
both exact-byte hashing and JSON parsing, avoiding separate reads for identity
and contents. This is an encoded-data budget, not an exact parsed-JSON heap cap.

The JSON visitor rejects repeated object keys at every depth, including keys
with equivalent escapes. It preserves ordinary JSON numeric/string/array/null
semantics, requires end of input and retains the parser's nesting protection.
Read/parse errors identify the evidence filename without dumping file contents.

The 8 MiB policy is deliberately scoped to this small four-mode example. It is
not a new maximum model size, general SDK transport limit or Engine contract.
Existing exact-byte publication, physical revalidation and no-replay rules stay.

## Verification Cases

Six focused tests exercise exact-byte/type preservation, top-level/nested/
array/escape-equivalent duplicates, the exact size limit and one byte over,
truncated/trailing/invalid-UTF-8/deep JSON, optional-file behavior, and Unix
symbolic links or special files. An initial Unix socket fixture exceeded the
macOS pathname limit; a short, atomically created owned directory fixes the
fixture without weakening file-type rejection.

Two added real-Agent tests exercise the actual example executable:

- The dangling-admission negative control now fails with `admission.json` in
  the error. The link, missing target and original result are not repaired.
- Each of six evidence types is made oversized and separately given a
  duplicate critical field. Dependent input/result byte hashes are deliberately
  recomputed where necessary, so rejection must come from the duplicate-key
  gate, not an incidental stale hash. All twelve cases fail with the expected
  filename and limit/ambiguity cause. Before/after snapshots prove read-only
  behavior, and the untouched original export still passes.

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target passed all 18 opt-in real-Agent tests on macOS, alongside
the digest/receipt/intent/transport and new file-reader regressions. The full
SDK all-target run passed 156 tests, zero failures, with 18 live tests ignored
by default and separately executed. Clippy passed with warnings denied.
The ordinary count includes the earlier no-op child fixture; overlapping runs
are not additive. Unix link/special-file controls are platform-gated; these
numbers do not claim Windows execution.

The updated native validation profile `modal-sdk-durable-research` also passed
with `executed: true` and `ok: true`. Documentation book/inventory, organization,
formatting and diff-whitespace checks passed. The organization audit reports
no tracked size debt. The refreshed tensor retains zero structural gaps, two
maturity gaps and 23 evidence-grade gaps, with 18 P0 gaps still blocking the
overall readiness gate; this bounded repair does not upgrade that global status.

## Limits

The directory is caller-owned and single-writer. Static leaf/container links
are rejected, but there is no atomic no-follow traversal of every ancestor,
filesystem-race defense, cryptographic authentication, host-power-loss proof
or concurrent-storage transaction guarantee. A hostile filesystem can change
between metadata inspection and opening. The bounded stream read protects its
byte count, not a guarantee of stable contents under concurrent writers.

This does not retroactively qualify historical reports, change the independent
physics tolerances, or close general modal/installed/remote research obligations.
Tensor evidence remains `verified`, not `qualified`. No version, installed
runtime, Git commit or push is part of this turn.
