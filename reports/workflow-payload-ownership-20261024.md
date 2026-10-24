# Rust Workflow Payload Ownership

Date: 2026-10-24

Source base: `70340d07`, retaining the uncommitted harmonic reliability follow-ups.
Package metadata and the installed desktop/runtime remain 3.7.1. No version
bump, commit, deployment or desktop rebuild is included. Operator, Solver,
Agent, TaskIR and public workflow result interfaces are unchanged.

## Reproduced Redundant Copies

The owned request's input value was cloned from its map, then cloned again into
each declared input-node output. Solve, transform, extract and export results
were also cloned once for every output port, even when there was only one port
and the original was immediately discarded. Last-transient-consumer transfer
already avoided a copy at consumption, but publication added one back at each
subsequent node.

Two before-repair public regressions fail: compact/ephemeral chains do not
preserve the original buffer, and the retained cached input is itself a copy
rather than the request's original value. These are deterministic ownership
checks, not numerical corruption or wall-clock speed measurements.

## Owned Publication

After the existing complete request preflight, each input node removes its
value from the owned input map. It still executes exactly once. A small shared
publication helper visits ports in declaration order, cloning for every port
except the last, which receives the original owned value. The same helper is
used for input and all four generated-result node kinds.

For k declared ports, publication now needs k-1 deep clones when k is positive,
and no clone for zero ports. Input admission additionally avoids cloning from
the request map. A sole-output node can therefore transfer its value directly.
This is not shared mutable storage: retained sibling values remain independent.
Cache-aware consumption and release are unchanged; only the last transient
consumer may take an artifact. Cached or still-shared sources are still cloned
where their independent retention requires it.

The change does not strip echoed research inputs, recompute solver provenance,
truncate a result, change any public payload schema, alter port/lineage order,
or change fail/skip policy. Existing generated-output validation still runs
before any publication, including a producer with zero output ports. Existing
rollback removes failed producer ports and lineage.

## Budget Contract Clarification

Initial inspection considered raw input-to-output/condition forwarding as a
possible missed output check. The retained shared publication contract and
Rust/Elixir control tests show that it is deliberate: already admitted raw
input keeps the input budget, while newly generated operator values use the
stricter generated-output budget. No change to this contract is made.

The new controls retain a 500,001-byte input through raw/condition forwarding,
while `transform.first_available` must reject the same value as a generated
output even with no result projection or declared output ports. The previous
65,536-element harmonic workflow output-budget denial also remains a guard;
this ownership change does not make that result budget-admissible.

## Regression Coverage

Eight new functions cover one private publication unit and seven public Engine
integrations:

- Zero, one, two and 32 ports preserve declared order and value equality. The
  final port keeps the original string, nested array and nested string buffers;
  sibling ports own distinct buffers. Pointer addresses are compared only as
  integers; no freed pointer is dereferenced and no unsafe code is introduced.
- Zero-, one- and 20-transform chains with 128 KiB text preserve the original
  buffer under output-only projection or fully ephemeral retention. This proves
  the tested transfer path, not zero-copy serialization or network transport.
- Cached inputs keep their original allocation. A shared source is copied for
  an earlier consumer and moved to the final transient consumer. Mutating one
  retained output does not change its sibling or the cached source.
- All/output-only/no-artifact projections, cached/ephemeral policies and reversed
  node/edge declarations preserve completed/failed/skipped nodes, run traces,
  lineage, branch decisions and retained output values.
- A rejected generated value publishes no producer port or lineage. Explicit
  skip preserves raw evidence with its correct ownership, and fresh healthy
  requests still execute. Zero-output producers remain subject to validation.
- Original raw-input forwarding and stricter generated-output limits stay
  distinct rather than being silently unified by this optimization.

No new dependency, matrix snapshot, global payload cache or additional full-size
buffer is added. Metadata, necessary independent retained copies, typed
conversion, serialization and final transport still allocate. Aggregate retained
memory and allocator high-water use remain separate concerns. No production
throughput, peak-RSS reduction or speedup percentage is claimed.

## Completed Checks

Pinned Rust 1.88.0 and locked offline dependencies reuse the macOS AArch64 cache:

| Completed scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 924 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 712 | 1 ignored |
| Selected harmonic, ownership and dynamic integrations | 123 | Not every integration target |
| Total | 1,759 | Not a full workspace or all-physics run |

The dedicated profile executes all eight new functions plus six existing output
publication/execution-plan controls in four commands. These 14 overlap the table
and are not an additional test count. Existing 1,024-node scheduler and solver
workflow controls pass as part of the Engine library; they do not establish a
new throughput result. Solver/Engine all-target Clippy with warnings denied,
formatting and diff checks pass. Earlier reports retain their own source scopes.

Native catalog inspection validates 106 profiles without executing them all.
Book, documentation inventory and organization checks pass, with source/config
within 800 lines and documents within 2,000. The tensor retains 13 modules and
11 paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps; Daji status remains blocked. This is local contract/recovery
evidence, not a release-readiness promotion. No new Linux, Windows, remote,
installed-payload, Elixir-runtime or durable-restart run is claimed.

This is bounded in-process Rust workflow payload-ownership and recovery regression evidence, not zero-copy transport, elimination of required retained copies, an aggregate memory bound, a remote benchmark, Elixir ownership qualification or durable restart.
