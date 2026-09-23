# Steady planar heat contact, 2026-09-23

## Scope and architecture

This is local component screening, not contact-model release qualification.

The existing triangle and split-quad steady heat operators now accept matching
boundary-edge contacts with independent nodes and finite area-specific thermal
resistance. The complete protocol, units and limitations are described in the
[operator contract](../docs/thermal-contact-operator.md).

Data lives in the protocol crate. Geometry validation, contact integration,
matrix assembly and field recovery live in the solver crate. Engine production
changes are catalog metadata only: no new solver branch, scheduler feature,
lifecycle path or dependency was added. Elixir's catalog metadata is aligned;
its graph-model normalizer already preserves extra model fields. This last
path has a focused local ExUnit regression for both element types, preserving
contact payloads and independent node identities. It is not a live remote-service
test.

The engine does not assemble or interpret the thermal contact law.

Old Rust fixture initializers explicitly add empty contact lists. Serde omits
empty arrays, so no-contact wire inputs/results retain their original shape.
No-contact solves skip the new topology/index work. With contacts, only requested
edges are indexed, plus a disjoint-set topology check over the nodes. This has
not been benchmarked at large scale.

## Regression design

The first seven tests ran before implementation: six failed, while the legacy
no-contact behavior passed. The old solver ignored the extra contact payload;
it could not transfer heat between the bodies or report interface results.

The retained numerical suite covers:

- A two-body serial heat path with area 0.5 m^2, bulk resistances 1 and 0.5 K/W,
  exterior temperature difference 20 K, and area-specific contact resistances
  0.01, 1 and 100 m^2 K/W. Expected power is `20 / (1.5 + 2*R)` W.
- Borrowed and owned triangle/quad entrypoints return consistent temperature,
  bulk flux, interface jump, total flow and endpoint flow.
- A 4 W source-driven body is anchored only through contact with the sink-side
  body. Contact jump is 8 K at `R=1`, and bulk/interface power remains 4 W under
  absolute reference shifts of +/-2^40 K.
- A nonuniform prescribed jump checks exact endpoint integration; a separate
  four-free-unknown rational solution exercises the actual assembled contact
  block rather than just postprocessing prescribed temperatures.
- Subdividing the contact into 1, 2, 4, 8 and 16 matching edges preserves total
  power for both element types, including rotation and coordinate shifts of
  2^40 m. Largest retained fixture: 68 nodes, 32 quads or 64 triangles.
- Side reversal changes only signed transfer. Uniform zero jump and opposing
  local jumps do not generate a net source.
- A concave split quad checks the actual edge-adjacent subtriangle, not an
  arbitrary opposite corner. The initial implementation rejected the valid
  interface in this regression; after correction, the equivalent triangle/quad
  models agree and an overlapping adjacent subtriangle is rejected.
- Invalid IDs, repeated/out-of-range nodes, internal/mismatched edges, overlapping
  bodies, unequal thickness, duplicate contacts, invalid resistance, subnormal
  integration-weight distortion and floating islands fail explicitly.
- Headless plan serialization preserves the input; existing engine entrypoints
  execute it; errors propagate and clean retry succeeds. The SDK temperature
  handoff preserves the jump between coincident but independent node IDs, then
  a real restrained thermal-plane solve matches analytic regional stress.

Scalar physical checks use absolute error below `1e-10 * max(1, |expected|)`;
the downstream stress comparison uses relative `1e-12`. Contact geometry uses
local edge-length tolerance `1e-10`; common thickness and representable
integrated-conductance checks use relative `1e-12`. These tolerances are stated
for the bounded fixtures, not arbitrary material qualification.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-cli --test heat_plane_contact --test heat_contact_operator
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-cli --release --test heat_plane_contact --test heat_contact_operator
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-material-explore
cargo check --manifest-path workers/rust/Cargo.toml --workspace --all-targets --locked
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib --test heat_plane_contact --no-deps -- -D warnings
elixir -r apps/web/lib/kyuubiki_web/fem_model_normalizer.ex -e 'ExUnit.start(); Code.require_file("apps/web/test/kyuubiki_web/fem_model_normalizer_test.exs")'
./scripts/kyuubiki check-operator-validation --execute --profile heat-plane-contact-screening --out tmp/heat-plane-contact-20260923.json
./scripts/kyuubiki check-operator-validation --execute --profile thermal-plane-patch --out tmp/heat-plane-contact-legacy-20260923.json
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

Verified locally on macOS ARM64:

- Debug and release: 15 contact solver tests and 3 SDK/engine handoff tests pass
  in each build mode.
- Core library regression: protocol 108, solver 284, engine 451 and headless SDK
  270 pass; 7 pre-existing ignored tests remain ignored.
- Legacy `thermal-plane-patch`: 11 commands pass, covering 50 tests with one
  intentionally ignored local microbenchmark.
- Standalone Elixir model normalization: 5 tests pass without starting services.
- Material research CLI regression: 39 tests pass.
- All Rust workspace targets pass the locked dependency compile check.
- Strict Clippy passes for protocol, headless SDK, CLI library and the new CLI
  integration target with dependency linting disabled (`--no-deps`).
- The initially blocking solver `type_complexity` warning is resolved in a
  follow-up: `linear_ic0.rs` now returns and stores a private named
  `LowerTranspose`, distinguishing offsets, factor entries and source rows.
  Arithmetic, allocation count and cancellation checkpoints are unchanged.
  Strict solver Clippy for the library and heat-contact integration target now
  passes without a lint allowance. Three added unit regressions cover irregular
  transpose mappings, empty/diagonal-only patterns and a known Cholesky solve
  with workspace reuse. No engine interface or repository lint policy changed.
- Operator-validation metadata and its self-test pass for all 35 profiles;
  this static check is not an execution claim for all 35 profiles.
- Coverage-tensor checks pass with 13 modules, 11 paradigms and no structural
  gaps. Existing maturity gaps (4), evidence-grade gaps (16) and P0 gaps (11)
  remain; overall readiness is still blocked.
- Project organization and documentation-inventory checks pass. Changed source
  files stay within 800 lines and documents within 2000 lines.

## Limits

No pressure-dependent or evolving contact law, contact search, nonmatching mesh,
gap radiation, transient interface storage, 3D contact surface or mechanical
bond model is implemented. The three-layer ideal-interface SDK reference is not
an oracle for this new contact model. Contact capability and returned contact
records must be checked when targeting older separately deployed workers, which
may otherwise ignore fields unknown to their protocol version.

No remote deployment, app reinstall, 1M-node benchmark or release qualification
was performed. Global tensor readiness is not promoted by these local tests.
