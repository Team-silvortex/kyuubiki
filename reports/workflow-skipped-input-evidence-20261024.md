# Skipped Workflow Input Evidence

Date: 2026-10-24. Source base: `305e3009` (`daji 3.7.3` commit subject).
Package metadata remains 3.7.1. This round does not deploy, rebuild the desktop
applications or rewrite historical stored results.

## Reproduced Defect

A strict multi-input node cannot execute when one predecessor has resolved
without publishing the required port. Both Rust and Orchestra correctly skip
the node, but previously record any other available source as consumed. The
same defect affects multi-port output nodes. The record claims use of an input
that was never passed to an executor or copied to an output.

The shared Rust regression fails before repair with `strict: ["a.payload"]`.
The Orchestra runner and asynchronous API/SQLite readback tests fail on the
same non-empty consumed-input list. The active-branch failure control passes:
an attempted strict operator really receives both inputs and must retain them
in its failed-node record.

## Narrow Repair

Skipped traces now contain empty consumed and produced lists. No artifact
lineage is created for them. Rust still retires every incoming dependency edge,
so transient inputs are released after their last possible consumer, not
retained forever or removed before a live fallback uses them. Orchestra's
skipped-record helper no longer needs an input-resolution callback.

This changes new execution receipts only. Scheduling, graph/protocol schemas,
operator implementations, condition decisions, error policies and actual
payload values are unchanged. Failed attempts keep their existing input
records. No extra snapshots, dependencies or backup files are introduced.

## Shared Regression Coverage

`tests/fixtures/workflow-skipped-input-evidence.json` is a nine-node graph with
a strict named-input transform, multi-port output, preferred/backup fallback,
raw output and dependent output. Both runtimes use the same request fixture.

- An inactive condition, a failed condition and a missing input exercise
  separate ways for a predecessor to resolve without the required artifact.
- False, zero and null payloads remain present inputs; reversed node declarations
  must not change the contract. Callback sentinels reject execution of a skipped
  strict transform in Orchestra.
- Rust exercises cached/ephemeral policies and all/output-only/no-artifact
  projections. Required final values remain available and transient inputs do
  not leak into projected results after skipped edges retire.
- An active branch controls for over-broad suppression: the attempted strict
  transform fails with both real sources recorded, the multi-port output runs,
  and fallback selection correctly uses the preferred condition artifact.
- Local asynchronous API tests read results back from disposable SQLite storage
  in full, payload-omitted and compact response modes. These use Plug routing,
  not an installed TCP server or a remote Agent.

## Validation Entry

`make test-workflow-execution-evidence` now runs nine Elixir and five Rust test
functions, including five added in this round. The existing component profile
`workflow-execution-evidence-local-reliability` also runs six selected-input and
five output-retention controls. These counts overlap suite runs.

Engine library and six selected public integration targets pass locally:
712 library tests plus 27 integration tests, zero failures, one pre-existing
ignored library test. Engine all-target Clippy with warnings denied and
Rust/Elixir formatting checks pass. The shared entry passes all 14 tests.
The full web suite passes 3,478 tests with zero failures and 77 pre-existing
skips. The component profile executes all 25 overlapping tests successfully.
These checks use macOS AArch64, Rust 1.88.0 with locked offline dependencies,
and Elixir 1.19.5/OTP 28; they are not a new Linux or remote CI campaign.

Documentation book/inventory, project organization and diff checks pass. The
coverage tensor retains 13 modules, 11 paradigms, zero structural gaps, two
maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji remains blocked.

This is local execution-evidence and resource-retirement consistency, not a
new solver accuracy result, authenticated provenance, process-restart or
power-loss validation, distributed recovery or installed acceptance. The
existing tensor coordinate is extended without raising its qualification grade.
