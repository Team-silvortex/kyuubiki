# Graph Entity Input Normalization Verification

## Source Scope

October 8, 2026. Source macOS working-tree overlay on `f5a4247a`
(`daji 3.5.3` commit label); packaged metadata remains `3.5.0`. This round keeps
the preceding axial-bar normalization and native physical-result readback work.
It does not include a version bump, commit, push or installed App rebuild.

## Defects and Contract

HTTP graph input generated missing IDs, but most 1D field requests still
required them on Agent file decoding. The Agent repaired only four 2D heat/
electrostatic types with `Any` downcasts. Its whitespace trimming differed from
HTTP's literal-string policy. Both lacked collision refusal after generation.
HTTP also silently repaired wrong-typed IDs and passed through nonobject entries.

Typed parent request decoding now owns ID generation and uniqueness validation
for thermal, steady/transient heat, electrostatic, magnetostatic and advection-
diffusion bars, plus triangle/quad heat and electrostatic planes. Missing/empty
IDs generate positional labels; literal nonempty strings remain exact. Null and
other wrong types reject. Generated and explicit IDs share the same per-array
uniqueness gate. Node and element namespaces remain separate. Numeric topology
and all physical fields remain unchanged.

An additional failing regression showed that complete tuple-shaped arrays were
accepted as typed nodes by default Rust struct deserialization. A map-only typed
visitor now rejects those arrays without constructing a JSON value tree. The
Agent's type-specific repair shim and unnecessary `Any`/`'static` constraints
are removed; no solver algorithm or engine interface is modified.

## Shared Conformance

The single JSON fixture supplies ten small models, six accepted edit cases and
18 rejected edit cases. Rust checks **240 model/edit combinations**. Elixir
checks **216** for the nine existing direct HTTP families. The test explicitly
asserts the one excluded family rather than silently skipping unknown entries:
advection-diffusion has an engine workflow entry but no direct HTTP/SDK action.

Accepted edits cover omitted, explicit, empty, literal whitespace, Unicode and
cross-namespace IDs. Rejected edits cover five wrong ID types for each entity
kind, explicit/generated collisions, nonobjects and complete tuple-shaped
nodes/elements. Additional tests check canonical round trips, required arrays/
physical fields, 8 MB metadata skipping, duplicate known JSON fields on Rust
ingress, atom-key controls and strict rejection in broader HTTP graph callers.
The Protocol qualification contract now requires 125 unit tests and all five
new graph conformance tests.

## Actual SDK and CLI Chain

An isolated real Orchestra with temporary state/artifact storage and two Rust
Agents runs nine model families through ordinary and actual file submissions.
Every complete physical result agrees across both paths, including generated
IDs. Input files use approximately 8 MB disposable metadata padding to select
transport, not a committed large mesh. Upload receipts match prepared byte
length/SHA-256. Native SDK result readback yields physical fields directly.
Retained reports reload with identical results and no additional calculation.

A tenth pair uses literal whitespace and empty heat-bar IDs and proves exact
preservation/generation across both paths. Independent analytic checks verify:

- Thermal bar: 0.00010005 m displacement, 10000 Pa stress and 100 N force.
- Heat bar: middle temperature 30 and signed -80 heat flux in the model's SI profile.
- Stationary transient heat: unchanged linear temperature history, final time
  0.3 and total thermal energy 360.
- Electrostatic bar/planes: field magnitude 1, flux density 2 and stored energies
  1, 0.25 and 0.5 for bar, triangle and quad respectively.
- Magnetostatic bar: middle potential 1, signed field -2, flux -8 and energy 8.
- Heat planes: signed x flux -80 and zero y flux.

Four invalid heat-model cases exercise both SDK and CLI: generated node-ID
collision, numeric element ID, complete node tuple and complete element tuple.
Their eight inline attempts create no jobs. Their eight file attempts become
terminal failed jobs at Agent decode, with no result or downstream guarded
project write. Failure reports are non-retryable and survive CLI report reload.

An explicit healthy file-backed CLI task then succeeds. Final task admission
count is exactly **29**: 20 successful paired runs, eight failed file decodes
and one healthy recovery. Actual job counts and both Agent counters agree;
project state is unchanged and both Agents are accepting work. There is no
automatic resubmission or hidden extra task.

## Selected Regression Results

**1531 tests pass**, with one existing Engine test ignored. Focused repeats and
qualification reruns are not added to this count. This is selected regression
breadth, not full repository coverage.

| Suite | Passed |
| --- | ---: |
| Native Headless SDK | 527 unit and 25 integration |
| CLI | 16 unit and 97 selected integration, including 56 live/harness tests |
| Engine | 637 unit; one existing ignored |
| Protocol | 125 unit and seven integration |
| Solver selected field accuracy/reliability | 32 integration |
| Elixir normalizer, structural/field and artifact APIs | 65 |

All-target Clippy for Protocol, CLI and native Headless SDK passes with warnings
denied. Rust format checks pass. Elixir compiles without warnings; targeted Elixir format checks pass.
The documentation inventory, book check (26 HTML files) and exact version
contracts (224 checks) pass without changing packaged versions.

Both local qualification reports pass verification: Headless requires 543 tests
across two suites; Protocol now requires 125 tests, retaining 61 RPC methods and
five TaskIR examples. Reports remain small ignored local JSON files. Topology,
module/function matrix, extension standard and runtime API surface checks pass.
Organization has zero tracked debt at source/document limits of 800/2000 lines.

Tensor self-test and actual checks pass with zero structural gaps, four maturity
gaps, 19 evidence-grade gaps and 14 P0 gaps; Daji readiness remains blocked.
The existing service-action evidence now names the protocol normalization helper
and shared conformance tests instead of the deleted Agent `fill_model_ids` shim.
The new field-graph proof is contract-only, not a maturity/release promotion.

## Acceptance Limits

The contract covers ten typed Rust requests and nine actual direct service
families. It does not establish every solver's unit/default conformance, direct
advection-diffusion SDK support, installed App usability, remote acceptance,
Python/Elixir SDK producer/consumer parity or million-node performance. The
broader HTTP graph helper rejects ambiguous IDs, but unrelated file request
types still need explicit default/required-field qualification.

Typed mesh storage and the uniqueness set grow with entity count; large metadata
skipping is not a total RSS guarantee. Byte-level duplicate object-key parity
outside Rust ingress and mixed alias behavior retain separate boundaries.
Independent checks qualify these simple profiles, not every numerical regime
or authenticated/signed execution provenance. Coverage evidence is deliberately
contract-only and cannot promote installed/remote/release readiness.
