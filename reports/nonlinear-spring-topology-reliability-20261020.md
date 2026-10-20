# Nonlinear Spring and Gap Contact Restraint Validation

Date: 2026-10-20

## Reproduced Defect

The nonlinear spring and gap-contact solvers previously checked only that the
whole model contained a fixed node. A disconnected component or unused free
node could therefore retain rigid-body translation. If its load was zero or
below the requested residual tolerance, the convergence shortcut bypassed the
linear solve and published `converged: true`, `achieved_load_factor: 1.0`.

The initial regression suite reproduced five failures out of seven tests:
floating components, an unused free node, a floating cycle with an unrelated
fixed node, inactive/just-touching contacts, and a small-load false success.
The two supported-model analytical controls already passed before the repair.

## Repair and Scope

`nonlinear_spring_1d_validation.rs` now checks every spring-connected component
for a fixed node before assembly. Union by rank and path compression avoid
building an adjacency list or dense matrix. Auxiliary storage is O(nodes);
traversal is near-linear in nodes plus elements. The error identifies a node
in the unrestrained component rather than returning a misleading success.

Both borrowed and owned entry points use the same check. Contact validation
borrows the node and element slices instead of cloning a temporary spring
request. Input and topology passes check cancellation in bounded batches.

This is a prerequisite for the existing zero-displacement Newton load path:
with the supported non-negative gaps, contacts start inactive, including the
just-touching case. They do not supply bilateral initial restraints. An
initially floating body that becomes supported only after moving into contact
still needs a different active-set/initialization treatment; this repair does
not implement that problem class.

Independently supported disconnected components and unused fixed nodes remain
valid. No artificial support, stiffness regularization, tolerance relaxation,
load-step change or engine-specific exception was introduced. The graph
check is not a general tangent-conditioning, softening-stability or nonlinear
convergence certificate.

## Verification

Both platforms used Rust 1.88.0 with the existing offline build cache:
macOS AArch64 locally and Linux x86_64 in the lab's isolated CI source directory.
No installed service was changed and no desktop package was rebuilt.

| Test group | macOS | Linux |
| --- | ---: | ---: |
| Spring/contact solver regressions, including topology | 47 passed | 47 passed |
| Mechanical convergence cross-checks | 22 passed | 22 passed |
| Spring/contact engine workflow regressions | 6 passed | 6 passed |
| Rust headless plan-to-engine regression suite | 9 passed | 9 passed |
| Total distinct tests per platform | 84 passed | 84 passed |

Clippy with `-D warnings` passed for the solver library and the changed solver,
engine and CLI test targets. Workspace formatting and diff-whitespace checks
passed. The validation-profile structural check passed for 59 profiles; it did
not itself execute all profile commands.

The topology suite includes all 63 nonempty simple four-node edge sets with
all 15 nonempty support sets: 945 configurations. An independent support
reachability oracle checks all four public spring/contact entry points.
For every supported configuration, prescribed displacements generate loads
using an independent cubic force expression. Both the nonlinear spring and
gap-contact solvers must recover those displacements and the contact force,
not merely pass a zero-load topology check.

Other controls cover node permutations, reversed/duplicated edges, separate
supported components, active and inactive contacts, malformed references,
cancellation before assembly, and clean replay. Existing analytical,
refinement, numerical-range and last-converged-state tests were kept unchanged.
The headless tests use the Rust plan builder and route manifest to call the
real engine operator; they are not a remote Agent transport certification.

Reproduce the changed-path suites from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test nonlinear_spring_topology --test nonlinear_spring_recovery --test nonlinear_spring_range_reliability --test nonlinear_spring_reliability --test nonlinear_spring_1d_closed_form --test nonlinear_spring_1d_review --test contact_gap_reliability --test contact_gap_1d_closed_form --test contact_gap_1d_refinement --test contact_gap_1d_review --test mechanical_convergence
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --test nonlinear_spring_topology_workflow --test nonlinear_spring_recovery_workflow --test nonlinear_spring_workflow --test contact_gap_workflow
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test nonlinear_spring_range_operator
```

The existing `nonlinear-spring-range-local-reliability` validation profile and
tensor evidence entry include the new regression paths. No new qualification
grade, release readiness claim, coverage percentage or 1M-node benchmark claim
is inferred from these bounded tests. Full workspace, GUI, Windows and
large-scale performance suites were not rerun for this repair.

The tensor structural/command check also exposed an older SQLite isolation
evidence anchor still requiring `ExUnit.after_suite`. The existing cleanup uses
`System.at_exit` so repeated suites retain their database until process exit.
Only that evidence entry was aligned with the existing implementation and
previously verified lifecycle tests; no BEAM implementation changed here.
After alignment, tensor structure/command checks pass. Overall readiness
remains blocked (2 maturity gaps, 23 evidence-grade gaps, 18 P0 gaps), not a
release approval.
