# Mechanical Truss Output Reliability: 2026-09-27

## Scope And Findings

This round repairs native `solve.truss_2d` and `solve.truss_3d` assembly,
result recovery and summaries. Base revision is `6ab4c2dd`, daji 3.4.2, with
the preceding thermal-truss repair and this working-tree overlay. No Engine
protocol, public schema, GUI or installed runtime was changed.

Before the repair, **9 of 10** new public-solver tests failed. The axial
closed-form/connectivity-reversal test already passed. This is a count of
failing tests, not independent defects:

- Finite mechanical inputs could return successful output with nonfinite
  total energy, serialized as JSON `null`. The profiling entry point exposed
  the same invalid success, so checking only the public solve was insufficient.
- Squared norms erased tiny displacements or rejected finite long members.
  Energy-density/area multiplication could underflow before a compensating
  length was applied, losing a representable total.
- Origin/unit-seeded bounds allowed translated or short models to evade the
  existing displacement limit.
- Fully fixed supports could discard invalid scalar or accumulated stiffness
  before reduction/solution observed it.
- Raw solves lacked cancellation checks in assembly and result construction.

## Repair And Validation

Length and displacement norms now use `hypot`. Positive finite member
stiffness, the full assembled sparse matrix, node magnitudes, and recovered
strain/stress/force/energy fields are checked. Mechanical trusses reuse the
Solver-local balanced energy-volume accumulator, rejecting nonzero-density
energy underflow and member/total overflow rather than returning false success.
The bounds calculation streams actual coordinates, avoiding the temporary
point vector and implicit origin/unit extent.

Mechanical and thermal trusses now share `truss_numerics` for scalar/matrix
finite checks and the displacement-limit policy. This moves the previous
thermal helpers without changing their error context or numerical rules;
operator-specific assembly and recovery stay separate. Both retain the
existing Solver cancellation API. Mechanical recovery now observes entry and
terminal checkpoints plus bounded chunks, and preserves profiling labels,
owned-input behavior and the public result shape. Full-matrix validation adds
an O(nnz) read pass; there is no element cache or measured speedup claim.

The **10** new solver tests cover both dimensions: tensile/compressive axial
closed forms and external work, reversed connectivity, borrowed/owned parity,
tiny/large norms, balanced energy products, range-error replay, constrained
invalid stiffness, translation/scale checks, profiling error propagation,
entry/terminal result cancellation, and interruption after 64 assembly or
recovery items in a 130-node chain. The **3** new headless tests traverse Rust
execution planning, the registered bridge manifest and in-process Engine
dispatch for failure/replay, finite small scales and translated limit checks.

Focused mechanical and thermal Solver targets passed **10 + 14** tests.
The mechanical and thermal headless targets passed **3 + 3** tests. Strict
Solver/CLI Clippy passed with `-D warnings`. The full local Solver regression
passed **1041** tests across 175 test binaries/doc-test groups, with 0 failures
and 9 opt-in tests ignored. This includes the mechanical-convergence and
cohesive-interface host regressions, not just the new targets. Overlapping
runs are not a coverage percentage or additional distinct test count.

Both mechanical-truss validation profiles executed all six commands. Workspace
formatting, tensor self-test/structure, document book/inventory, and
source-800/document-2000 organization checks passed. The tensor retains
0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps,
with overall release status still blocked rather than promoted by this repair.

The remote SSH alias, mDNS hostname and IP discovery service timed out.
No remote source upload, service change or Linux execution completed in this
round; the preceding thermal-truss Linux run is not evidence for this overlay.

This is bounded mechanical-truss output validation, not general structural qualification.
Arithmetic-envelope fixtures use synthetic extremes, not plausible material
parameters. Linear elasticity, the member-length threshold above `1e-12`,
and the 25%-of-maximum-axis-extent displacement heuristic remain. That heuristic
is not a local strain bound or a general rotation-invariant stability test.
Arbitrary intermediate overflow/cancellation and underflow elsewhere, including
loss before energy density is formed, are not all solved. No nonlinear,
buckling, damaged-member, joint-eccentricity, dynamic, 1M-node, installed-Agent
or network-recovery qualification is added. There was no paired benchmark.
Old artifacts with invalid physical fields, lost energy or origin-dependent
acceptance must be recomputed before research reuse; none were rewritten.

## Reproduction

Run from the repository root using the pinned Rust 1.88.0 toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --test truss_output_reliability --test thermal_truss_output_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test truss_output_operator --test thermal_truss_output_operator
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -p kyuubiki-cli --test truss_output_operator --test thermal_truss_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile truss-2d-closed-form --execute --out tmp/truss-2d-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile truss-3d-closed-form --execute --out tmp/truss-3d-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```

The local runs use macOS ARM64 debug/test mode. When Linux connectivity returns,
repeat release Solver `--lib` and the mechanical/thermal truss, mechanical
convergence, and cohesive-interface host targets before claiming cross-platform
verification. The tensor entry is scoped `verified` numerical/recovery evidence,
not an upgrade to overall release qualification.
