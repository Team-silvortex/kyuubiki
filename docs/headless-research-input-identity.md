# Headless Research Input Identity

Native Rust Headless research evidence must correspond to the effective batch
that produced its run report. Matching workflow IDs, action lists and validation
summaries is not sufficient: changed parameters can leave all of those unchanged.
The native executor and dry-run producer capture a versioned input fingerprint
before resolving bindings, dispatching actions or compacting report payloads.

## Report Contract

New native `kyuubiki.headless-execution-run/v1` reports contain:

```json
{
  "execution_input": {
    "schema_version": "kyuubiki.headless-execution-input/v1",
    "sha256": "<64 lowercase hexadecimal characters>"
  }
}
```

The fingerprint binds all effective batch fields except diagnostic `warnings`:
schema, export timestamp, language, workflow ID, optional template ID, ordered
steps, step indices, registered risks, action names and complete payloads. Apply
parameter patches and command-level overrides before calling the executor.
Resolved payloads and results remain separately checked by execution contracts;
the fingerprint records the original effective batch, including binding tokens.
It is not a digest of the resolved network request or of external model contents.
Saved-reference solves additionally capture
[saved model sources](headless-saved-model-sources.md), with optional pre-submit
content pins. The batch fingerprint binds any caller pin, not the later GET response.

The input schema lives in
[`headless-execution-input.schema.json`](../schemas/headless-execution-input.schema.json).
`execution_input` remains optional when decoding old run files. Invalid preflight
reports omit it; newly captured dry, blocked or failed runs may carry it, but
that does not authorize research evidence.

## Evidence Admission and Recovery

`build_headless_research_round_evidence` and
`verify_headless_research_round_evidence` recompute the fingerprint from the
supplied effective batch and require exact receipt agreement. Missing,
unsupported or mismatched fingerprints reject admission. A retained execution
failure also rejects evidence even if the top-level report was marked `ok`.
The existing service-mode, full-completion, metric and lineage checks still apply.

Use the native `kyuubiki-headless-sdk` crate's ordinary execution and evidence
helpers. No new CLI wrapper or alternate executor is required:

```rust
let report = execute_batch_with_executor(&effective_batch, &mut service, false, false);
let evidence = build_headless_research_round_evidence(
    &effective_batch, &report, &round_spec, patch_receipt.as_ref(), previous.as_ref(),
)?;
```

If only a derived metric pointer was wrong, correct the spec and rebuild evidence
from the unchanged batch and captured report. This performs no calculation.
If the physical inputs changed, explicitly run the changed batch and retain its
new report. Do not attach a fingerprint computed from the changed batch to an
old report; that fabricates identity instead of repairing it.
Old reports without captured identity remain inspectable, but cannot qualify
new research evidence by guessing, backfilling or copying a fingerprint.

KCore research export and verification call the same evidence verifier.
An archive cannot bypass the gate by resealing changed input artifacts.
Retained v1 research-series archives whose reports lack captured input identity
now fail semantic verification even if their file checksums match. There is no
automatic downgrade to unchecked evidence or invented historical fingerprint.
Run/evidence files with numeric parameters should be retained together with
their effective batches; formatting or object-key order changes are allowed.

## Exact Encoding

The v1 SHA-256 input is the UTF-8 schema string, one NUL byte, and compact JSON
of the effective execution batch with `warnings` replaced by `[]`. Object keys
are sorted lexicographically at every level; arrays and step order are preserved.
An absent `template_id` remains absent. JSON strings use `serde_json` escaping,
and numbers use its lossless numeric serialization, without decimal rounding.
Integer and floating representations, including signed floating zero, remain
distinct when their serialized tokens differ. Numeric text is normalized through
the decoded Rust value, not retained as the file's original lexical spelling.

The implementation borrows the original payload and streams through a 16 KiB
buffer into SHA-256. It does not create a second full batch `Value` or retain a
full encoded JSON string. It still visits every input value; hashing time is
linear in encoded input size, not constant-time or free.
Unsorted object keys need temporary sorted references, not cloned values.

## Scope and Remaining Boundaries

This detects accidental stale-report pairing. It is not authentication:
a caller can deliberately fabricate reports and fingerprints. It does not prove
which Agent executed, which external model a mutable reference resolved to,
or whether a solver/result is scientifically correct. The actual bar regression
checks one independent closed-form relation, not general physical qualification.
Saved-source descriptors separately identify fetched content; they are not
authenticated Agent execution receipts or hashes of normalized solver requests.

Legacy `headless_batch_content_sha256`, parameter-patch receipts, research
lineage hashes and TaskIR canonical encoding are unchanged. Their fixed decimal
number normalization can collapse tiny changes. The exact input gate blocks
stale-report admission despite that collision, but does not make a tiny-change
multi-round patch lineage valid: it may still be refused as unchanged. A future
versioned lineage migration must address that separately rather than relabel old
digests under the same schema.

This API is part of the native Rust execution SDK. It does not add corresponding
Python/Elixir or standalone Rust controller SDK producers. Source verification
does not imply a rebuilt installed App or runtime.
See [the regression and reproduction steps](../reports/headless-research-input-fingerprint-20261008.md).
