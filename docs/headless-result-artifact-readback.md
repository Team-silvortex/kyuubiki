# Headless Result Artifact Readback

Native Rust Headless `result_fetch` now resolves a supported immutable result
reference into the actual solver JSON object. Research metrics and workflow
bindings can use physical fields directly, without a test-only file downloader
or resubmitting the calculation. This applies to both preferred embedded job
results and the separate result endpoint, after completed-job identity and
requested project/version association checks.

## Read Policy

The following optional payload fields belong to `result_fetch` and the combined
`solve_and_wait_from_model_version` action. Invalid types or out-of-range values
fail before that action performs service I/O; combined actions validate them
before submitting a solve.

| Field | Default | Accepted value |
| --- | --- | --- |
| `resolve_result_artifact` | `true` | JSON boolean. False validates and retains the reference without downloading. |
| `result_artifact_max_bytes` | `67108864` (64 MiB) | JSON integer from 1 through `536870912` (512 MiB). |
| `result_artifact_timeout_ms` | `600000` | JSON integer from 1 through `600000`. |

The byte budget limits a single raw content download and its owned temporary
file, not process RSS, JSON object expansion or the sum of retained reports.
Increasing it is an explicit caller capacity decision. For very large results,
use reference-only mode and a separate application-owned processing strategy:

```json
{
  "job_id": "{{steps.2.result.job_id}}",
  "resolve_result_artifact": false
}
```

The deadline is nonrenewable across connection, request and network reads.
Decoding has checks before and after JSON parsing, not a preemptive CPU timer.
It does not replace the earlier job wait or control-plane metadata read limits.
Automatic network replay after a content GET is not performed.

## Descriptor And Download

Supported wrappers declare `kyuubiki.solver-result-reference/v1`,
`storage_mode: orchestra_content_addressed` and a bounded nonempty solver-method
identifier. The descriptor requires the six protocol fields declared in the
[normalized result reference schema](../schemas/result-artifact-ref.schema.json).
Artifact ID and SHA-256 must be identical lowercase 64-digit digests; size must
be a positive integer, media type must be
`application/vnd.kyuubiki.result+json`, and `immutable` must be boolean true.
Unknown extensions are not retained in a verified descriptor.

Only the configured control plane receives the authenticated GET:
`/api/v1/result-artifacts/{artifact_id}/content`. Descriptor URLs are never used.
Redirects are rejected, not followed, and tokens never travel to a supplied
artifact location. This retains the native SDK's existing HTTP-only transport;
it does not introduce HTTPS or a cryptographic execution identity.

Content must return HTTP 200 and the expected media type. Exact Content-Length
or bounded chunked framing is supported, including fragmented network reads.
Conflicting length/transfer headers, duplicate representation headers,
compression, unframed content, unsupported chunk trailers, truncation, excess
bytes and oversized headers are rejected. Chunk-decoded raw bytes are streamed
in at most 64 KiB writes to an exclusively created temporary file while hashing.
Digest and length are checked before parsing JSON; there is no second complete
encoded response buffer or separate hash-only file read.

Unix spool files use mode `0600`. Their owning handle is closed before removal
on normal/error return, including on platforms that require closed handles.
This is not a crash cleanup guarantee or an isolated filesystem sandbox.
Hash-verified content must still be a JSON object; nested result references are
rejected rather than recursively downloaded.

## Physical Result And Receipt

The existing `result` field contains the physical object after successful
readback. The small `result_artifact_readback` companion records the original
descriptor, solver-method label and effective read limits:

```json
{
  "job_id": "completed-bar-job",
  "result": {"tip_displacement": 4.761904761904762e-7}
}
```

This illustration shows only the physical fields; the companion's full shape
is declared in the [receipt schema](../schemas/headless-result-artifact-readback.schema.json).
Inline results retain their existing shape without a fabricated receipt.
Reference-only reads likewise do not invent verification. Mock/dry previews
expose null, not a verified digest. A binding to the companion requires an
actual artifact-backed output; an inline result has no such companion.

For a plain third-step result fetch, a research metric pointer can be
`/steps/2/result_preview/result/tip_displacement`. A combined saved-version
solve/wait still wraps the fetched result one level deeper. Report compaction
continues to apply; large arrays in previews are not complete mesh exports.
The descriptor and physical metric can survive retained report reload without
another result download. Byte integrity and solver labels are not scientific
qualification, signed provenance or an independent result-identity gate for
arbitrary imported reports.

## Failure And Recovery

Read policy, descriptor, framing, digest or JSON-object rejection produces
`kyuubiki.headless.result_artifact_readback_failed`, stage `result_fetch`,
`retryable: false` and `retry_strategy: none`. No successful result binding or
later batch step follows it, even if another hint mentions connection failure.
The acknowledged computation remains completed on the service. Retain its
`job_id`, inspect the content and policy, and explicitly retry only
`result_fetch` after repair. Never replay the solve or full batch automatically.

The [source regression](../reports/headless-result-artifact-readback-20261008.md)
uses real temporary Orchestra and two Rust Agents. SDK and CLI extract bar
displacement, independently check `FL/(EA)`, reject deliberate raw-content
corruption without later project writes, then reread the same job and rebuild
research evidence from the retained CLI report. One Agent calculation occurs
throughout. This is bounded macOS source acceptance, not installed App, remote,
cross-language or million-node qualification. Other solver inline/artifact
normalization remains in the [weakness roadmap](weakness-roadmap.md).
