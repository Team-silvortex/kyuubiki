# Typed TaskIR numeric identity verification

Source macOS verification on 2026-10-11 adds explicit opt-in typed binary64
TaskIR digest encoding. Rust and Elixir produce the same digest and the control
plane, Rust SDK and Agent independently recompute it. This addresses fixed-15
aliasing for opted-in tasks without changing physical inputs or task structure.
Default rollout and installed v2 crash recovery remain open.

## Contract and implementation

- Task structure stays `kyuubiki.operator-task-ir/v1`.
- `integrity.task_digest_encoding` selects
  `kyuubiki.task-digest/typed-ieee754-v2` explicitly; an absent field alone means
  legacy `kyuubiki.task-digest/fixed15-v1`.
- SHA-256 hashes a version-specific zero-terminated domain then typed,
  length-prefixed values. Object keys sort by UTF-8 bytes, arrays retain order,
  integers use minimal decimal text and floats retain their eight binary64 bytes.
- The compiled eleven-field task projection is unchanged. Supplied field lists
  cannot narrow coverage; descriptor digests remain on their historical contract.
- Types, integer/float representation, signed floating-point zero, tiny values
  and adjacent floats remain distinct. Numbers are never rounded or rewritten
  in execution payloads.
- Unknown/null/non-string encoding and unsupported algorithms return errors.
  Removing or downgrading a typed marker without resealing fails digest checking;
  verifiers do not try other encodings to guess an identity.
- `seal_operator_task_digest` is exposed by Rust Protocol and Headless SDK;
  Elixir uses `OperatorTaskIR.seal_task_digest`. Rust mutation happens only after
  successful computation; Elixir returns no changed task on failure.
- The JSON schema declares encoding selection and absent-only fallback. Native
  contract checks validate that declaration and the shared vector file instead
  of computing task digests through a separate fixed-15 implementation.

The numeric model is decoded finite binary64 plus integers in the combined
int64/uint64 range. This is not arbitrary-precision decimal arithmetic or an
identity for original JSON token spelling. Different decimal tokens that decode
to one binary64 execution value share that value's identity. Elixir refuses
out-of-range integers, invalid UTF-8, non-string keys and non-JSON values. Hash
depth is limited to 128; transport parsing may impose stricter limits.

Both encoders update SHA-256 incrementally, borrowing the current invocation
instead of cloning a large artifact into an encoded JSON buffer. Sorting still
needs transient per-object key-entry storage. There are no new retained input
copies, snapshots, database tables or backup roots. No throughput, allocation
benchmark or large-mesh qualification is claimed by these tests.

## Shared vectors and negative tests

`schemas/examples.task-ir-typed-digest.json` contains 26 small codec vectors:
null/booleans/strings, integer and floating-point zero, signed zero, tiny and
adjacent floats, the smallest nonzero binary64, the largest finite binary64,
integer range boundaries, nested structures and Unicode string/key ordering.
They include expected SHA-256 values from the defined byte grammar, not merely
equality between two implementations.

The same fixture describes a deterministic 1,024-value binary64 bit corpus.
Both implementations generate it independently, match one expected digest and
retain that identity through their own JSON serializer/parser. The corpus is
generated in tests rather than retaining another large data file.

Rust and Elixir each exercise six numeric/type drift pairs across all eleven
projected fields (66 checks per implementation). Input, config, descriptor,
execution program and routing/context values cannot bypass exact identity.
Tests also cover removed/forged encoding, unsupported algorithms, ignored caller
field lists, key order, depth rejection and idempotent/atomic authoring. Existing
v1 golden fixtures remain unchanged. Protocol smoke fuzzing now mixes legacy
and typed task seeds under JSON/value and byte mutations.

API tests confirm `200` for a valid typed task, `422` with a digest mismatch for
tiny drift and an explicit `operator_task_digest_invalid` code for malformed
encoding/algorithm. A later valid task succeeds after each rejected request.

## Real source transport

`real_rust_and_elixir_typed_tasks_are_verified_independently_by_agent_and_http`
starts an isolated Rust Agent and source Orchestra HTTP service. Five physical
diagnostic domains are covered: thermal, electrostatic, magnetostatic,
thermomechanical and Stokes flow.

For each domain, one task is authored by Rust and one by a separately invoked
Elixir authoring helper. The helper lowers an actual descriptor and seals the
complete task; this is not a Rust fixture relabelled as Elixir. All ten tasks
have no `workflow_native_invocation_sha256` adapter binding. The Rust SDK verifies
each Elixir result; the source HTTP prepare route independently verifies every
complete task; TCP Agent execution independently recomputes its digest and
returns the expected diagnostic result with the same identity.

Each task then produces three negative requests: tiny config drift that still
aliases under old canonical JSON, deletion of its typed encoding marker and an
unknown encoding. All 30 are rejected by both HTTP prepare and Agent execution
admission. Agent failures name `verify_digest`, not a later computation stage.
Ten subsequent valid Agent preflight requests succeed, showing that rejection
does not poison the next task. Raw physical results are recomputed as small
test inputs; they are disposable and removed with the owned temporary harness.

This verifies ten diagnostic executions and 30 pre-execution rejections, not
ten production research runs, crash recovery, independent numerical accuracy
or exactly-once execution. HTTP prepare and Agent execution are separate legs;
this test does not claim the generic HTTP execute route dispatched those tasks.

## Verification results

- Expanded SQLite recovery/admission selection: 727 listed, 718 passed,
  9 JSON-only skipped, zero failed, 31.4 seconds.
- Isolated JSON selection: 414 passed, zero failed, 9.8 seconds.
- Rust Protocol full library: 139 passed, zero failed.
- Rust Headless SDK full library: 534 passed, zero failed, 33.09 seconds;
  the long-response test intentionally waits beyond the old 30-second cutoff.
- Agent TaskIR RPC unit selection: 32 passed, zero failed.
- Rust Engine full library: 650 passed, one prebuilt-plugin test ignored,
  zero failed.
- New real Rust/Elixir HTTP/TCP test: one passed, 1.87 seconds.

These counts include existing regression selections, not only new tests. The
new tests do not extend a numerical solver's physical accuracy qualification.

Strict Rust/Elixir compilation, formatting, TaskIR contract checks/self-test,
module topology/matrix/extension checks, runtime API surface, document inventory
and all 26 book HTML files pass. Version audit retains 224 exact matches with
zero mismatches; source/document limits remain 800/2000 with zero tracked debt.
The coverage tensor remains 0 structural gaps, 4 maturity gaps, 20 evidence-grade
gaps and 15 P0 gaps. Verified opt-in support does not close the all-author
qualification requirement or change Daji's blocked qualification status.

## Rollout still required

The tensor records verified opt-in contract support, not qualified universal
numeric identity. Its existing all-author requirement remains open because:

1. Current builders and retained jobs still default to legacy encoding.
2. Source routing now requires an explicit encoding capability; authenticated
   installed mixed-peer acceptance is still separate work.
3. Bounded source native workflows now restore solves, branches and receipts
   under explicit v2 selection. Installed/remote recovery and actual PostgreSQL
   acceptance remain open. See the
   [workflow recovery follow-up](workflow-typed-execution-recovery-20261011.md).
4. External executable package hosts, installed platforms and separately
   distributed Python/Elixir SDK authoring paths are not newly qualified.
5. Descriptor-only identities remain fixed-15; a matching descriptor digest
   alone is not a claim of lossless numeric descriptor identity.

The next adoption step should negotiate required digest encoding before
dispatch, make newly authored tasks choose it explicitly, and reject legacy
recovery evidence when the new identity is required. Old records must not be
silently resealed or used to authorize replay. Caller-recomputed hashes are
integrity checks, not authenticated identity or execution permission.
