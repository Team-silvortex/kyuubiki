# Axial Bar Input Normalization

`solve_bar_1d` accepts the same scalar inputs through inline HTTP models and
Agent-decoded model artifacts. The protocol adapter converts compatibility input
to the existing SI `SolveBarRequest`; the engine and solver remain independent
of Elixir, the GUI, upload transport and display-unit labels.

## Input Rules

| Field | Accepted meaning |
| --- | --- |
| `length` | Required finite number, strictly positive, metres. |
| `area` | Required finite number, strictly positive, square metres. |
| `tip_force` | Required finite number, newtons; negative and zero are allowed. |
| `elements` | Required integral number in `1..=4294967295`. |
| `youngs_modulus` | Finite, strictly positive elastic modulus in pascals. |
| `youngs_modulus_gpa` | Finite, strictly positive elastic modulus in gigapascals; multiplication by `1e9` must remain finite and positive. |

At least one modulus field is required. When both are present, the SI value must
agree with the GPa-derived value within
`4 * f64::EPSILON * max(abs(Pa), abs(GPa * 1e9))`. This permits conversion
roundoff only, not a physical tolerance. The GPa-derived SI value is canonical
when both are supplied. Contradictory units fail instead of selecting one.
An explicitly invalid or null modulus fails even when the other form is valid.

Scalars may be JSON numbers or strings using strict JSON-number syntax, including
exponents. Numeric strings must be nonempty, at most 256 UTF-8 bytes, and have no
surrounding whitespace. Boolean, object, array, null, nonfinite and overflowing
values fail. Fractional element counts are rejected, not rounded as the old HTTP
normalizer did. The portable integer bound is not an execution memory allowance;
Agent resource admission and solver reliability checks remain necessary.

```json
{
  "length": "1e0",
  "area": "0.01",
  "elements": "4.0",
  "tip_force": "1000",
  "youngs_modulus_gpa": "210"
}
```

The normalized request has five native fields, including `youngs_modulus` in SI
and an integer `elements`; serialization does not introduce a GPa compatibility
label. Unrelated model metadata is ignored. Agent artifact decoding skips unknown
metadata directly through the reader without first building a second full JSON
model in the control plane. Invalid nested scalar objects/arrays are rejected
without scanning their complete bodies.

## Admission And Recovery

Inline public HTTP normalization happens before job creation. Invalid inline
input therefore creates no job and starts no Agent execution. Artifact-backed
input is acknowledged as a job first and decoded by its assigned Agent later.
Invalid artifact scalars produce a terminal failed job without a solver result;
the Agent watchdog may count that attempted execution. These are distinct stages.

Native SDK/CLI terminal decode failures preserve `invalid_solver_params` at
`agent_decode`, with `retryable: false` and `retry_strategy: none`. Terminal
status still takes precedence over queue, timeout or connection hints embedded
in a message; unknown-outcome and cancelled receipts retain their own meaning.

A batch must wait successfully for completion before result consumption or later
writes. The native SDK and CLI halt at the failed wait and do not automatically
replay the task. An explicit corrected task can run afterward. Upload and job
records may remain in managed storage; this contract promises neither rollback
of acknowledged uploads nor zero disk use.

Raw [upload identity](headless-model-artifact-identity.md), effective batch
fingerprints and normalized SI values describe different representations. Their
digests must not be substituted for one another. Result-file references also
remain references in current SDK output, not automatically extracted physical
metrics.

## Conformance Scope

[The shared fixture](../schemas/examples.axial-bar-input-normalization.json)
contains ten accepted and 26 rejected cases. Rust protocol decoding and Elixir
HTTP normalization run the same parsed-input cases, plus required-field and
native serialization checks. Rust additionally rejects duplicate known JSON
fields during raw decoding; the fixture does not assert duplicate-key parity for
Jason or every possible IEEE-754 decimal rounding edge.

A real source-built macOS Orchestra/two-Agent regression compares full inline
and approximately 8 MB padded-artifact results for GPa-only, Pa-only,
roundoff-consistent dual units and numeric-string compressive loading. The current
regression requires [native SDK readback](headless-result-artifact-readback.md)
to establish file-result equality without its old test-side downloader.
Independent closed forms verify displacement, stress, reaction, axial force and
strain energy. Retained batch/report JSON can be reloaded and read back without
recomputing. SDK and CLI tests reject contradictory moduli and fractional counts,
block later writes, and recover via a distinct explicitly submitted healthy task.

This closes axial-bar scalar parity only. It does not establish all solver
schemas, mesh defaults/IDs, million-node
scale, authenticated provenance, installed Apps or remote/cross-platform
qualification. See the [verification report](../reports/axial-bar-input-normalization-20261008.md)
with its historical test-side scope, and the separate [native readback proof](../reports/headless-result-artifact-readback-20261008.md).
