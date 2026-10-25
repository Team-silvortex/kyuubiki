# Retained Result Content Binding

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [terminal observation consistency](headless-terminal-observation-consistency-20261025.md).
This extends the official Rust headless SDK and the retained modal research
reader. Solver numerics, Engine/Agent execution policy and wire formats are
unchanged. It is not installed or general modal qualification.

## Reproduced Gap

The offline example compared the parsed export to the original receipt using
ordinary `serde_json::Value` equality. That comparison treats positive and
negative floating-point zero as equal. A real native Agent modal result was
exported; a test-owned copy flipped one restrained displacement's zero sign
and updated its file hash. Offline verification incorrectly succeeded. The
new regression failed on the old implementation before the repair.

This is a consistency gap, not evidence that either zero sign changes the
modal physics. A physically acceptable result and an internally consistent
file hash do not establish that the original result was retained unchanged.
The Agent ledger and original export were not modified by fault injection.

## SDK Contract

`DurableExecutionHandle::validate_completed_result(receipt, result)` checks the
receipt's identity and successful terminal state, then compares its result with
the supplied parsed value using the bounded typed encoder. Both payloads are
borrowed, not cloned or serialized into additional whole-result buffers.

The internal result-content digest has a separate domain and is not a public
wire digest, exchange format or persisted record. The shared request-digest
golden vectors and terminal-observation comparison remain unchanged.

- Preserve binary64 bits, signed zero, full integer precision and numeric type.
- Preserve array order and absent versus explicit-null fields within results.
- Permit object-key order, whitespace and equivalent spelling of decoded values.
- Reject excessive nesting in either retained or candidate content.
- Reject foreign, contradictory, running or unknown receipts before accepting a result.
- Return the original structured solver error for a valid failed receipt.

This deliberately uses typed identity rather than numerical tolerance. For
example, `21.0` and `21` are different retained result types. Inputs supplied
as already-parsed values cannot retain discarded JSON token spellings or
duplicate-key evidence; callers still need an appropriate parser/file policy.
Work is proportional to content size with per-object key sorting; no benchmark
or exact heap-budget claim is made.

## Research Integration

Offline modal verification keeps three separate gates:

1. Match the exact exported bytes against their retained SHA-256.
2. Bind decoded result content to the original successful receipt via the SDK.
3. Independently reassemble and recheck the supported synthetic model physics.

The example now replaces its ordinary JSON equality check with gate 2. Errors
name `verified/result.json` and `receipt.json`. Verification remains read-only,
offline and unable to repair files or authorize replay. Existing online export
staging, exact-byte reread and retained terminal reconciliation are unchanged.

The new live regression submits and exports through the real Agent, stops it,
then checks test-owned copies. Signed-zero, one-ULP frequency and float-to-integer
mass substitutions all still pass the independent physical reference, and their
saved hashes are updated. All are rejected as different original results.
A compactly reformatted file with an updated hash remains accepted. Every
offline check asserts unchanged file paths and bytes; the original receipt and
original export remain intact.

Five SDK tests add roundtrip coverage for normal/subnormal/extreme finite values,
integer precision, nested values, scalar payloads, structural differences,
equivalent JSON spellings, terminal identity/failure propagation and depth bounds.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target passed all 20 opt-in live Agent tests and the shared digest,
receipt, intent, transport and bounded-file regressions. The full SDK run passed
167 tests, zero failures, with 20 live tests ignored by default and executed
separately. Counts include the existing no-op child fixture; overlapping runs
are not additive. Clippy passed with warnings denied.

The native `modal-sdk-durable-research` validation profile passed with
`executed=true`. Documentation book/inventory, targeted Rust formatting,
project organization and `git diff --check` passed. The source/document limits
remain 800/2000 lines with zero tracked organization debt. Tensor structure
passed with zero structural gaps; two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps remain. This bounded repair does not close those readiness gates.

## Limits

This does not authenticate either record or defeat coherently rewritten history.
It does not make an identity-consistent result physically qualified, provide
power-loss durability, or qualify installed/cross-language/remote deployments.
The tensor claim remains bounded `verified`, not `qualified`. No package version,
installation, Git commit or push is part of this change.
