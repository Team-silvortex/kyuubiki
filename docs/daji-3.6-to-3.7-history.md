# Daji 3.6 To 3.7 Checkpoint History

Historical checkpoint descriptions with their original versions, results and
limitations. They are not the current installation or a fresh qualification run.
Use [current-line.md](current-line.md) for active status and next work.

## Daji 3.7.1 Local Packaging

October 24, 2026. Package metadata is reconciled with the existing 3.7.1 Git
checkpoint. This working-tree build includes compact desktop information cards
and the retained harmonic reliability changes. Local build and installation
results are recorded in [the snapshot](../releases/snapshots/3.7.1.json): all three
installed shells boot, the current runtime is healthy, and the compact Workbench
layout and logs are observed in the installed WebView. See [the local rebuild
report](../reports/daji-3.7.1-local-rebuild-20261024.md) for scope; historical
reports keep their original versions and qualification scopes.

### Harmonic Assembly Source Follow-up

The [assembly follow-up](../reports/harmonic-assembly-20261024.md) retains lost
addition tails in represented dynamic coefficients before elimination. A
cancelling oscillator previously returned amplitude 0.5 instead of about
0.4472136, or falsely reported singularity without damping. Dense/path, Engine
and workflow regressions now cover the corrected response, force and failure
boundaries. This source change does not replace the installed 3.7.1 payload,
alter the operator protocol or promote physical/platform qualification.

### Harmonic Inertia Source Follow-up

The [inertia follow-up](../reports/harmonic-inertia-precision-20261024.md) carries
admitted frequency-square/mass product tails into assembly and original-equation
checks. An independent unit-response oscillator previously returned about 0.625.
Scaled intermediate tails now avoid breaking the existing tiny common-scale
control; final range guards remain. This is source-only, not a new installed,
all-physics or platform qualification.

### Harmonic Residual Source Follow-up

The [residual follow-up](../reports/harmonic-residual-enclosure-20261024.md) uses
outward bounds when original-equation scaling loses tiny terms. Only a certified
backward-error upper bound admits those rows; ordinary rows and pivot thresholds
stay unchanged. This is source-only evidence, not a new installed qualification.
The [many-term follow-up](../reports/harmonic-residual-many-term-20261024.md)
prevents interval growth from falsely rejecting parallel-element models while
preserving mixed-row checks, cancellation and workflow output security budgets.

### Workflow Source Follow-ups

[Payload ownership](../reports/workflow-payload-ownership-20261024.md) removes redundant copies; [cancellation boundaries](../reports/workflow-cancellation-boundaries-20261024.md) stop cancelled inner graphs despite skip policy.
[Selected-input attribution](../reports/workflow-selected-input-20261024.md), [output retention](../reports/workflow-output-retention-20261024.md) and [execution receipts](../reports/workflow-execution-evidence-20261024.md) protect selected sources, requested results and Rust/Orchestra trace consistency without claiming durable checkpoints.

## Daji 3.7.0 Checkpoint

October 23, 2026. Source packages, SDK metadata, language-pack targets,
installation/update contracts and current documentation advance to 3.7.0.
The release snapshot records base commit `0a65f2e8` plus this source overlay;
it is not a clean-commit desktop binary or an installed qualification run.

- Selected scalar and mechanical solves now validate the complete original
  assembled system before constraint elimination. Dimensions, finite values,
  exact fixed values and free-equation equilibrium are checked together.
  Nonzero support reactions remain legitimate and are checked separately.
- Projected thermal-frame constraints preserve their physical support basis.
  Newmark steps and Newton increments check the original equations before
  history publication or line-search acceptance; these are not new constitutive
  models or blanket qualification of every solver family.
- Harmonic solves validate the complete complex state and constraint mapping.
  Binary-scaled frequency products, checked complex elimination and complete-row
  back substitution recover finite results from exceptional intermediates.
  True overflow or erasure fails instead of producing a plausible zero.
- Displacement, element extension and element-force ranges are independently
  checked before publication. Bounded cancellation and explicit same-input replay
  regressions do not imply durable process recovery or automatic retry.
- Numerical algorithms remain in Solver; Engine/Agent retain execution and
  publication ownership. Repository cleanup removes only disposable build
  outputs, not regression source, credentials or current installed components.

Read the retained reports in order: [original prescribed equations](../reports/original-prescribed-equation-admission-20261023.md),
[mechanical zero constraints](../reports/homogeneous-mechanical-equation-admission-20261023.md),
[projected thermal frames](../reports/projected-thermal-frame-equation-admission-20261023.md),
[Newmark](../reports/newmark-original-equation-admission-20261023.md),
[Newton](../reports/newton-original-equation-admission-20261023.md),
[harmonic admission](../reports/harmonic-original-equation-admission-20261023.md),
[frequency products](../reports/harmonic-frequency-product-range-20261023.md) and
[complex arithmetic/force recovery](../reports/harmonic-complex-arithmetic-range-20261023.md).
Their recorded 3.6.x working-tree provenance and exclusions remain unchanged.
The final complex-arithmetic report retains 1,540 passing checks on each of
macOS AArch64 and Linux x86_64, with ten ignored and 292 modal cases excluded
per platform. These overlapping suites must not be summed across reports or
promoted into full physical, Windows, installed or scale qualification.

After version alignment, the macOS source overlay passes 1,598 Rust checks:
786 Solver library, 711 Engine library, 48 Solver integration and 53 Engine
integration tests. Ten tests are ignored and 292 modal cases remain excluded.
The frontend product-identity check also passes. This release rerun is not a
fresh Linux or installed-platform run; the snapshot lists its exact targets.

The [October 24 complex RHS follow-up](../reports/harmonic-complex-rhs-range-20261024.md)
recovers finite harmonic solutions when row normalization or forward elimination
temporarily puts a right-hand side outside binary64 range. Independent scaled
components survive pivot swaps and solution division. Ordinary storage retains
its allocation; matrix scaling, pivot floors and original-equation gates stay
unchanged. Signed/oriented force readback, late output failure, cancellation and
explicit clean replay have scoped tests. This follow-up is not a new release
snapshot, full complex-solver qualification or durable recovery evidence.

The [RHS promotion follow-up](../reports/harmonic-rhs-promotion-20261024.md)
keeps the finite source intact until the complete wide buffer passes conversion
checkpoints. A cancelled promotion leaves the same object's rows and allocation
unchanged, permitting an explicit retry with fresh control. This is local
conversion failure atomicity, not whole-solver rollback or persisted recovery.

The [coefficient-normalization follow-up](../reports/harmonic-coefficient-normalization-range-20261024.md)
rejects erased or partially rounded tiny matrix components before elimination,
with explicit prepared-row/column/component diagnostics. Public extreme-contrast
fixtures were already blocked by original-equation admission; the guard moves
that rejection earlier and prevents the internal factorization from returning a
candidate for a silently modified matrix. Exact tiny couplings, nonzero quantum
responses, ordinary bits and explicit repaired runs retain bounded controls.
This is conservative admission, not a wider-range factorization or new physics.

The [complex zero-cancellation follow-up](../reports/harmonic-complex-zero-cancellation-20261024.md)
also replays complete expressions when nonzero normal terms round to zero.
Signed dyadic controls retain tiny product/determinant tails through complex
products, elimination and back substitution. Exact zeros remain admissible and
ordinary non-cancelling arithmetic keeps its order. These are internal arithmetic
and manufactured-system counterexamples plus existing public/workflow regressions,
not a newly qualified physical model or general forward-accuracy proof.

The [nonzero near-cancellation follow-up](../reports/harmonic-complex-near-cancellation-20261024.md)
extends this routing to strongly reduced nonzero components. Independent
real/imaginary row peaks also detect gradual shrinkage that exposes earlier
lost terms. The relative threshold selects compensated replay, not physical
acceptance or singularity tolerance. Signed dyadic references, both internal
factorizations, threshold controls and existing public/workflow regressions
bound the evidence; general forward accuracy is still not certified.

The [nested accumulation follow-up](../reports/harmonic-complex-exact-accumulation-20261024.md)
fixes loss inside replay's own floating-point error balance. Already aligned
parts now use checked fixed integer storage and one final binary64 rounding.
Independent integer references, nested FMA tails, subnormal boundaries and
cancellation controls bound this change. Ordinary routing, upstream alignment,
matrix preparation and physical admission remain unchanged; assembly and final
division are not thereby exact.

The [binary-normalization follow-up](../reports/harmonic-binary-normalization-20261024.md)
addresses upstream rounding that could flip a tiny component's sign in a
well-conditioned manufactured system. Rows prefer exact power-of-two shifts,
with legacy scaling retained when needed for reversible tiny couplings.
Explicit row weights keep pivot comparisons in original max-normalized units
through swaps. This changes preparation arithmetic, not the pivot-floor
constant, original-equation gate or physical model.

The [quotient-rounding follow-up](../reports/harmonic-quotient-rounding-20261024.md)
fixes double rounding near the subnormal lattice and avoids unnecessary squaring
for exceptional purely real/imaginary divisors. A public static spring fixture
previously published a response one minimum quantum above the independent
reference; Solver and serialized Engine/workflow readback now retain that
reference bit pattern. Range admission stays unchanged, and general complex
division or whole-model forward accuracy is not thereby certified.

The [general-division follow-up](../reports/harmonic-general-division-20261024.md)
retains exceptional complex numerator products and the positive squared
denominator as bounded integers until final ratio rounding. This avoids false
range errors from extreme component contrast and a public damped-response
quantum error. Ordinary fast-path arithmetic and upstream assembly remain
unchanged; local exact-ratio controls do not certify whole-model forward error.

The [ordinary-division follow-up](../reports/harmonic-fast-division-20261024.md)
replaces legacy weighted reciprocal rounding with a compact integer-pair
candidate and exact midpoint admission. Normal candidates must fit the correct
rounding cell, including ties; unsupported or ambiguous cases use exact replay.
This deliberately changes some ordinary division bits, not product/update bits,
pivot thresholds or original-equation acceptance. Upstream rounding remains separate.

The [row-update follow-up](../reports/harmonic-row-update-20261024.md) addresses
product tails lost before final division in RHS elimination and back substitution.
Fused scalar updates retain selected weak numerators; an upward-padded local
rounding budget routes amplified partial-sum loss into complete-expression replay.
A two-spring public closed form now retains its weak tip response. Matrix updates
and numerical gates are unchanged; the unrounded full expression is not certified.

The [Schur-update follow-up](../reports/harmonic-schur-update-20261024.md) applies
the shared fused/replay policy to dense coefficients, path diagonals and fill-in.
A three-spring soft mode isolates coefficient-tail loss with an unchanged RHS;
independent rational readback now meets the selected component accuracy checks.
Coefficient update bits may change, but weighted pivot gates and legacy standalone
product/difference arithmetic do not. Factorization and assembly remain rounded.

The [factor-compensation follow-up](../reports/harmonic-factor-compensation-20261024.md)
adds one transient residual correction to eligible rounded elimination factors.
The same two-term factor updates coefficients and RHS, including pivot-generated
fill-in, without a full factor snapshot. Shared step preflight checks read-only
headroom; unsupported coefficients or RHS disable correction for the entire step.
This improves selected non-dyadic-factor examples, not exact global factorization.

The [replayed-row quotient follow-up](../reports/harmonic-row-quotient-20261024.md)
retains admitted integer numerator sums until final complex quotient rounding.
This removes a reproduced second rounding in cancellation-sensitive substitution,
including axial and general complex divisors. Ordinary fused rows retain their
previous fast route; alignment, pivot and physical-admission gates are unchanged.
The local expression guarantee does not remove upstream factor or assembly error.

The active tensor profile becomes `daji-3-7-hardening`, without promoting
evidence grades or changing thresholds. Release qualification remains blocked;
the last recorded review has two maturity, 23 evidence-grade and 18 P0 gaps.
See [the source snapshot](../releases/snapshots/3.7.0.json) for the checks rerun
after version alignment. No desktop/runtime rebuild, installation, registry
publication or public distribution is part of this source release.

## Daji 3.6.0 Checkpoint

October 12, 2026. Commit `a1740601` records the 3.6.0 development checkpoint;
the version-alignment overlay brings source packages, language-pack targets,
installation/update contracts and current documentation forward from 3.5.0.
This does not rebuild, install or publish binaries or SDK packages.

- Workflow recovery now fences dispatch admission, cancellation and ambiguous
  restart outcomes. Scoped receipt/output/branch reuse remains bound to the
  original invocation and current ownership; unknown work is not replayed.
- Collection, scoring, quality composition and next-round planning reject
  malformed or contradictory evidence rather than silently dropping failures.
  Incomplete and blocked candidates cannot become successful research stops.
- Rust and Elixir check canonical candidate order, declared ranks and matching
  row/objective scores. Corrected fresh runs are explicit; no sorting or replay
  silently repairs invalid evidence.

The retained [ranking consistency report](../reports/quality-ranking-consistency-recovery-20261012.md)
records 31 shared cases, permutation/JSON-exchange controls and 16 controlled
graphs per runtime. Its full Engine suite passed 690 tests with one ignored;
the service selection passed 2,078 with 71 conditional skips. These are prior
source runs at the report's recorded overlay, not new 3.6.0 installed, physical,
remote or durable-recovery qualification. Earlier follow-ups below retain their
original provenance and limitations.

The [October 16 tensor recalibration](../reports/tensor-recalibration-20261016.md)
records two maturity, 22 evidence-grade and 17 P0 gaps; 55/77 coordinates and
17/46 named scenarios meet their targets. New recovery obligations expose
unqualified native LAN authentication, installed direct/typed-workflow recovery,
host-power-loss durability and sustained multi-host Mesh load. Local desktop
tests now have correct verified mappings; no installed grade is promoted.
Release qualification remains blocked. Direct sweep-request, plan and expansion
quality-metadata admission now have scoped follow-ups below. Expansion counts and
direct execution-batch quality evidence also have bounded checks. Batch structure,
unique recovery identities and checkpoint input/policy admission now have a scoped
follow-up. Preparation counters/identities and persisted completion evidence now
have bounded revalidation too. Execution failure identity, recovery permissions and
outcome/counter partitions now have a scoped follow-up. Remaining audits include
cross-process attempt ownership, authenticated provenance, standalone TaskIR boundaries and producer-side
readiness preservation through ranking.
See `releases/snapshots/3.6.0.json` for the source checkpoint and scope.

The [native network-access follow-up](../reports/agent-network-access-20261016.md)
defaults every Agent control mode to loopback-only ingress. LAN deployments now
require explicit peer IP configuration before upgrading; a wildcard bind does
not opt in. This containment is not authentication and does not close the tensor's
native authorization, authenticated multi-host or installed-recovery obligations.
The same follow-up now covers deployment preflight: Agent, Installer environment
validation and managed runtime share one policy parser. Invalid local Agent
settings reject restart before stopping existing services; status and explicit
stop remain available. Runtime summaries distinguish configured intent from
observed running policy. Native lifecycle fixtures verify the control path, not
an installed GUI, remote deployment or full transactional restart guarantee.

The [Mesh submission identity follow-up](../reports/mesh-submission-identity-20261018.md)
closes a native direct/Mesh gateway acceptance gap: durable submission receipts
and explicit non-admission replies must match the requested method and exact
typed input digest, not just the execution ID and field shapes. Lookup-only
`not_found` cannot acknowledge a submission. Unknown replies retain the original
recovery handle without retry or migration. Matching historical receipts and
read-only inspection still work. Local TCP regressions and real Agent process
recovery tests pass; authenticated multi-host, installed WebView and host-power-loss
obligations remain open. Tensor grades are unchanged.

The [Mesh discovery contract follow-up](../reports/mesh-discovery-contract-20261018.md)
rejects incompatible roles, transport declarations and missing discovery methods
before updating peer health or merging hints. A native Agent regression reproduces
the old false-healthy/hint-acceptance behavior and verifies degradation, isolation
across forced restart, rediscovery after protocol repair and retention of the exact
committed execution receipt without replay. Busy/draining peers and differing
solver inventories remain discoverable. This is local protocol compatibility,
not authenticated membership, work admission or multi-host qualification.

The [live store binding follow-up](../reports/mesh-live-store-binding-20261018.md)
closes a second owner-fence gap: on Unix, the live Agent rechecks the current
private owner file's device/inode and bounded checksummed identity before store
operations and receipt commit. A moved directory, replacement lock file or corrupt
identity cannot silently redirect the old process through its configured path.
The descriptor reports the store unavailable; queries do not become `not_found`,
new work is not admitted, and completion is not written into a replacement store.
Local two-Agent tests retain completed receipts and recover interrupted work as
`outcome_unknown` without replay after restoring the original store and restarting.
This is operation-boundary checking, not protection against concurrent filesystem
mutation; Windows handle identity, multi-host and physical-power-loss qualification
remain open. Tensor grades are unchanged.

The [missing admission follow-up](../reports/mesh-missing-admission-20261018.md)
prevents an execution ID from being admitted again when its known admission file
disappears during the Agent process lifetime. Fetch and cancellation fail rather
than treating missing evidence as absence; orphan receipts remain inspect errors
after restart. Recovery listing checks admission membership before pagination so
missing records cannot silently disappear behind a cursor. This adds only a
bounded ID set, not snapshots, input/result caching or automatic repair. Two real
local Mesh regressions cover spare-capacity duplicate prevention, unaffected
sibling/neighbor work, forced restarts and explicit restoration without replay.
Deletion of all execution evidence followed by restart remains outside this guard;
the retained ledger, not the in-memory set, supplies restart evidence.

The [immutable publication follow-up](../reports/mesh-immutable-publication-20261018.md)
prevents failed staging creation from deleting a preexisting `.next` file and
replaces overwriting rename with atomic no-replace hard-link publication. The
visible `record_publication` policy requires same-filesystem hard-link support;
unsupported stores fail without a copy/rename fallback. Startup removes pending
names before reading records, never promotes them, and retains existing final
records. Native process tests cover staging collisions and constructed publication
intermediate states through forced restarts without replay. The format and
engine/operator contracts are unchanged. This does not qualify arbitrary external
filesystem mutation, physical power loss or untested filesystems/platforms.
