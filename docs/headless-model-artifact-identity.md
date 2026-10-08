# Headless Model Artifact Identity

Native Rust Headless direct FEM submission streams a large model to the control
plane instead of sending its entire JSON through the job endpoint. A valid-looking
artifact ID alone cannot prove that the reply identifies the model just uploaded.
The upload boundary now verifies the exact transmitted content before submitting
the solver job. This complements the
[effective batch fingerprint](headless-research-input-identity.md) and
[saved model source](headless-saved-model-sources.md) contracts.

## Upload Receipt

The response must contain an explicit `artifact` object. Its supported fields are:

| Field | Required meaning |
| --- | --- |
| `schema_version` | Exactly `kyuubiki.model-artifact-ref/v1`. |
| `artifact_id` | Lowercase SHA-256 of the bytes actually transmitted. |
| `sha256` | The same transmitted-byte digest. |
| `size_bytes` | An unsigned JSON integer exactly equal to the transmitted length. |
| `media_type` | Exactly `application/vnd.kyuubiki.model+json`. |
| `immutable` | Boolean `true`. |

Before checking the reply, the SDK compares the transmitted digest and length
with those recorded while preparing its temporary file. Even a valid reply for
changed temporary content cannot silently replace the intended upload. Unknown
reply extensions are not forwarded as reference fields. See the normalized
[reference schema](../schemas/model-artifact-ref.schema.json); cross-field and
prepared/sent-byte agreement are semantic checks beyond JSON Schema.

Hashing occurs while serializing and while sending bounded chunks, without a
second full encoded model buffer or an extra hash-only file read. It still adds
linear SHA-256 work; this round is not a performance benchmark. Unix temporary
files are created exclusively with mode `0600` and removed by their owner on
normal/error return. This is not an isolated filesystem sandbox or crash-cleanup
guarantee.

## Retained Output

Successful native direct-FEM outcomes expose `model_artifact_upload`. A verified
SDK upload produces the normalized six-field descriptor; an inline submission
produces `null`. Saved/direct-mesh solve wrappers preserve it, and the combined
solve/wait action also retains the same value under `solve.model_artifact_upload`.
The small descriptor survives report compaction and is available to bindings:

```json
{"retained_upload": "{{steps.1.result.model_artifact_upload}}"}
```

An externally supplied `model_artifact_ref` is not an SDK-performed upload and
does not receive an invented verified upload record. Mock/dry previews use null,
not a fabricated digest. `model_source`, `execution_input` and the upload digest
bind different representations; their hashes are not interchangeable.

Research evidence preserves retained run output under its existing input/source
admission. Upload metadata is not a physical result metric and remains outside
the metric-pointer whitelist. This round does not add an upload-specific
semantic admission gate or make every legacy report require upload metadata.

## Failure And Recovery

A parsed reply contradicting prepared/sent content or its required typed fields
fails with `model_artifact_receipt_invalid:`. The standard failure receipt has
category `contract_failure`, stage `artifact_upload`, `retryable: false` and
`retry_strategy: none`. No solver POST, new calculation or downstream batch step
follows this rejection. Inspect the discrepancy before an explicit new attempt.

The upload may already exist on the server. SDK failure does not remove a remote
content-addressed object or promise zero server disk use; retention belongs to
the server's managed storage policy. Lost, malformed or oversized transport
acknowledgements retain the existing unknown-outcome failure policy. Neither
case authorizes automatic batch replay.

## Known Unfinished Boundaries

The axial-bar scalar mismatch found by the live test is now closed by a
[shared normalization contract](axial-bar-input-normalization.md). Both inline
HTTP and Agent artifact decoding accept GPa-only, Pa-only and roundoff-consistent
dual units; invalid units and fractional counts fail. The upload identity test
no longer needs an extra SI field. The [graph entity contract](graph-entity-input-normalization.md)
also closes ID defaults/type/collision and object-shape parity for ten field
submission routes, including the [transport service chain](advection-diffusion-service-chain.md). Other
solver schemas and defaults still need inline/artifact conformance; neither
scoped proof establishes complete format parity.

An Agent configured for artifact transport publishes results as file references.
Native `result_fetch` now performs [bounded verified readback](headless-result-artifact-readback.md)
by default, returning physical fields with a separate raw-byte receipt. The
upload identity regression now extracts actual bar displacement through the
SDK and retains it as its research metric. The historical upload-only report
keeps its original test-side readback scope; the newer readback regression
adds SDK/CLI corruption refusal and explicit same-job read recovery. Other
solver normalization and large-result capacity qualification remain open in
the [weakness roadmap](weakness-roadmap.md#roadmap-principle).

Further solver acceptance must pair inline/file inputs using consistent units,
defaults, IDs, required fields and contradictory values without full-model
control-plane buffering. Native result readback now extracts physical metrics,
retains/reloads evidence and blocks replay on read failure within explicit
download limits and no-redirection/auth policy. Parsed JSON memory expansion,
other solver content contracts and large-result scale still need qualification. Upload-only proof cannot qualify
this broader journey at `sdk-headless/sdk_headless` or
`runtime-agent-cli/solver_execution`.

## Proof Scope

The macOS source regression uses a temporary real Orchestra and two Rust Agents.
It uploads one valid model, substitutes that existing same-size model's reference
into a different upload's reply, and checks SDK and CLI refusal without extra
jobs/calculations or later writes. A separate explicit healthy run succeeds;
upload bindings and retained evidence survive reload without computation replay.

The approximately 8 MB padded four-element bar fixture tests transport, not
large-mesh scale. Hash agreement is not authenticated execution, normalized
request identity or independent qualification of every physical solver. Installed
Apps, remote platforms and Python/Elixir producer parity are not verified here.
See the [verification report](../reports/headless-model-artifact-identity-20261008.md).
