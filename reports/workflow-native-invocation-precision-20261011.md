# Native workflow invocation precision and recovery limits

Orchestra's bounded native diagnostic/scoring adapter now prevents different
exact input, configuration or context values from reusing an old receipt solely
because TaskIR v1 rounds their numbers alike. The guard adds a computed lossless
invocation digest to existing task identity, without changing the cross-language
v1 digest contract or the actual calculation inputs.

This is a source macOS follow-up to consumed-path recovery binding. The base
commit is `6d9e1094` (`daji 3.5.6`); package metadata remains `3.5.0`. No version
bump, commit, packaging or installed-app acceptance is included.

## Reproduced numeric alias

The Rust and Elixir v1 canonical encoders deliberately use fixed-decimal
float encoding with up to fifteen fractional digits. Different values such as
`1e-30` and `2e-30` both encode as `0.0`. Adjacent ordinary-magnitude floats can
also round alike. A digest over this encoding is not exact numeric identity.

The existing native recovery gate compared that TaskIR digest with the retained
successful dispatch. A controlled receipt test changed nested input or config
numbers while keeping the parent, task name, node, operator and source path
unchanged. All eleven supported native diagnostic/scoring operators adopted
the old successful output. The twenty-two numeric-alias cases failed against
the original adapter; the selection had 60 cases and 22 failures.

After the change, the same cases reject drift with
`native_workflow_recovery_blocked/input_changed`, before receipt probing or
dispatch. Each case covers five collision pairs independently: tiny and very
tiny positives, zero vs tiny, signed negative zero vs negative tiny, and adjacent
floats. The matrix covers 110 input/config drift attempts without mutating
original dispatch bytes. Existing ambiguity, parent/lease guards, missing
receipts and normal unchanged-result reuse remain in force.

## Adapter-owned exact binding

`WorkflowNativeInvocation.bind_context` hashes a sorted lossless JSON object
containing the actual input artifact, config and orchestration context, plus
the internal `kyuubiki.workflow-native-invocation/v1` domain tag. It stores the
lowercase SHA-256 string as `workflow_native_invocation_sha256` in the existing
context. That string participates in the unchanged native TaskIR v1 digest.

The computed binding omits its own context key from the hash input. Repeated
binding is therefore stable, and caller-provided binding values are overwritten
rather than trusted. Authority and consumed-path fields remain unchanged and
participate in the exact invocation hash. Nil config still uses the existing
empty-map normalization before binding. Payload/config numbers are not rounded
or rewritten; the digest represents parsed runtime JSON values, not original
file whitespace or alternate decimal spellings of the same decoded value.

One digest string is added to task context. Encoding uses transient memory
proportional to the invocation; there is no new retained payload copy,
checkpoint bucket, scheduler snapshot, backup or journal format. Existing
graph/RPC budgets, journal bounds and Agent receipt-cache policy remain in force.
No constant-memory, throughput or aggregate RSS qualification follows.

Changed invocation values, including signed zero and integer/float representation,
cannot silently adopt an old successful native receipt. Old receipts without
this exact binding also block before probe or redispatch, even if they carry a
consumed-path binding. No migration rewrites original evidence or authorizes a
rerun. Explicit fresh submission remains separate from recovery.

## Verification

Thirty new Elixir tests cover the 22 operator/input-config drift cases, old
unbound receipt rejection, six exact-binding helper cases and adapter forwarding
of original values. The helper checks nested arrays, small/adjacent floats,
signed zero, integer/float spelling, stable key order, JSON round trips, caller
override, idempotence and unchanged routing fields. Existing callback/recovery
tests still cover unchanged receipts and the original dispatch guards.

| Selection | Result |
| --- | --- |
| Expanded SQLite workflow/recovery/dispatch selection | 693 listed, 684 passed, 9 JSON-only skipped, 0 failed |
| Isolated JSON checkpoint/recovery/binding/API/adapter selection | 368 passed, 0 failed |
| Complete Rust Protocol library suite | 133 passed, 0 failed |
| Complete Rust Engine library suite | 651 listed, 650 passed, 1 prebuilt-plugin test ignored, 0 failed |
| Rust Protocol/Engine/CLI all-target Clippy | Passed with warnings denied |
| Elixir test-environment compilation | Passed with warnings treated as errors |
| Rust and changed Elixir file formatting | Passed |

Selections overlap and do not establish an additive coverage percentage. The
ignored plugin case requires a prebuilt template dynamic library. The
new Rust protocol test explicitly records the remaining generic v1 limitation:
different tiny inputs/configs can keep the same unaugmented digest and pass
digest verification. It is a limitation fixture, not a generic protocol fix or
an endorsement of numeric aliasing as a recovery identity.

## Actual Agent and restart tests

The new
`real_native_workflow_distinguishes_tiny_inputs_under_the_same_task_name` test
constructs two all-constrained, zero-source thermal quad models with uniform
temperatures `1e-30` and `2e-30`, and solves them through the local Rust Engine
to obtain valid distinct inputs. Their complete raw outputs canonicalize alike
under v1. The same synchronous HTTP graph then submits both through source
Orchestra and real Rust Agents without changing diagnostic/scoring task names.

Both runs preserve the raw values, pass the native diagnostic/scoring chain
and retain distinct task digests for each identical task name. The test passed
in 1.78 seconds and requires exactly four Agent executions: two diagnoses and
two scores. The source solves in this identity test are local Engine references,
not extra Agent executions. The numerical comparison uses the same Engine,
not an independent scientific oracle or calibrated thermal material model.

The existing solved-workflow restart test also passed in 84.14 seconds across
thermal, electrostatic, magnetostatic, thermo and Stokes-flow chains. Actual
BEAM loss after diagnosis preserves the successful typed-solve checkpoint and
original Agent receipt, with fifteen Agent executions and no repeated solve or
diagnosis. The branch/join restart test passed in 33.68 seconds across thermal
true and electrostatic false routes, retaining selected-source lineage in six
executions. Original successful dispatch evidence stays byte-stable and normal
completion prunes private checkpoints.

These three focused service tests total 25 Agent executions. They are source
macOS tests, not a fresh whole-service-suite run, installed-app acceptance,
remote Linux, Windows, mobile, 1M scale or benchmark qualification.

## Open generic protocol scope

Full lossless numeric identity across arbitrary TaskIR authors remains open.
The Agent verifies the existing v1 task digest, but does not independently
recompute the adapter's lossless binding. Generic SDK/package-authored TaskIR,
operator descriptors and executable programs therefore do not inherit this
workflow recovery guarantee. This is not authenticated provenance, a malicious
peer defense or a versioned generic TaskIR digest upgrade.

The tensor records verified contract/recovery evidence for the Orchestra adapter
only and a separate open `runtime-protocol/validation/contract` qualification
scope. Historical fixed-15 ingress qualification cannot satisfy the new named
cross-authoring obligation. Completion requires a versioned numeric identity
contract, independent Rust Agent recomputation, Rust/Elixir authoring parity,
actual transport and unchanged-invocation recovery evidence. The next protocol
revision must retain tiny, adjacent, signed-zero and input/config drift cases.

The current improvement is bounded receipt consistency, not independent numerical accuracy,
general durable per-node continuation, exactly-once execution or release qualification.

Topology, module/function matrix, extension standard, tensor and tensor self-test,
runtime API surface, five TaskIR example contracts and their gate self-test,
document inventory/book and version-line checks pass. Project organization
passes the 800-line source and 2,000-line document limits with no tracked debt.
The tensor reports zero structural gaps, four maturity gaps, twenty evidence-grade
gaps and fifteen P0 gaps; Daji qualification remains blocked. The added
evidence-grade/P0 gap records the newly explicit generic numeric-identity
obligation, not a regression of the adapter's proven recovery guard.
