# Electric Conduction Reference Recovery: 2026-09-20

## Scope And Reproductions

This change strengthens the existing steady, linear, isotropic split-quad
conduction operator. It follows the shared scalar-plane kernel work at base
revision `3007110a`; it changes neither the request/result schema nor the
physical model. Tests ran locally on macOS ARM64, not on a remote Agent.

The original conduction solve and power recovery failed three common-potential
regressions when prescribed and external voltages were shifted by `+/-2^40 V`:

- A contact model reported current-balance relative error
  `0.0005490483162518302` instead of zero.
- An impedance-only model reported current-balance relative error
  `0.0001830831197363603` instead of zero.
- A current-driven conductor returned peak current density `1.0001220703125`
  rather than `1 A/m^2`.

An additional fixed-electrode case reproduced a separate cancellation: a
`2^60 A` nodal source balanced by its electrode reaction erased the `1.5 A`
conductive injection at that node. Direct voltage-difference recovery retains
the injection and closes the `3 W` network power balance.

## Fix And Numerical Contract

Assembly, prescribed reduction, and postprocessing use a common relative
potential. Terminal external voltages shift before conductance multiplication
because terminal diagonal terms do not satisfy `K * 1 = 0`. Field, contact,
terminal-current, and Joule recovery retain the relative solution; absolute
node potentials and element-average potentials restore the reference.

Bulk/contact nodal currents come from off-diagonal voltage differences. A
fixed electrode's net injection uses that recovered current, not a sum of a
potentially large source and its opposing reaction. Domain work is accumulated
in the relative reference, then total source work is regrouped as
`domain_input + sum((V_external - V_node) * I_terminal)`.

This does not set source power equal to reported Joule loss or force residuals
to zero. Bulk field integration and contact losses still supply an independent
power check; current-balance and free-node residual diagnostics remain active.
Individual terminal source/delivered-power fields keep their absolute-reference
meaning and can change under a common shift even when network totals do not.

The input preflight additionally rejects nonrepresentable relative voltage
ranges and nonfinite reciprocal interface resistances, including the
all-nodes-fixed path. This is not a guarantee that every extreme representable
input has a representable power output. Absolute `f64` inputs that already lost
their voltage differences cannot be repaired by shifting them later. Similarly,
absolute output potentials may round away tiny differences which remain intact
in the internally recovered fields and powers.

## Validation

All 11 new solver integration tests passed. They cover positive/negative
reference shifts, constant skew patches, imposed currents, contacts,
terminal-only anchoring, mixed boundaries, reversed currents, owned/borrowed
requests, source/reaction cancellation, invalid ranges, and a clean solve after
rejection. Invariant comparisons use `1e-10 + 1e-8 * abs(expected)`; restored
absolute potentials allow `0.0005 V` for rounding at `2^40 V`. Individual
absolute terminal powers allow `0.001 W` after the expected reference shift.

The sparse test uses a `34 x 34` mesh: 1,225 nodes and 1,190 free DOFs, above
the dense threshold. Jacobi, symmetric Gauss-Seidel, and incomplete Cholesky
each run at zero and both shifted references. The `1 A`, `sigma = 3 S/m`
unit-square solution is checked against `E_x = -1/3 V/m` and `P = 1/3 W`;
source-power balance and free-current residual must both stay below `1e-8`.

Both Engine integration tests passed. The new chain solves a `2 A` copper
conductor through direct and workflow entrypoints, transfers its `6.72e-5 W`
Joule power into explicit nodal heat loads, and solves the thermal operator.
It checks power to `1e-15 W` and temperature/heat flux to `1e-12` absolute
tolerance at all three references. This is explicit conservative nodal-load
transfer, not a volume-source projector or temperature-dependent feedback loop.

The broader debug run passed 281 solver unit tests and 45 integration tests
across 15 targets, including the adjacent heat/electrostatic/magnetostatic
reviews and orientation/input/refinement regressions. Six opt-in unit tests
were ignored by the ordinary run. Release repetition passed all 281 units,
the 11 conduction and 12 scalar-plane reference tests, and both Engine tests.

The native validation runner executed all seven conduction profile commands
successfully, including the actual Agent RPC handler test and 13 headless
route/catalog/dispatch contract tests. Those in-process checks do not constitute
a deployed network or remote-service test. These overlapping runs are not
added together as a unique-test coverage percentage.

Validation-profile structure, tensor structure, project organization and
documentation inventory checks passed. Source and document limits remain
800/2,000 lines with no tracked debt. The tensor's overall daji status remains
blocked (four maturity, 16 evidence-grade and 11 P0 gaps); this evidence does
not erase those obligations. Strict Clippy still encounters the pre-existing
`type_complexity` warning at `linear_ic0.rs:146`. With only that lint exempted,
the solver library and new integration target pass with other warnings denied.
Formatting checks and `git diff --check` also pass.

This is bounded component evidence, not release qualification, a million-node
benchmark, an end-to-end performance claim, or general material validation.
No GUI rebuild, installed-app test, Windows test or remote run is claimed.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test electric_conduction_reference_invariance
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --test electric_conduction_operator
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --release --lib --test scalar_plane_reference_invariance --test electric_conduction_reference_invariance
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --release --test electric_conduction_operator
./scripts/kyuubiki check-operator-validation --execute --profile electric-conduction-plane-quad-screening --out tmp/electric-conduction-reference-20260920.json
```

The component profile includes the regression alongside its analytic,
refinement, malformed-input, Agent RPC, Engine and headless discovery checks.
The coverage tensor records only this bounded numerical/workflow evidence;
broader qualification and cross-platform obligations remain unchanged.
