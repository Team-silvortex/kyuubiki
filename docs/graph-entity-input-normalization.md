# Graph Entity Input Normalization

Graph entity IDs are part of the input contract, not an Agent repair step.
Native Rust request decoding now generates and validates IDs for ten thermal,
electromagnetic and transport models. All ten now have direct HTTP/Headless
submission routes; the same shared cases qualify their HTTP normalization.
The [transport service follow-up](advection-diffusion-service-chain.md) closes
the formerly protocol-only advection-diffusion entry with actual Agent execution.

## Covered Requests

| Request family | Rust typed decoding | HTTP and live SDK inline/file parity |
| --- | --- | --- |
| Thermal bar 1D | Covered | Covered |
| Steady heat bar 1D | Covered | Covered |
| Transient heat bar 1D | Covered | Covered |
| Electrostatic bar 1D | Covered | Covered |
| Magnetostatic bar 1D | Covered | Covered |
| Advection-diffusion bar 1D | Covered | Covered |
| Heat plane triangle and quad 2D | Both covered | Both covered |
| Electrostatic plane triangle and quad 2D | Both covered | Both covered |

Rust decoding applies the contract to the parent request's node and element
arrays, including nested request decoding used by results and workflows. It is
not a constructor hook: manually constructed Rust structs remain caller-owned.
Standalone entity decoding cannot generate a positional ID without a parent
array. The engine and solver algorithms do not depend on this private helper.

## Identifier Rules

Nodes and elements must be JSON objects inside their respective arrays. A tuple
with all the expected fields is not an alternative entity encoding. The
protocol's map-only visitor rejects it without creating an untyped JSON tree.

- Missing IDs and the empty string generate `n{array_index}` or `e{array_index}`.
- Nonempty strings remain exact, including whitespace, Unicode and casing.
- Explicit `null`, numbers, booleans, objects and arrays are invalid IDs.
- IDs must be unique within each array after generation. Generated/explicit
  collisions reject instead of renaming or silently selecting an entity.
- Node and element namespaces are separate; the same string in both is legal.
- Array order, geometry and numeric connectivity indices remain unchanged.
- Missing graph arrays and required physical fields remain decoding errors.
  Empty arrays still require solver geometry validation.

For example, an unnamed first node and a second node explicitly called `n0`
are ambiguous and reject. An unnamed node at index 2 becomes `n2`, not the next
unused label. Rename the explicit ID or assign all labels deliberately.

The HTTP `FemModelNormalizer` delegates graph-ID handling to `GraphEntityInput`.
Its graph-normalizer callers share strict type/collision rejection, including
existing structural families. This does not establish protocol/file default
parity for those other families. Specialized submission normalizers that do not
call the graph helper retain their own rules.

Rust deserialization retains typed field defaults and ignores unknown metadata
without materializing it. Its uniqueness check stores borrowed IDs in a set;
it does not duplicate all ID strings. The decoded mesh and validation set are
still proportional to entity count. This is not an RSS or scale qualification.

## Transport and Failure Behavior

Inline HTTP input is normalized before admission. Invalid entity IDs or shapes
reject before creating a job. File submissions retain the existing bounded,
digest-verified artifact path and decode directly into the same typed request
on the selected Agent. Invalid file content produces a terminal failed job at
`agent_decode` with `kyuubiki.headless.invalid_solver_params`.

Both failures stop later batch steps. A file decode failure is an acknowledged
job, not an unknown submission; it does not authorize automatic retries or
computation replay. Repair the model before a deliberate new submission.
Healthy subsequent tasks continue through the same runtime.

Agent artifact decoding no longer enumerates four plane types with `Any`
downcasts or applies a separate trim-and-fill policy. That old trim policy
could change literal whitespace IDs on the file path. Protocol decoding is
shared by inline native Agent input and digest-verified file input.

## Conformance and Physical Checks

The [shared fixture](../schemas/examples.graph-entity-input-normalization.json)
contains ten small physical models, six accepted edits and 18 rejected edits.
Rust and Elixir each check all 240 model/edit combinations for the ten direct
HTTP families. Additional tests cover round trips,
required fields, atom-key input, broader HTTP graph callers, duplicate known
JSON fields on Rust ingress and reader skipping of 8 MB unknown metadata.

The [live regression](../workers/rust/crates/cli/tests/support/headless_graph_entity_normalization.rs)
uses a temporary real Orchestra and two Rust Agents. It compares all ten
complete inline/file physical results and a further heat-bar pair with literal
whitespace/empty IDs. File upload receipts match the exact prepared byte digest
and length; native result reads return physical JSON, not just file descriptors.
Retained reports reload without another solve.

Independent checks use prescribed linear heat and electric profiles, stationary
transient heat, thermally expanding loaded bars, magnetic potential profiles
and constant transport concentration with signed advective flux.
They verify temperature, displacement, signed flux and stored-energy metrics
rather than relying only on two paths sharing the same engine.

SDK and CLI attempts reject generated-ID collisions, numeric IDs and complete
tuple-shaped nodes/elements. Inline cases create no jobs; file cases fail at
Agent decode. Guarded project creation never runs. An explicit healthy CLI task
then succeeds; admission counts detect extra or replayed calculations.

The [original report](../reports/graph-entity-input-normalization-20261008.md)
retains its nine-route historical scope. The current ten-route repeat has
exactly 31 admissions (22 paired solves, eight failed file decodes and one
healthy recovery); see the [follow-up report](../reports/advection-diffusion-service-chain-20261008.md).

## Remaining Boundaries

This is source macOS contract evidence with small analytic models. It does not
qualify installed Apps, remote platforms, million-node memory/performance,
Python/Elixir numerical producer/consumer parity or every numerical method.
The transport follow-up separately tests official SDK route/RPC mappings;
broader engine and cross-language numerical acceptance remain separate.
No signed execution provenance is established by transport digests.

Other solver families still need explicit inline/file unit, default, ID and
required-field conformance. Byte-level duplicate object-key behavior outside
typed Rust ingress, mixed atom/string aliases, additional physical parameters
and parsed-result capacity retain their own acceptance boundaries. The
[weakness roadmap](weakness-roadmap.md#roadmap-principle) stays open for them.
