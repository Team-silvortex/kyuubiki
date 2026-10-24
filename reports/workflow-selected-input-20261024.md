# Rust Workflow Selected Input Attribution

Date: 2026-10-24

Source base: `091b1393` (`daji 3.7.3` Git checkpoint). Workspace package metadata
still reads 3.7.1. This round does not reconcile versions, commit, deploy or
rebuild the installed desktop. The operator/Engine boundary, result structures,
TaskIR and existing numerical and cancellation controls are unchanged.

## Reproduced Contract Gaps

The existing input-binding contract says `transform.first_available` waits for
all predecessors and chooses the first available source in edge declaration
order. Its trace and output lineage must name that selected source, not unused
alternatives. The Rust resolver selected the correct value, but the scheduler
recorded every available incoming artifact as consumed. With both inputs
present, a correct payload therefore carried an inaccurate contributor list.

A second issue applied partial-input readiness based on the optional operator
name without checking the node kind. An output node carrying a
`transform.first_available` annotation could run with only one of two inputs
available, then fail while reading the missing source. Without the annotation
the same output correctly waited for all inputs and was skipped. Operator
metadata must not silently give an output node transform behavior.

Both new public reproductions fail before repair. The first returns
`[primary.payload, backup.payload]` instead of the actual selected contributor.
The second attempts output publication after a recoverable backup-input failure
and reports a missing backup instead of skipping the blocked output.

The initial shared-retention audit did not reproduce premature removal. This
repair does not relabel source-attribution errors as lost numerical data.

## Narrow Scheduler Repair

Partial-input handling now requires a Transform node as well as its supported
partial-input operator. After readiness resolution, a fallback retains only the
first available key for its consumed-artifact trace and output lineage. The
payload resolver follows the same incoming-edge order in the same synchronous
runner, with no intervening artifact mutation.

Scheduling still waits for every predecessor, not the first one to finish.
Retention still retires every incoming edge, including an unused alternative
or a duplicate source alias. It does not use the shortened contributor list to
decrement reference counts. Other consumers can therefore still read shared
sources; transient sources are released after their actual final planned use.

Named pair/aggregate operators remain separate. They still assemble every
target-port binding and may legitimately record the same source twice when it
fills two actual inputs. This is not global source deduplication. Nested joins
record the selected direct upstream edge, not a flattened guessed ancestor.

JSON budgets, raw-input forwarding rules, fail/skip policy, caller-owned
cancellation, cache defaults, payload values and result schemas are unchanged.
An invalid selected source is not silently replaced by a different present
source after a budget failure; explicit graph logic must decide that recovery.
No second payload copy, snapshot, dependency map or scheduling framework is added.

## New Regression Controls

Six new public Engine functions cover:

- Selected trace and lineage under all/output-only/no-artifact projection,
  reversed node declarations and reversed edge priority. Payloads and raw side
  outputs agree with the declared choice, after both predecessors complete.
- Equal objects, null, false and zero remain present artifacts with distinct
  source identity. Equal values do not erase a changed selected edge.
- Cached/ephemeral policies, ordinary and duplicate-alias join inputs, and later
  raw consumers retain their data. Original string-buffer transfer is checked
  for transient final consumers, with no leaked extra intermediate artifacts.
- A selected over-budget value fails the join and produces no join output or
  lineage under skip policy; its trace names only the attempted selected source.
  Raw/independent evidence survives. A safe selected value succeeds even when
  its unused raw alternative exceeds the generated-output string limit.
- Two nested joins retain each selected direct source without attributing the
  downstream result to an unused backup or flattening intermediate lineage.
- Output annotations do not change readiness: both complete inputs retain
  both port bindings and their separate lineage; a missing input skips the
  output. An annotated output with no incoming edges behaves like its ordinary
  no-input counterpart instead of acquiring partial-transform semantics.

The existing compiled-plan unit now also asserts the selected consumed key and
lineage, not only the correct returned value. Existing shared named-input and
branch-recovery targets protect multi-input duplicate bindings and failed-
primary/backup selection. Those shared fixtures are exercised in Rust here,
not rerun through Elixir or a deployed Orchestra service.

## Scope

This makes the live Rust behavior agree with the previously documented binding
contract. The older binding report retains its original run scope; this report
does not turn that historical report into new Rust lineage evidence.
Previously retained lineage is not silently rewritten or newly authenticated;
the corrected attribution applies to fresh executions.

The result is local execution attribution, not a cryptographically authenticated
provenance system or a new recovery checkpoint. Cross-runtime durable receipt
reuse, Agent/network interruption, installed GUI, numerical accuracy and scale
acceptance require their own evidence. No such qualification is claimed here.

## Completed Checks

Pinned Rust 1.88.0 and locked offline dependencies reuse the macOS AArch64 cache:

| Completed scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 926 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 712 | 1 ignored |
| Selected workflow, harmonic, domain-integrity and dynamic integrations | 168 | Not every integration target |
| Total | 1,806 | Not a full workspace or all-physics run |

The new profile executes six new selected-input integrations, six existing
shared named-input tests, three existing branch-recovery tests and three
compiled-plan units. These 18 overlap the table and are not additional counts.
The existing scheduler unit was strengthened without adding a new function.
All-target Solver/Engine Clippy with warnings denied passes. Earlier source
reports keep their own scopes; counts are not additive across reports.

Native catalog inspection validates 108 profiles without executing them all.
Book, documentation inventory, formatting and diff checks pass. The organization
gate enforces 800-line source/config and 2,000-line document limits; the current
version-page introduction is compacted rather than adding an oversize exception.
The tensor check passes with 13 modules and 11 paradigms, zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Daji remains blocked;
this is bounded contract/recovery evidence, not a readiness promotion.

This is bounded Rust workflow selected-input attribution and branch-retention regression evidence, not authenticated data provenance, numerical qualification, cross-runtime execution, distributed recovery, durable restart or a scale benchmark.
