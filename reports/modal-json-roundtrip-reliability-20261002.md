# Modal JSON Readback and Independent Protocol Contract

Date: 2026-10-02
Version: daji 3.4.3 working tree
Scope: local macOS, independently built Rust Protocol/Solver and in-process Rust headless dispatch

This is bounded Rust JSON numerical-contract validation, not general modal qualification.

## Reproduced Defect

The [published-shape validation](modal-published-shape-reliability-20261002.md)
checks the actual in-memory physical output. It explicitly leaves text/network
JSON round trips outside that proof. A successful solve must not be treated as
proof that its serialized result retains the same binary64 numbers.

Protocol did not request `serde_json/float_roundtrip`. Full applications happened
to obtain it through Project Bundle, while independently built Protocol and
Solver did not. The same JSON payload therefore had feature-dependent readback.
Independent Headless and Rust-only Operator SDK builds inherited the same gap.

Before the fix, all four initial protocol regressions and both new actual
Solver-output regressions failed. The codec corpus changed
`1.00952715248558e-308` to `1.0095271524855797e-308`, one binary64 bit step away.
Actual planar and spatial modal shape components also changed on JSON readback.
A finite numeric digest fixture then computed a different task digest after
serialization, so verification could not retain its original integrity value.
The digest fixture is not an admitted executable TaskIR or an Agent journey.

These failures prove a lossless-codec contract defect. They do not establish
that every altered shape exceeds the physical residual tolerance.

## Change and Ownership

Protocol now explicitly enables `float_roundtrip`, so its dependents no longer
rely on another package incidentally enabling accurate decimal readback. Cargo
feature trees confirm the setting in independently selected Headless and
Operator SDK packages as well as Protocol itself.

No new package, JSON field, task format, digest algorithm, eigenvalue algorithm
or numerical tolerance is introduced. Engine dispatch stays decoupled from
the solver. Full applications that already enabled this feature do not acquire
a new parsing mode from this change. Already-altered historical numeric payloads
cannot be repaired retroactively.

Accurate decimal parsing may cost more CPU than the approximate fast path.
This report makes no parsing-performance or benchmark-improvement claim.

## Retained Coverage

Nine new tests are organized by boundary rather than one monolithic harness:

- Five Protocol cases exercise compact/pretty text, slice and reader decoding,
  planar/spatial modal result DTOs, material/geometry DTO fields, numeric digest
  consistency, invalid/out-of-range values and valid parse replay.
- The deterministic codec corpus contains 714 finite values, including both
  signed zeros, subnormals, minimum normal, maximum finite and mantissas sampled
  over eleven exponent buckets. It is not exhaustive binary64 or fuzz coverage.
- Two public Solver cases serialize and read back actual outputs, rather than
  fabricated result fixtures. Planar requests cover 96/100 elements with 6, 20
  and all 192/200 modes. Spatial requests cover three repeated bending pairs
  at 66/100 elements, not a full 400-mode spatial spectrum.
- Reconstructed physical `K - lambda M` residuals are checked after decoding
  every returned mode, against the original `1e-8` gate. Planar checks also retain
  the independent discrete roots, unit shapes and mass orthogonality.
- Two headless cases encode/decode input models, execute official Rust plans
  through Engine, then encode/read the result and compare every numeric field's
  bits. They cover 96/100-element planar twenty-mode and 100-element spatial
  six-mode requests plus unresolved-spectrum error and valid replay.

The unresolved 128-element six-mode request still returns an internal refinement
error within its existing four-step budget. This is a retained fail-closed
boundary, not successful 128-element numerical coverage. Shape arrays, frequencies
and physical residual acceptance are not replaced by a structural JSON check.

## Verification

| Check | Result |
| --- | --- |
| Independent Protocol suite | 115 passed, 0 failed; 108 unit and 7 integration cases |
| Mass-coordinate / published Solver target | 22 passed, 0 failed |
| Rust headless modal target | 35 passed, 0 failed |
| Independent Headless and Operator SDK suites | 329 passed, 0 failed |
| Focused Protocol/Solver/CLI Clippy | Passed, warnings denied |
| Full Solver suite | 1275 passed, 0 failed, 9 existing ignored; 186 result groups |
| Executed modal profile | 25 commands, 225 passed, 0 failed; artifact `ok=true` |
| TaskIR examples and self-test | 5 examples validated; self-test passed |
| Documentation, tensor, organization and formatting gates | Passed |

Overlapping runs must not be added as distinct coverage. The standalone external
Rust SDK already explicitly enabled accurate float parsing and is not changed.
The nine ignored Solver cases are existing opt-in microbenchmarks and a retained
physical comparison; no ignored case was added. The compact local machine-readable
artifact is `tmp/modal-json-roundtrip-validation.json`, intentionally ignored.
Tensor structure passes with zero structural gaps, but global readiness remains
`blocked`: four maturity, sixteen evidence-grade and eleven P0 gaps remain.

## Scope Limits

This is a local Rust text/byte/reader and in-process headless boundary check.
It is not installed-Agent network transport, Python/Elixir parser parity,
source-detached research, deployment recovery, Linux execution, long-running
load, exhaustive input validation or industrial modal accuracy. No solver size
limit, convergence guarantee or release readiness is promoted.

The tensor retains two distinct scoped claims: the Protocol numeric contract,
and published Solver/headless result plus numerical-error replay. Their grades
remain `verified`, not `qualified`; codec fixtures must not qualify all physical
inputs or all required protocol dimensions.

Reproduce the standalone boundary with the native Rust tools:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-protocol --test modal_json_round_trip_contract
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_mass_scaling_reliability json_round_trip
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator json_round_trip
```

Codec fixtures and encoded solve outputs stay in memory. Development-generated
test executables and incremental caches are reclaimed after verification with
the native `dev-disk` entrypoint; compact evidence is retained, not duplicate
large model files or whole target trees.

## Source Fingerprints

SHA-256 of the final numerical contract, new tests and evidence bindings:

```text
b4812449439bb0538a42ed91b4a9f32327f857faaf1603061d9325efe3c293ec  workers/rust/crates/protocol/Cargo.toml
8dbf92f72cf8d39be30ac615784bcbfa97d90f46d8ef4a9d09da3c133c88576e  workers/rust/crates/protocol/tests/modal_json_round_trip_contract.rs
a24651bdd69d28b9c5b2cdb16b3af5d18cec6571d13711627f95e6e5ef83a8d8  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/json_round_trip.rs
288c8dcbf0ce5247c3e98731665e15924fb7207a15edd54b8825b68eb6faeffe  workers/rust/crates/cli/tests/modal_spectrum_operator/json_round_trip.rs
a35a42288036db7ca50dd9347983b50de47590de2996217c8317afd1f7eef3f9  config/operator-validation-profiles/modal.json
78e39407a5bc76f24ccff795cd458d1284ed734dbe8ff13ef2024935734abc1f  config/architecture/module-function-coverage-evidence/runtime-frame-fields.json
```
