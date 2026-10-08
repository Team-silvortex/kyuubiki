# Headless Saved Model Sources

An unchanged batch can point at changed external model content. Native Rust
Headless now records the snapshot read for each saved-reference solve and accepts
an optional caller pin. This complements the
[effective batch fingerprint](headless-research-input-identity.md), which binds
the reference tokens and pin but does not fetch or hash external model content.

## Capture and Pin

The source contract is `kyuubiki.headless-model-source/v1`. It contains
`source_kind`, `project_id`, `model_id`, optional `model_version_id`, solver
`kind`, and a lossless content SHA-256. See its
[schema](../schemas/headless-model-source.schema.json).

Successful `solve_from_model_version`, `solve_and_wait_from_model_version`, and
saved-reference `direct_mesh_solve` outcomes expose `model_source`. The combined
solve/wait action also keeps the same descriptor under `solve.model_source`.
Inline direct-mesh solves expose `model_source: null`, not an invented source.
The small descriptor survives report compaction and can be bound as
`{{steps.1.result.model_source}}` by a later step.

To freeze a selected snapshot, obtain its explicit saved record and use the
native SDK helper. Pass the nested `version` or `model`, not its HTTP envelope:

```rust
let source = headless_saved_model_source(
    &saved_version_record,
    HeadlessModelSourceKind::ModelVersion,
)?;
let payload = serde_json::json!({
    "model_version_id": source.model_version_id,
    "project_id": source.project_id,
    "endpoints": ["127.0.0.1:5001"],
    "expected_model_source": source,
    "timeout_ms": 5000,
    "interval_ms": 10
});
let outcome = service.execute_step("solve_and_wait_from_model_version", 1, &payload)?;
```

A source captured by an earlier successful run may also be reused as the pin
for an explicit later run. Do not turn a changed current model into a purported
historical baseline by relabeling its descriptor.

## Fail Before Computation

`expected_model_source` is optional for the three saved-reference solve actions.
When present, it must be a complete supported descriptor. Null, unknown fields,
invalid identities, unsupported schemas, bad hashes, or a source type/selected-ID
conflict are errors, not requests to disable verification.

The loader validates the returned record identities, parents, kind and object
payload, computes its source fingerprint, and compares the entire descriptor.
Any mismatch fails with `model_reference_invalid:` before solver POST, large-model
artifact upload, job creation, or downstream steps. It offers no automatic replay.
The pin and descriptor are SDK metadata; they are not sent as physical model
fields to the solver. Pins on inline direct-mesh inputs are rejected.

Without a pin, the SDK records the fetched snapshot but does not promise that
the caller chose a fixed baseline before execution. A mutable model source is
not equivalent to its latest saved version: its `model_version_id` records the
latest-version label if present, but does not assign that version to the job.
Choose a saved-version reference and pin when historical repeatability matters.

## Evidence Admission

Native research-round admission requires a supported `model_source` for every
saved-reference solve. Its identities must match the effective batch reference
and parent hints. Pins and reference bindings are resolved against prior retained
step outputs, not blindly trusted resolved-payload labels. A combined action must
retain identical outer and nested source descriptors. KCore research semantic
verification uses the same evidence gate.

Legacy run JSON remains readable. Saved-reference reports without a captured
source cannot qualify new research evidence by backfilling a guessed descriptor.
Execute the explicitly selected snapshot again. Inline and non-model actions do
not require saved-source descriptors.

## Exact Hash Projection

The SHA-256 input is the UTF-8 source schema string, one NUL byte, and compact
lexicographically key-sorted JSON containing the descriptor fields except
`sha256`, the complete saved `payload`, and record-level `material` and
`model_schema_version` when present. The source type and all IDs are included;
the version label is always encoded as a string or null. Missing material/schema
fields differ from explicit null. Display names, timestamps and other bookkeeping
outside the payload are excluded. Metadata inside the payload is bound.

Arrays retain order. JSON numbers retain lossless Rust serialization rather than
legacy fixed-decimal rounding. A tiny or adjacent-float content edit changes the
fingerprint. Hashing borrows the saved record and streams through a 16 KiB buffer
without retaining a second encoded full-model string; cost remains linear.

## Limits

This is fetched-snapshot consistency, not authenticated provenance or an Agent
attestation. The descriptor does not hash normalized solver-request bytes, prove
which engine/operator artifact ran, or establish physical correctness. Existing
job identity, completion and result-read gates still apply. The real axial-bar
regression checks one independent closed-form relation, not all solver physics.

The live regression uses source-built macOS Orchestra and two Rust Agents with
temporary state. It proves no remote, installed-App, large-scale, or Python/Elixir
producer parity. See the
[verification report](../reports/headless-saved-model-sources-20261008.md).
