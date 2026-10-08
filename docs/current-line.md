# daji 3.x

This is the single entrypoint for the active Kyuubiki product line.
The current development point in this line is `daji 3.5.0`.
Source package metadata targets `3.5.0`; the last locally built and installed
desktop/runtime baseline remains `daji 3.4.0`.
Local installation does not imply that public downloads or SDK packages have
been published; platform acceptance remains separately scoped.

## Daji 3.5.0 Checkpoint

October 7, 2026. Commit `0712b88d` starts this source checkpoint with the
preceding native Headless recovery repairs; current metadata and documentation
now follow 3.5.0. Historical runs retain their base commit, source metadata,
platform and working-tree scope rather than being relabeled as 3.5.0 builds.

- Runtime completion requires a valid task-bound receipt. Unknown, failed or
  blocked execution cannot authorize downstream work or successful-step counts.
- Whole-value bindings require actual completed source outputs; missing or
  empty required values stop before the dependent service request.
- Batch risk labels must match registered action contracts. Sensitive and
  destructive authorizations are independent and cannot repair invalid labels.
- Job waits validate identity and public state; result reads require a matching
  completed job and an explicit object, not merely a retained runtime object.
- Lost or invalid write acknowledgements and HTTP 5xx responses fail as unknown
  outcomes without automatic replay. Zero successful steps does not establish
  zero server-side effects; inspect and reconcile before explicit continuation.
- Native submissions require a usable job identity and consistent public state,
  even after a complete 2xx response. Invalid receipts halt before downstream
  work; known failed/cancelled submissions remain explicit terminal failures.
- Native library writes require usable record identities and matching requested
  objects/parents. Malformed successful replies stop as uncertain writes.
  Registered FEM saved-version solves retain server-side version association.
- Original TaskIR receipt reads preserve the selected attempt and ownership.
  Expiry, eviction, restart or endpoint loss remain unknown without peer fallback.
- Explicit asynchronous solver previews supply job bindings without scientific
  results. Research posture continues to reject mock computation.

The retained [write-acknowledgement regression](../reports/headless-write-acknowledgements-20261007.md)
covers actual project commits and accepted bar jobs with their real replies
withheld from SDK/CLI callers on macOS and remote Linux. The
[job-result](../reports/headless-job-result-gates-20261007.md),
[binding](../reports/headless-runtime-bindings-20261007.md),
[risk](../reports/headless-contract-risk-20261007.md), and
[original-receipt](../reports/original-receipt-boundaries-20261007.md)
reports retain their own source fingerprints. These tests are not new installed
App, authenticated deployment, cross-language, durable or exactly-once proof.

The [submission receipt follow-up](../reports/headless-submission-receipts-20261007.md)
tests the 3.5.0 source overlay at `0712b88d`. A deliberately corrupted actual
job acknowledgement stops SDK/CLI callers while the original jobs calculate
once per explicit request. Test-only reply mutation is not a backend fault
observed in deployment, and successful submission is not result validation.

The [library receipt follow-up](../reports/headless-library-receipts-20261007.md)
checks five library write actions and actual database commits with deliberately
corrupted replies. A normal seven-step research chain retains its saved version
in the server-side job, computes with actual Rust Agents, independently checks
the bar displacement and explicitly removes its project. This remains source
verification, not durable recovery or general scientific qualification.

The [saved reference follow-up](../reports/headless-model-reference-gates-20261007.md)
checks read identities, parent hints and source ambiguity before computation.
Corrupted actual model/version replies stop without a job or Agent calculation.
Normal model and version references retain their distinct context in native FEM
fallback jobs; mutable models are not relabeled as saved versions. The check is
not full payload validation or generic mesh deployment qualification.

The [job-read follow-up](../reports/headless-job-read-gates-20261007.md) makes
native read options and explicit project/version/case constraints enforceable.
Combined saved-version solves keep their version constraint through waiting and
fetching. Actual corrupted job associations stop SDK/CLI continuation, while
normal rereads retrieve the same independently checked result without another
Agent calculation. These checks do not establish complete result provenance.

The [submission association follow-up](../reports/headless-submission-context-20261007.md)
checks the project/version actually transmitted before accepting a native job
acknowledgement. Wrong or missing association stops as an unknown write without
replay. Orchestra now rejects contradictory explicit project/version pairs
before creating work, replacing the earlier version-project precedence rule.
Real SDK/CLI reply-loss tests retain one calculation per explicit experiment.
This is scoped consistency checking, not authorization or result provenance.

The [single job observation follow-up](../reports/headless-job-fetch-gates-20261007.md)
closes the ordinary `job_fetch` path's identity/state/association gap. Invalid
successful replies stop before bindings or downstream actions. Valid failed or
cancelled jobs remain inspectable: query success is distinct from calculation
success and completed-result admission. Actual SDK/CLI fault injection preserves
the original job/result and does not reexecute computation.

The [response-budget follow-up](../reports/headless-response-budgets-20261007.md)
removes unbounded ordinary HTTP response buffering and makes upload receipts
use the same bounded reader. Header/declared-length/observed-byte limits and
nonrenewable network deadlines fail before successful bindings. Large model and
result reads keep a separate 512 MiB channel; non-chunked bodies avoid one full
string copy. Actual committed writes remain unknown when their reply exceeds
the budget, and an already computed result is reread without another calculation.
Parsed values and retained raw/result mirrors are not a process-wide memory cap.

The [binding lifetime follow-up](../reports/headless-binding-lifetimes-20261007.md)
retains only later-referenced outputs and moves each field on its final use,
instead of retaining every complete step result until batch end. Fanout stays
independent; missing outputs, confirmation gates and report shapes are unchanged.
A 20-layer ownership test and old-resolver comparisons check native semantics.
Real SDK/CLI nine-step chains store complete 513-node, 512-element results in
new model versions without repeating the source calculation. Public result
normalization and live branches still allocate; this is not an RSS or scale claim.

The [result ownership follow-up](../reports/headless-result-ownership-20261007.md)
removes extra deep copies when native job/result replies and combined saved-version
solve/wait envelopes are normalized. Original arrays and strings move into their
existing public shapes; required raw/result mirrors remain independently mutable.
Receipt, association, completion and no-replay gates are unchanged. Real SDK/CLI
experiments store complete results with one calculation per explicit solve.
Pointer/value checks are not deployed RSS, throughput or scale qualification.

The [request buffer follow-up](../reports/headless-request-buffers-20261007.md)
encodes inline JSON into one bounded buffer, rejects oversize before connection
with exact byte diagnostics, and sends borrowed header/body slices without another
full request copy. The body is released before response reading; partial writes
retain their original deadline and unknown writes never authorize replay. Real
SDK/CLI chains preserve complete Unicode/escaped research notes and original
results, including a version list larger than the former test-reader cap. This
is not a total RSS/CPU bound, throughput benchmark or scale qualification.

The [report compaction follow-up](../reports/headless-report-compaction-20261007.md)
moves eligible owned payload/result fields into reports, while later bindings
retain their complete original values and independent previews. Array samples
do not retain source capacity; overreserved short arrays/strings release excess
capacity. Borrowed inputs, summary thresholds, completion and failure/no-replay
gates are unchanged. Real SDK/CLI chains compare summary samples and Unicode
prefixes with stored full results; this is not an aggregate RSS or scale claim.

The [output publication follow-up](../reports/headless-output-publication-20261007.md)
streams native Headless CLI JSON through a 64 KiB buffer and replaces individual
files only after staging, flushing and syncing, without old-report backups.
Publication failure preserves the available run receipt on writable stdout and
has a separate non-retryable diagnostic. A real accepted bar calculation is
recovered by reading its original job/result, with one calculation throughout.
This is not a multi-file transaction, durable restart/power-loss guarantee,
installed qualification or total memory/throughput claim.

The [artifact path follow-up](../reports/headless-output-paths-20261007.md)
reserves native CLI input/output paths before patch receipts or execution, then
rechecks them at each publication boundary. Workflow/patch/spec/previous-evidence
inputs cannot be overwritten by output aliases; output files cannot alias each
other or serve as each other's parent directory. Initial rejection writes only
an invalid zero-step JSON stdout receipt, not a failure file on a conflicting
path. Actual Orchestra/Agent tests retain source bytes and unchanged project/job
records with zero calculations. This is scoped macOS source verification, not
a hostile-directory sandbox, multi-file transaction or filesystem qualification.

The [artifact generation follow-up](../reports/headless-artifact-generation-20261007.md)
closes the post-run receipt-loss and error-classification gap. Material/evidence
construction failure retains the actual completed report but exits nonzero with
non-retryable `report_generation_failure`, separate from execution, publication
and preflight errors. Missing/non-numeric metrics cannot masquerade as a job
timeout or fabricate qualified evidence. A real bar calculation retains its
original job/result while public SDK helpers explicitly rebuild verified evidence
from a corrected mapping, with one calculation throughout. This is derived-output
recovery, not automatic replay, durable recovery or qualification promotion.

The [research preflight follow-up](../reports/headless-research-preflight-20261007.md)
adds a public Rust SDK check for the effective batch, metric step references and
baseline/continuous round lineage. The native research CLI calls it before
service execution. Nonexistent steps, first-round patches and malformed/stale
previous evidence stop with a non-retryable zero-step receipt rather than running
and only then failing evidence generation. Real Orchestra/two-Agent rejection
tests retain unchanged project/job state with zero calculations, while a valid
two-round controlled service chain still runs once per round. Runtime metric
values and producer authenticity remain separate gates, not new qualification.

The [material preflight follow-up](../reports/headless-material-preflight-20261007.md)
checks complete candidate solves, owned waits and ordered result readbacks for all
five built-in material studies before service execution. Missing, duplicate or
misidentified sources cannot be silently interpreted as fixed candidate results.
Canonical and grouped schedules remain supported. Actual Orchestra/two-Agent
rejection leaves projects/jobs unchanged and performs zero calculations. This
checks declared identities and structure, not model/material parameter fidelity,
authenticated result provenance or scientific qualification; generic SDK workflows
and explicit retained-result report construction remain separate paths.

The [research input identity follow-up](../reports/headless-research-input-fingerprint-20261008.md)
binds native run reports to lossless effective input fingerprints captured before
dispatch or compaction. Changed parameters cannot reuse old results merely by
matching workflow/actions/validation. Two actual bar rounds check their analytic
displacements and reverify retained evidence without replay; KCore uses the same
gate. Legacy files remain readable but missing input identity cannot qualify new
evidence. Legacy lineage digests stay unchanged, and this is not authenticated
execution, external-model content binding or general scientific qualification.

The [saved model source follow-up](../reports/headless-saved-model-sources-20261008.md)
adds versioned fetched-content fingerprints and optional native SDK source pins.
Changed saved content refuses before submission, artifact upload or downstream
writes; research evidence checks captured source identities and pins. Actual
source Orchestra/two-Agent bar runs verify mutable updates and historical-version
results. This is fetched-snapshot consistency, not authenticated engine execution,
normalized-request identity or installed/cross-language qualification.

The [model upload identity follow-up](../reports/headless-model-artifact-identity-20261008.md)
checks prepared/sent byte digests and typed server references before solver POST,
retains `model_artifact_upload` and refuses a different existing same-size model's
reply without extra calculations or downstream writes. The real source macOS
test checks healthy native-SI bar results by separately reading result files.
The [axial-bar normalization follow-up](../reports/axial-bar-input-normalization-20261008.md)
closes the Pa/GPa scalar mismatch with shared Rust/Elixir conformance. Four actual
inline/file pairs match complete results and independent closed forms; SDK/CLI
invalid inputs block later writes, then a healthy task succeeds without replay.
Other solver schemas remain open; bounded native result readback is now covered below.
These are scoped source contract checks, not installed/remote qualification.

The [native result readback follow-up](../reports/headless-result-artifact-readback-20261008.md)
resolves supported immutable files into physical JSON after completed-job gates,
with explicit byte/deadline limits, raw digest verification and no redirected
credentials. Reference-only mode remains available. Real SDK/CLI corruption
refusal blocks later writes; explicit same-job reads recover research metrics
and retained evidence with one calculation throughout. Temporary files are
owned and removed on normal/error return. This is not a process RSS bound,
signed provenance, installed/remote qualification or general solver accuracy.

The [graph entity normalization follow-up](../reports/graph-entity-input-normalization-20261008.md)
moves ID generation/type/collision checks into ten typed Rust field requests,
removing the Agent's four-type repair shim. Ten HTTP/SDK routes now share
conformance and complete inline/file physical parity with analytic checks.
Tuple-shaped entities reject even when their field count matches. SDK/CLI
invalid chains stop later writes; healthy tasks recover without replay.
The [transport service follow-up](../reports/advection-diffusion-service-chain-20261008.md)
adds direct advection-diffusion submission and discovery, three official SDK
route/RPC mappings and actual source-Agent workflow execution. Native saved
version solves no longer require irrelevant explicit Agent endpoints. Nine
transport tasks check analytic concentration/flux, immutable source-bound
research evidence, failure-stop behavior and recovery without replay; the
ten-family graph repeat has 31 admissions. Official SDK mapping tests use mocks,
not cross-language numerical acceptance. The initial hookup did not change solvers.
This remains source contract evidence, not installed/remote/scale qualification.

The [transport numerical follow-up](../reports/advection-diffusion-output-reliability-20261008.md)
separately fixes global-x orientation in assembly and flux recovery, protects
representable extreme averages/gradients/products, rejects unrepresentable
outputs and checks dense assembly before fixed boundaries can mask overflow.
Shared result checkpoints make cancellation observable before publication.
Nine actual source-Agent admissions retain analytic checks, five terminal
numerical failures without results or downstream writes, and healthy recovery.
This does not qualify unstabilized high-Peclet advection or general CFD.

The [explicit upwind follow-up](advection-diffusion-upwind.md) adds opt-in
conservative transport without changing legacy Galerkin defaults. Physical
flux, artificial diffusion and numerical flux stay distinct. Independent
discrete solutions, first-order continuum refinement and layered nodal balance
extend the current validation profile; eight actual source-Agent admissions
check inline/file/workflow execution, retained research metrics and failure
recovery. This does not promote historical qualification or general CFD accuracy.

The [stabilized research-chain follow-up](advection-diffusion-research-chain.md)
retains artificial diffusion through diagnostics, opt-in quality targets and
objective decisions. Native source workflows block coarse/corrupt evidence,
retain independent branches and recover with explicit healthy submissions.
Typed graph/dataset wire repairs and shared Rust/Elixir float rounding keep the
same chain executable through Headless, Orchestra and Agent without duplicate
physics implementations. This is bounded source evidence, not material acceptance.

The [diagnostic cancellation follow-up](../reports/diagnostic-cancellation-reliability-20261008.md)
adds cooperative safe points to shared borrowed-result scans, including the
stabilization pass. Native task cancellation blocks partial summaries and later
writes, while an explicit same-task rerun retains the original failed attempt.
This does not promise bounded JSON processing, arbitrary-plugin preemption,
durable restart or remote-scale cancellation latency.

The [cross-domain cancellation follow-up](../reports/cross-domain-diagnostic-cancellation-20261008.md)
closes both independent Stokes diagnostic loops and tests thermal, electric,
magnetic and thermo-mechanical vector/component reductions. Five synchronous
Engine graphs cannot publish cancelled results even with `on_error: skip`;
fresh complete graphs recover. A real single-quad Stokes result can explicitly
repeat diagnosis without another solve. This does not qualify non-transport
native TaskIR routing, public graph cancellation or durable continuation.

The [native domain service follow-up](../reports/native-domain-diagnostic-chains-20261008.md)
then extends the existing diagnostic/scoring whitelist to eleven Engine built-ins.
Five actual study chains plus a Stokes triangle chain run sixteen graph jobs in
33 Agent executions. Corrupt results block dependent decisions; retained healthy
results can be diagnosed/scored again without another solve. Five exact-target
diagnostic cancellations retain failed attempts through explicit successful
reruns in ten executions. Stokes quad/triangle RPC names and execution-program
generation now use the registered methods. This adds bounded source routing and
recovery evidence, not new TaskIR solver support, public whole-job cancellation,
installed/remote acceptance, scale or numerical accuracy.

The [retained material run follow-up](../reports/headless-material-run-results-20261008.md)
rejects unfinished or contradictory execution records, checks fetched job
identities and ties fixed candidate reports to their own completed waits/results.
SDK and standalone native report generation share the gate, while raw result
arrays remain caller-owned and explicit mock outputs remain previews. Five actual
Orchestra/Agent studies regenerate reports from retained runs without another
calculation; deliberately corrupted records cannot overwrite prior reports.
This is receipt consistency, not authenticated provenance or physical accuracy.

The [research bundle import follow-up](../reports/material-research-bundle-import-20261008.md)
adds `MaterialResearchBundle::from_json_verified` to both Rust SDKs and uses it
in the published validation example. Saved v1 JSON is checked against all four
embedded artifact digests without changing key order, number tokens or escapes.
Formatting whitespace is allowed. In-memory validators and Python/Elixir remain
structure checks; digest agreement is neither authenticated provenance nor
whole-bundle metadata protection, numerical qualification or a verified badge
that survives later object edits.

The [material input profile follow-up](../reports/headless-material-input-profiles-20261007.md)
closes a fixed-report mismatch: edited solver inputs could still be ranked using
original factory constants. All five built-in reports now require their matching
physical input profiles, including composite models and coupling parameters.
Synchronized SI edits alone do not prove the original candidate. Annotations,
study aliases, runtime context and generic research/patch execution remain
independent. This protects report assumptions, not material truth or numerical
qualification. The artifact test service also waits for bounded complete request
headers instead of racing delayed clients.

See `releases/snapshots/3.5.0.json`. Package, update, language-pack and book
targets are aligned, without changing protocol/schema versions, translation
content or historical grades. The installed baseline remains 3.4.0. The tensor
continues to have four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
release qualification remains blocked. The 3.5.x mainline continues reliable
research execution and recovery under the existing Engine/Solver split.

## Daji 3.4.7 Checkpoint

October 5, 2026. This source patch retains the bounded modal research and
recovery checks developed after commit `dc5647af` (`daji 3.4.6`), then removes
superseded tooling and redundant development outputs.

- Preserve the 39-of-42 independently read-back candidate pipeline. Three
  128-member inputs remain unresolved; bounded lattice alternatives recover
  no new inputs. These modules remain test-only, not runtime fallback routes.
- Retain callback fault, cancellation, fresh replay, request-numbering and
  independent physical readback checks without relaxing numerical gates or
  moving Solver algorithms into Engine/Agent.
- Remove three superseded JavaScript contract checkers and redirect their
  coverage evidence to the existing Rust implementations.
- Prune redundant historical run reports and disposable local build outputs;
  preserve regression source, independent failure evidence and private config.
- Align first-party source, SDK, language-pack and documentation versions to
  3.4.7 without relabeling historical evidence or rebuilding installed binaries.

See `releases/snapshots/3.4.7.json` and the
[bounded lattice report](../reports/modal-bounded-lattice-reliability-20261005.md).
The installed baseline remains 3.4.0. Production modal recovery, full spatial
spectra, external correlation and sustained scale qualification remain open.
The existing tensor calibration profile remains unchanged; a patch number does
not promote research candidates or satisfy open readiness gates.

## Daji 3.4.5 Checkpoint

October 4, 2026. This source patch hardens bounded modal solving while keeping
numerical algorithms in Solver and execution authority in Engine/Agent:

- Tridiagonal preparation validates physical symmetry and representable bands
  once. Preparation errors and cancellation do not become an unsupported fallback.
- Vector arithmetic, refinement and final spectrum/shape validation cooperate
  with bounded cancellation checkpoints. Private candidates publish only after
  all original physical checks succeed; a later failure discards earlier modes.
- Nontridiagonal single-mode problems up to 256 active degrees of freedom use
  a complete dense spectrum and the unchanged 1e-8 physical residual gate.
  Bounded automatic coordinate correction retains the eigenvalue and root guard.
- Rust Headless and development Agent-stage tests cover failure isolation and
  replay without adding numerical algorithms or research fallback routes to Agent.
- Independent physical reassembly compares multimode, projection, phase and
  quantized-QR proposals. These remain test-only; their per-fit budgets are not
  an admitted aggregate runtime portfolio or a throughput claim.

See `releases/snapshots/3.4.5.json`, the
[preparation](../reports/modal-tridiagonal-preparation-reliability-20261003.md),
[final validation](../reports/modal-final-validation-reliability-20261003.md),
[bounded single-mode](../reports/modal-bounded-single-mode-admission-20261003.md),
and [quantized-QR comparison](../reports/modal-triangular-grid-comparison-20261003.md)
reports. Original reports retain their 3.4.4 working-tree provenance. Overlapping
tests are not additive coverage, and registry checks are not execution evidence.
The tiny-scale 128-element public solve remains fail-closed; passing test-only
physical candidates do not establish public Solver, installed Agent or remote
qualification. Recompute affected historical modal results before research reuse.
External correlation, complete spatial spectra and sustained scale readiness
remain open. No binaries are rebuilt, installed or published here; tensor gaps
remain open.

The [October 4 tensor recalibration](../reports/tensor-recalibration-20261004.md)
updates the hardening profile to this source checkpoint without promoting
candidate research into production proof. It retains all 224 claims and adds
twelve modal scenarios: four verified retained scopes and eight open production
obligations. The current target result is 58/77 coordinates and 10/32 reviewed
scenarios, not test coverage or release qualification. The
[HTML architecture chapter](book-ch03-architecture-boundaries.html#tensor-calibration)
and [roadmap](weakness-roadmap.md#current-tensor-status) carry the full limits.

The subsequent [public request-reassembly correction](../reports/modal-request-reassembly-reliability-20261004.md)
fixes sampled node-numbering-dependent acceptance in a bounded horizontal
single-mode bending path, preserving original output numbering and numerical
gates. Independent readback covers 27 fixtures, 108 layouts and 216 owned/borrowed
solves, plus late-cancellation replay and two local Rust Headless bridge tests.
The tensor now retains 225 claims; the reassembly proof is verified, not yet
qualified, so target counts remain unchanged. Hard 128-segment cases, general
modal recovery and actual Agent/installed Headless study qualification remain
open. No package or version is changed by this follow-up.

The [bounded banded-inverse candidate follow-up](../reports/modal-banded-inverse-candidate-reliability-20261004.md)
then recovers all six retained 128-segment graded/layered candidate fixtures,
including graded-large, with frozen roots and independent physical JSON checks.
Four wide inverse iterations initialize the existing discrete portfolios;
nearest rounding alone still rejects. This code is test-only: actual public
Solver calls remain rejected, and production, cumulative-budget and Agent/SDK
qualification remain open. Current retained claims total 226, without changing
the 58/77 coordinate or 10/32 scenario targets met.

The [separate fresh banded candidate cost record](../reports/modal-banded-inverse-pipeline-cost-20261004.md)
removes initializer copies/history and measures all six alternative chains with
exact repeated receipts. Actual certificate callbacks replace duplicate nested
event counting. Its hardest chain still uses six grid factors and 23 certificates;
scoped reservations and process RSS do not qualify a complete production budget.
Existing claim and target counts remain unchanged.

The [ranked-then-reverse candidate comparison](../reports/modal-banded-ranked-policy-cost-20261004.md)
retains six successes with two fits/thirteen certificates per stage and a lower
worst-case grid reservation. It records both the graded-large improvement and
tiny-layer regression in paired timings. Production admission/budget scopes and
claim/target counts remain unchanged.

The [checked-order handoff comparison](../reports/modal-banded-checked-order-handoff-20261004.md)
uses the accepted internal strategy tag for one independently certified physical
fit, never reusing its permutation, factor or certificate. Six outputs retain
exact bits; layered-tiny drops four grid fits to three. Formal chain caps are
three fits/twenty certificates, with no fallback after physical rejection.
Local cost improves for that case, not universally; complete budgets stay open.

The [rebuilt-request follow-up](../reports/modal-banded-handoff-request-reassembly-20261004.md)
retains exact results for 24 fresh numbering layouts and independent physical
readbacks for sixteen of eighteen changed material inputs. The two tiny-layer
density-times-four cases reject handoff, independent ranked and independent full
physical portfolios, with healthy fresh baseline replay. These are test-only
boundaries, not a production fix or new qualified tensor scope.
Two separate reference-only directions pass the failed changed-material
operators/readbacks near 8.59e-9 without seeding searches; candidate generation,
not a proven impossibility under the gate, remains the next numerical gap.

The [cold inward-chart follow-up](../reports/modal-material-inward-chart-reliability-20261004.md)
now constructs passing candidates for both counterexamples without reference
seeds or additional physical fits. Twenty-four material inputs pass 52 fresh
original-numbered JSON readbacks and eighteen spectral/material relation checks.
The anchor moves one f64 step toward zero before the sole fit, then stays frozen.
Fault/cancellation/replay and one-fit/seven-check bounds remain explicit.
Layered-tiny baseline residual gets worse, though still below 1e-8; this is not
universal improvement or production admission. Original-chart failures remain
asserted; full budgets, broader selection and actual Agent/SDK qualification stay open.

The [independent recipe/mesh holdouts](../reports/modal-material-holdout-reliability-20261004.md)
compare 36 inputs through 72 fresh cold routes. Both charts publish 31 checked
results and retain four internal and one physical failure. Inward movement
improves 16 margins and worsens 15; it is not promoted to a default. Actual
failed-fit accounting, preparation declines and final publication cancellation
replay distinguish early exits from completed physical/readback validation.

The [internal construction follow-up](../reports/modal-internal-chart-construction-reliability-20261004.md)
retains 204 isolated diagnoses and 108 fresh cold routes. Moving the internal
anchor one step inward recovers the three-layer 100-member tiny request, but
loses a formerly passing unequal-length large request: each policy still passes
31 of 36. Three 128-member internal failures and six wide preparation faults
remain explicit. This complementary test-only policy is not a default or a
qualified production recovery; cancellation and healthy replay remain bounded.

The [wide QR range follow-up](../reports/modal-wide-givens-range-reliability-20261004.md)
separates two overwritten-tail faults from four retained tiny-fill faults without
relaxing the range guard. A cold Givens factor moves, rather than removes, the
difficult-input failures. Over 144 isolated material/order routes it gains three
normalized acceptances and loses five; no physical result is published. The
three 128-member layered cases remain unresolved, and no default is changed.

The [banded floating-grid follow-up](../reports/modal-banded-grid-construction-reliability-20261004.md)
tests one or four fixed construction passes with at most 729 frontier states.
Across 42 inputs and 84 cold routes, each policy accepts nine holdout normalized
directions and none of the six old 128-member baselines. Three difficult
128-member inputs remain unresolved despite lower residuals. Prediction is
not acceptance: two actual receipts, fixed gates and bounded cancellation
remain required. No physical publication, default change or new qualified
claim follows from these test-only negative results.

The [amplitude and rounded-beam follow-up](../reports/modal-amplitude-rounded-beam-reliability-20261004.md)
retains 128 isolated amplitude charts and 96 beam diagnostics. In 84 fixed-order
pairs, width-16 beam gains ten normalized acceptances and loses one, compared
with four-pass rounded greedy recovery. The 100-member tiny input passes
internally but both retained physical charts reject it; all three 128-member
inputs remain unresolved. Four healthy original-numbered readbacks and late
fault/cancellation replays are test-only, not a default or production promotion.

The [physical-first joint follow-up](../reports/modal-physical-first-joint-reliability-20261005.md)
retains 42 inputs and 126 cold routes: the old full pipeline accepts 37, while
joint GridNorm/Reverse accept only one/two. The joint policy adds rounded
internal re-encoding of the final physical shape at the initializer's frozen
anchor, which the old contract does not require. Its negative boundary does
not invalidate old outputs. Real final-pair/publication cancellation, faults,
fresh numbering and healthy replay are retained; no default or scope is promoted.

The [coupled two-stage follow-up](../reports/modal-coupled-shape-selection-reliability-20261005.md)
keeps the old internal-to-physical contract without final internal re-encoding.
Mapped physical error ranks only internally eligible directions; the recovered
physical shape still needs strict norm, actual residual and independent JSON
readback. Of 42 inputs, old/GridNorm/Reverse accept 37/36/28. GridNorm gains the
unequal-length 128-member tiny case but loses two old successes; Reverse loses
nine. Separate one-fit/eighteen-receipt stages and real late cancellation with
fresh exact replay remain test-only. No production default or scope is promoted.

The [legacy-first physical follow-up](../reports/modal-legacy-first-hybrid-reliability-20261005.md)
isolates those losses with twelve cold ablations. Keeping the old internal
policy and greedy physical output, with a residual-only beam continuation on
the same factor, preserves all 37 old successful JSON outputs and counts
exactly and gains one tiny unequal-length input: 38 of 42 accept. Four internal
ThreeLayers failures remain. The physical cap is one fit/twenty-five receipts;
faults, cancellation, stale final gates and norm loss cannot start continuation.
This bounded positive research does not replace production defaults or scopes.

The [breadth and iteration follow-up](../reports/modal-breadth-iteration-reliability-20261005.md)
preserves all 38 previous successful JSON outputs and counts exactly. A single
preselected Reverse width-64 internal pass, only after old numerical exhaustion,
recovers the ThreeLayers 100-member tiny input at independent readback
`8.943162509310455e-9`: 39 of 42 pass, with three internal failures open.
The 72 isolated diagnostics show that narrower repeated passes' better internal
residuals do not recover this physical output. The extended internal cap is
three fits/79 receipts; the physical cap remains one fit/25 receipts. Real
faults, signed/renumbered shapes, late cancellation and fresh exact replay are
retained. This is test-only Solver evidence, not production admission.

The [bounded lattice follow-up](../reports/modal-bounded-lattice-reliability-20261005.md)
changes integer coordinate construction rather than tolerance or beam width.
Twenty-four adjacent-pair routes add no success. Twelve bounded triangular
routes retain whole transformations at explicit step/integer limits; two
alternate directions on the already recovered 100-member tiny input pass
physical JSON readback. Three 128-member internal failures remain, despite a
lower tiny-case diagnostic residual. Distinct-input coverage stays 39 of 42.
Exact integer mapping, real faults, cancellation and fresh replay stay private
Solver research, not production admission or Engine/Agent retry behavior.

## Daji 3.4.4 Checkpoint (Historical)

October 2, 2026. This source patch hardens modal calculation and its admitted
Agent execution paths without moving numerical ownership out of Solver:

- Physical assembly and shared mass normalization reject invalid range loss.
  Dense and sparse paths retain representable weak couplings and soft components.
- Repeated-root subspaces, sparse inverse refinement, chain recovery and published
  physical shapes keep the original residual and convergence gates. Exceptional
  sparse products avoid intermediate range loss; ordinary arithmetic is retained.
- Agent TaskIR admits only the existing bar and the two modal built-ins. Matching
  entrypoints dispatch through Engine; authority and package ownership stay enforced.
- Numeric JSON round trips retain task digests. Local live TCP tests cover rejection,
  cancellation, failure isolation, same-connection recovery and successful replay.
- Native cache inspection and allowlisted cleanup reclaim development artifacts
  without deleting source, dependencies, retained evidence or installed releases.

See `releases/snapshots/3.4.4.json`, the
[normalization](../reports/modal-normalization-reliability-20261002.md),
[sparse product range](../reports/modal-sparse-product-range-reliability-20261002.md),
and [Agent TaskIR](../reports/modal-agent-taskir-reliability-20261002.md) reports.
The reports retain their 3.4.3 working-tree provenance; overlapping tests are not
additive coverage. Recompute affected historical modal results before research reuse.
The retained 128-element bending case remains fail-closed within its original budget.
Local debug timings do not qualify release throughput, and these tests do not
establish external correlation, general physics, installed-platform or scale readiness.
Tensor gaps remain open. No binaries are rebuilt, installed or published here.

## Daji 3.4.3 Checkpoint (Historical)

September 28, 2026. This source patch hardens existing structural and
thermal-structural operators while retaining the Engine/Solver boundary:

- Truss, thermal-truss, beam, and frame paths check assembly and recovered
  physical fields. Range-aware arithmetic preserves representable outputs;
  invalid values fail even when every degree of freedom is restrained.
- Planar thermal-frame energy includes bending-field variation. Spatial
  thermal-frame energy avoids subtracting large, almost equal totals.
- Spatial directions use scale-first normalization. Exact thermal-frame
  support constraints and recovered reactions share a stable basis; redundant
  or overcomplete support directions fail without a rank-underflow panic.
- Explicit spatial section hints form a right-handed orthonormal basis.
  Equivalent hints retain mechanical/thermal fields and rigid modes, while
  the default reference-axis convention is preserved.
- Public Solver and Rust Headless plan/bridge/Engine regressions cover closed
  forms, orientation, refinement, cancellation, and fresh valid replay.

See `releases/snapshots/3.4.3.json` and the
[truss](../reports/truss-output-reliability-20260927.md),
[thermal truss](../reports/thermal-truss-output-reliability-20260927.md),
[beam](../reports/beam-output-reliability-20260927.md),
[planar frame](../reports/frame-2d-output-reliability-20260927.md),
[spatial frame](../reports/frame-3d-output-reliability-20260927.md),
[support](../reports/frame-3d-support-reliability-20260927.md), and
[orientation](../reports/frame-3d-orientation-reliability-20260927.md) reports.
Those reports retain their 3.4.2 working-tree test provenance. Recompute affected
historical results before research reuse. Scoped macOS/Linux regressions are
not external-solver correlation, general physical qualification, installed-Agent
or GUI acceptance, Windows qualification, or a new large-scale benchmark.
Tensor readiness gaps remain open; no binaries are rebuilt or published here.

## Daji 3.4.2 Checkpoint (Historical)

September 27, 2026. This source patch strengthens existing workflow and physical
execution paths without coupling operator implementations to the engine:

- Workflow admission and deletion commit the job/result pair atomically inside
  the existing ownership boundary. Failed writes preserve the prior state;
  successful deletion retires tracked local runners and rejects late publication.
- Result administration uses one guarded mutation. Missing receipts are not
  recreated, private recovery data cannot be injected, and terminal edits retain
  workflow identity. This is not acknowledged remote Agent cancellation.
- CST triangle gradients retain orientation, correcting clockwise and mixed
  meshes in mechanical and thermal solves. Historical affected results must be
  recomputed before research reuse.
- Q4 and tetrahedral geometry use local coordinates, with finite coefficient
  guards and reused stiffness intermediates. Paired Linux benchmarks measure
  individual kernels, not an end-to-end solver speedup.
- Thermal-plane recovery uses range-aware averaging, checked energy accumulation,
  and stable displacement norms. Invalid fields fail instead of reaching a
  successful JSON result as null.

See `releases/snapshots/3.4.2.json`, the
[workflow admission](../reports/workflow-admission-reliability-20260927.md),
[deletion](../reports/workflow-deletion-reliability-20260927.md),
[result administration](../reports/result-administration-reliability-20260927.md),
and [thermal output](../reports/thermal-plane-output-reliability-20260927.md) reports.
Local SQLite/memory and scoped macOS/Linux solver regressions do not establish
live PostgreSQL, remote failover, power-loss, all-physics, large-scale, or
installed-GUI qualification. Tensor readiness gaps remain open. This patch does
not rebuild desktop apps or publish downloadable packages.

## Daji 3.4.1 Checkpoint (Historical)

September 27, 2026. This source patch hardens the existing research execution
path rather than adding more physical operators:

- Shared Rust/Elixir contracts align workflow conditions, numeric comparisons,
  and named-input routing before downstream execution.
- Job transitions compare full observed snapshots; stale watchdog writes cannot
  overwrite newer progress. Solver completion is published with its validated
  result, not inferred from an intermediate Agent message.
- Workflow runtime, recovery envelopes, and job updates share an atomic commit
  boundary. Memory jobs/results persist in one integrity-checked generation;
  legacy import and recovery files stay bounded.
- Scoped storage failures leave watchdog and recovery coordinator processes
  alive. Replay still obeys idempotency/checkpoint policy and ownership claims.
  Health reports degraded state and unknown counts without masking programming
  errors or claiming that failed writes succeeded.

See `releases/snapshots/3.4.1.json` and the
[storage outage report](../reports/storage-outage-recovery-20260927.md).
Local SQLite/memory regression evidence is not live PostgreSQL, remote failover,
power-loss, numerical, or installed-GUI qualification. Existing tensor gaps remain
open. This patch does not rebuild desktop apps or publish downloadable packages.

## Daji 3.4.0 Checkpoint (Historical)

September 27, 2026. This checkpoint carries the 3.3.x operator and workflow
hardening into a coordinated desktop/runtime rebuild:

- Graph preflight rejects malformed topology, ambiguous inputs, cycles, and
  oversized JSON before jobs, indexes, or operator callbacks are created.
- Operator results are validated before artifact publication and lineage.
  Invalid callback envelopes or oversized output fail the producing node;
  explicit skip policy isolates the branch instead of reporting false success.
- Rust and Elixir share graph, artifact, field, and diagnostic-bundle fixtures.
  Diagnostic source ordering is explicit even when the workspace enables
  JSON insertion-order preservation.
- CI checks include split metric-alias modules and the terminal workflow
  receipt. The GUI harness retries only one identified pre-page libdbus crash,
  preserving both errors on repeated failure; test assertions are not retried.
- Product, SDK, language-pack, installation, and documentation metadata align
  to 3.4.0. TaskIR and exchange schema identifiers remain unchanged.

Build and validation outcomes are recorded in `releases/snapshots/3.4.0.json`.
Graph and artifact checks do not prove numerical validity, process memory
isolation, an aggregate response-size cap, or new scale/platform qualification.
The coverage tensor retains its dated evidence and open acceptance gates.

## Daji 3.3.0 Checkpoint (Historical)

This checkpoint records the source state on September 20, 2026, including the
recent 3.2.x work carried into 3.3.0:

- The normal modeling viewport now keeps study setup, modeling, inline checkpoint
  save, execution, and result review together in a compact action strip. Saving
  uses the same project/version controller as immersive mode; execution uses the
  existing PWDT action path. The fullscreen workspace retains its own tool strip
  without another duplicate row. Empty results and duplicate runs are guarded,
  including delayed submissions across panel switches; cancellation remains
  available while execution is being observed.
- Hub project creation now has a focused bundle-creation flow and a native
  directory picker, rather than requiring every destination to be typed.
  See the [first research tutorial](tutorial-first-research.html).
- Workbench PWDT has a dedicated workspace, embedded Python editor, and grouped
  execution and inspection panels. Dense material-library, runtime, and security
  panels use compact grouping rather than one long stack. See the
  [PWDT tutorial](tutorial-pwdt-automation.html).
- Local language-pack repairs cover mixed-language root and extended copy.
  Arabic, Persian, and Spanish PWDT copy now covers 148 panel entries, eight DSL
  entries, and 19 catalog-control entries per language. This is a bounded copy
  contract, not proof of complete translation across all 30 locales.
- Localization checks distinguish unit/browser state-retention tests from actual
  Pyodide calls in the installed macOS WebView. SDK-bridge copy, generated action
  summaries, diagnostics, and other locale surfaces still need separate review.
  See [language-pack coverage and limits](language-packs.md).

The local macOS rebuild has a scoped installed startup and research-journey report
in `releases/snapshots/3.3.0.json`, plus 18 focused browser regressions. It adds no
new numerical, scale, or non-macOS qualification. The coverage tensor keeps its dated calibration, evidence grades,
and open scenario requirements; its current version label is not a new test run.
The next work remains complete, recoverable research journeys and closure of the
[evidence-backed weak coordinates](weakness-roadmap.md#current-tensor-status).

## What Changed

Daji succeeds the moxi 2.x development line. Version 3.0.0 aligns the product
brand, three independent desktop shells, Orchestra, Rust Agent and Engine,
Worker SDK, official Rust/Python/Elixir Headless SDKs, language packs,
installation contracts, update channels, and documentation.

This is a product-line transition, not a wholesale protocol version bump.
Existing TaskIR, workflow, dataset, material, and KCore schema identifiers stay
unchanged unless their contracts actually change.

## Early Daji Mainline

Early Daji is a hardening phase toward an agent-driven industrial research
system, not another broad feature-expansion phase. Industrial reliability is
the acceptance goal, not a status granted by the version number.

Research agents, including external AI callers, use the official Headless SDKs
to discover capabilities, propose bounded studies, submit work, observe results,
and prepare subsequent rounds. Rust Agents are separate execution processes:
they admit language-neutral tasks and run operators through their engines.
Neither role replaces the other, and neither may bypass caller-owned approval,
resource limits, or numerical quality gates.

The primary acceptance journey is:

1. Declare a research objective, constraints, metrics, and stopping budget.
2. Discover the installed runtime and validate a reproducible workflow.
3. Authorize and execute real operator tasks without a GUI prerequisite.
4. Observe progress and diagnose, cancel, or resume interrupted work safely.
5. Validate results before ranking candidates or admitting another round.
6. Export the research lineage and evidence for replay and human review.

Workbench remains first-class for modeling, inspection, intervention, and
review of the same backend state. PWDT automates the fixed GUI; it is not a
mandatory bridge for Headless SDK users. Orchestra and explicit direct/mesh
control paths keep their distinct, equally supported authority boundaries.

Prioritize blockers in this complete journey over isolated feature counts.
The [weakness roadmap](weakness-roadmap.md#priority-order) maps that work to
existing tensor coordinates and retained evidence. New physics or abstractions
should be added when they remove a demonstrated blocker, not to widen the
catalog alone.

## What Carries Forward

- Contract-driven mechanical, thermal, electromagnetic, acoustic, modal,
  transport, and bounded flow studies with explicitly different maturity.
- Serial and parallel operator workflows, typed datasets, provenance, and
  path-independent `.kcore` research exchange.
- Decoupled GUI, Orchestra, direct Agent, and offline mesh control paths.
- Rust-only Worker extensions, Rust/Python/Elixir Headless control SDKs, and
  frontend-owned PWDT automation as three distinct extension surfaces.
- Persisted research outcomes, bounded recovery, integrity-checked runtime
  installation, and native operational tooling.
- Thirty-locale local language packs shared by the product-owned UI surfaces.
- Retained scale, numerical, recovery, and usability evidence with original
  platform, version, and execution scope.

The detailed [moxi closeout](moxi-closeout.md) remains a historical record;
old evidence is not relabeled as a new Daji run.

## Readiness Boundary

A 3.0.0 version number does not certify every solver or every platform.
The coverage tensor and usability release gate remain authoritative.
Open external numerical correlation, installed-platform, recovery, and
GUI/PWDT parity coordinates must close through new retained evidence.

The planned public channel remains Reddit. This source transition does not
publish packages, create a signed tag, notarize desktop applications, or
authorize an automatic rollout. Download and apply still require the
configured source, artifact integrity, and explicit Installer policy.

## Version Cadence

The Daji line spans `3.0.0` through `3.20.9`: 21 minor positions and 10
patch positions per minor. No subsequent codename is declared.
The archived moxi stabilization window is not an active Daji restriction.

## Reading Path

- [Daji 3.0.0 release notes](daji-3.0.0.md)
- [HTML book](book.html)
- [Formal version policy](version-line.md)
- [Current architecture](current-architecture-map.md)
- [Weakness roadmap](weakness-roadmap.md)
- [Operator reliability](operator-reliability.md)
- [Headless SDKs](headless-sdks.md)
- [Worker / Operator SDK](operator-sdk.md)
- [Packaging and deployment](packaging-and-deployment.md)
- [Minimal industrial closure](minimal-industrial-closure.md)
