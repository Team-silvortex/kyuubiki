# daji 3.x

This is the single entrypoint for the active Kyuubiki product line.
The current development point in this line is `daji 3.6.0`.
Source package metadata targets `3.6.0`; the last locally built and installed
desktop/runtime baseline remains `daji 3.4.0`.
Local installation does not imply that public downloads or SDK packages have
been published; platform acceptance remains separately scoped.

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

The tensor still records four maturity, 20 evidence-grade and 15 P0 gaps;
release qualification remains blocked. Direct sweep-request, plan and expansion
quality-metadata admission now have scoped follow-ups below. Expansion counts and
direct execution-batch quality evidence also have bounded checks. Batch structure,
unique recovery identities and checkpoint input/policy admission now have a scoped
follow-up. Preparation counters/identities and persisted completion evidence now
have bounded revalidation too. Execution failure identity, recovery permissions and
outcome/counter partitions now have a scoped follow-up. Remaining audits include
cross-process attempt ownership, authenticated provenance, standalone TaskIR boundaries and producer-side
readiness preservation through ranking.
See `releases/snapshots/3.6.0.json` for the source checkpoint and scope.

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

The [public graph cancellation follow-up](../reports/native-workflow-job-cancellation-20261011.md)
connects native diagnostic/scoring activity to the existing workflow watchdog.
Cancelled-job heartbeats retain dispatch ownership for targeted cancellation
without renewing state or permitting node/result publication. Five domains under
default and skip-on-error handling exercise ten cancellations and ten explicit
recovery graphs in sixty Agent executions. Bystander requests stay untouched;
cancelled diagnostic attempts remain failed with no score dispatch or automatic
replay. Retained recovery diagnostics are not successful scientific outputs.
This is source macOS diagnostic-stage graph-route evidence, not arbitrary parallel
graph cancellation, installed/remote acceptance, numerical accuracy or durable recovery.

The [queue cancellation and progress-fencing follow-up](../reports/native-workflow-queue-fencing-20261011.md)
then closes the non-persisted node-progress path: write throttling no longer
bypasses terminal-job, execution-claim or Orchestra-lease checks. Controlled
300-node graphs cannot enter dependent scoring after cancelled completion.
Five domains under both default and skip-on-error policies cover twenty queued
cancellations and twenty explicit recovery graphs in one hundred Agent executions.
Queued diagnosis/scoring never dispatches, completed diagnosis observations stay
unchanged, and releasing capacity cannot resurrect the cancelled graph. This
does not qualify every pre-admission race, arbitrary parallel graphs or durable recovery.

The [dispatch-admission follow-up](../reports/native-workflow-dispatch-fencing-20261011.md)
separates in-flight activity from permission to start a new operator. Typed solves
and bounded native diagnostics/scoring recheck job, execution claim and Orchestra
lease immediately before dispatch, including capacity retries. TaskIR client
options now preserve that guard without serializing it. Twelve public-cancellation
runner cases and twenty-four actual-client TCP failure cases prevent new admission,
peer retries and extra journal attempts without liveness writes or Agent health
penalties. This is controlled source admission evidence, not exhaustive races,
remote/installed acceptance or independent numerical qualification.

The [parent dispatch identity follow-up](../reports/native-workflow-dispatch-identity-20261011.md)
separates executable TaskIR identity from the actual RPC parent job in the compact
v2 dispatch record. Legacy v1 records remain readable without bulk rewriting or
guessed cancellation authority. Five real graph diagnoses are inspected and
precisely cancelled through the Rust SDK while same-parent peer executions remain
active; five explicit recovery graphs succeed in 25 Agent executions. Failed
diagnoses cannot dispatch scoring, satisfy a successful wait or become successful
old attempts. This is local source contract/recovery evidence, not remote or
installed acceptance, independent numerical qualification or exhaustive races.

The [native restart-fencing follow-up](../reports/native-workflow-dispatch-recovery-20261011.md)
checks retained ambiguous dispatches before graph claims, including ordinary
startup after rollback to pending. Idempotence or checkpoint metadata cannot
override an unconfirmed original outcome. Five real BEAM restarts preserve
SQLite, dispatch records and lease identity; old generations stay blocked and
five explicit new graphs succeed in 20 Agent executions. Original receipts remain
read-only, and pure local recovery does not depend on the native dispatch journal.
This is local source restart denial, not durable node continuation, exactly-once,
power-loss or installed/remote qualification.

The [native successful-receipt recovery follow-up](../reports/native-workflow-success-receipt-recovery-20261011.md)
lets a later authorized graph generation reuse an observed successful native
node only after rebuilding its TaskIR, matching the actual parent and reading
the original attempt through existing completion gates. Claim/lease checks run
before and after receipt retrieval. Five real BEAM restarts complete the same
parents in ten Agent executions without repeating diagnoses; a sixth restart
also loses the Agent cache and blocks without replay, retaining its envelope.
Missing or changed evidence cannot become skip-on-error success. This is bounded
native receipt-assisted recovery, not general durable per-node continuation,
exactly-once or installed/remote qualification.

The [native attempt-selection follow-up](../reports/native-workflow-attempt-selection-20261012.md)
does not choose between multiple dispatched attempts merely because one succeeded
or sorts newest. A single successful candidate is rechecked against the journal
after receipt retrieval. Concurrent outcome changes, another relevant dispatch,
record eviction or journal loss block reuse rather than publishing a stale result.
Unrelated parents and confirmed non-dispatched attempts do not poison a uniquely
owned success. Controlled local tests cover journal restart and bounded retention;
the graph retains blocked recovery state even with skip-on-error. No new Agent
calculation, result archive or larger storage budget is introduced. This is bounded
service recovery evidence, not an atomic cross-store publication or power-loss proof.

The [dispatch observation follow-up](../reports/operator-dispatch-observation-transitions-20261012.md)
seals each attempt's first observation, including unknown outcomes. Conflicting or
malformed finish messages reject only that request instead of rewriting history or
poisoning unrelated dispatches. Identical repeats verify the original on-disk
generation without rewriting timestamps or refreshing retention; missing, corrupt,
replaced or partial storage still stops dispatch. First observations cannot move
timestamps backwards. Local transition, restart and controlled TCP tests preserve
the recovery fence and allow explicit read-only receipt retrieval without clearing
uncertainty. No extra archive, larger storage budget, remote acceptance or atomic
cross-store publication guarantee is introduced.

The [live dispatch-read integrity follow-up](../reports/operator-dispatch-read-integrity-20261013.md)
checks directory membership and selected on-disk observations before returning
history or admitting native recovery. Empty queries still verify the inventory;
new dispatch reservation cannot silently recreate a lost nonempty journal.
Missing, changed or partial evidence blocks inspection, receipt retrieval and
cancellation before their Agent RPC. Native reuse also checks storage after receipt
retrieval. Relevant payloads alone are decoded, with no read-time writes, archives
or retention renewal. This covers live cached history and controlled local recovery,
not authenticated provenance, an atomic cross-store snapshot or detecting a journal
already lost before process startup.

The [durable native-attempt follow-up](../reports/workflow-native-dispatch-evidence-20261013.md)
now closes that fresh-process gap for coordinator-managed native workflows whose
private recovery record survives. Before an Agent RPC, the reserved attempt ID
and task identity must commit through the existing fenced job/result transaction.
Replay admission requires every recorded attempt in the journal, even after a
restart with an absent or empty directory. Another attempt with the same task
cannot substitute. Runtime checks repeat before receipt reuse and reservation;
failed persistence sends no RPC, and unknown completion retains blocked recovery
even with skip-on-error. The private bucket has 512 entries/512 KiB maximum, stores
no payloads, results, endpoints or credentials, and prunes on ordinary terminal
states. Public recovery exposes only policy/count. Missing legacy native evidence
blocks recovery pending explicit inspection/resubmission; pure local graphs remain independent.
This uses controlled local TCP and process-restart tests, not a real Rust Agent,
installed/remote acceptance, cross-store atomicity or exactly-once proof. Joint
loss/rollback of both stores remains separate work.

The [active-history retention follow-up](../reports/operator-dispatch-active-retention-20261013.md)
protects terminal dispatch observations while their actual parent is active,
recovery-blocked, inconsistent or still retains private recovery evidence.
Ordinary matching completed/failed/cancelled parents release this protection
after their recovery envelope and attempt bucket are pruned. Only unprotected
terminal history participates in the 128-record retention window; unresolved
attempts remain protected, and the overall 512-record/4-KiB-per-record cap is
unchanged. A full protected journal rejects new work rather than evicting evidence.
Bounded parent metadata is read outside the journal process, avoiding a dependency
cycle with lease holders or a saturated database pool. Only unchanged records in
that observation can be pruned. Storage lookup failure rejects admission without
deleting files or quarantining a healthy journal. Local SQLite and durable-memory
tests cover turnover, restart, release and failure isolation; this is not an Agent
receipt-retention extension or protection against joint store rollback.

The [public receipt-read follow-up](../reports/operator-dispatch-result-revalidation-20261013.md)
extends the post-retrieval journal check to the shared read-only API, not only
workflow-native reuse. Even a valid Agent reply is withheld if its selected
observation changed, disappeared or failed on-disk validation during the query.
Legitimate transitions/eviction return `operator_task_dispatch_observation_changed`
without quarantining healthy history; a fresh read remains possible. Actual
damage retains the existing fail-closed journal behavior. HTTP errors contain no
completion, raw storage content or local paths. Complete journal restarts can
still return the original receipt when the exact observation survives. This is
local controlled-TCP/source evidence, not a longer Agent cache lifetime, atomic
cross-store read or new real-solver qualification.

The [native result-handoff follow-up](../reports/workflow-native-result-handoff-20261013.md)
adds bounded, verified native success receipts to the existing private recovery
record before downstream publication. Later authorized generations can use that
handoff without an available Agent cache, but still require the original unique
successful dispatch, exact observation digest, rebuilt TaskIR, durable attempt
identity and current claim/lease. It retains at most 32 receipts, 512 KiB per
receipt and 2 MiB encoded bytes per parent, separate from solve/branch budgets.
Oversize or full capacity omits new entries without evicting completed evidence;
the original receipt remains required on recovery if no handoff exists. Failed
commits and damaged checkpoints block, including skip-on-error. Ordinary terminal
states remove the bucket; blocked recovery retains it and prevents early journal
release. Ten actual local application-restart tests use controlled TCP producers
for extract/transform paths, then remove all Agent endpoints. This is bounded
SQLite/memory source evidence, not a live Rust Agent restart, general node
continuation, exactly-once execution or an atomic Agent-to-database handoff.

The [managed uncertainty propagation follow-up](../reports/workflow-managed-dispatch-uncertainty-20261013.md)
closes the live path before recovery: once a managed native dispatch seals an
unknown observation, it must return a blocking recovery error, not an ordinary
transport/application error or an unverified success. Skip-on-error cannot hide
it, and even idempotent/checkpoint hints cannot dispatch that attempt to a peer.
The parent retains its original generation and evidence across coordinator and
journal restart. Concurrent cancellation cannot replace this uncertainty with a
plain cancellation error. Transport/protocol faults still affect Agent health;
local callback failures do not. Confirmed pre-execution connection/capacity
rejections retain safe failover, and independent work remains available.
This applies to the trusted pre-RPC evidence callback used by managed workflows;
standalone TaskIR routing, its existing explicit replay policies and raw solver
RPC behavior are unchanged. This is controlled local TCP/source evidence, not
new physical-Agent, installed or power-loss qualification.

The [native failure-finalization follow-up](../reports/workflow-native-failure-finalization-20261013.md)
checks the recovery envelope and native dispatch fence before ordinary failure
cleanup. Killing a local waiter at the workflow runner deadline is not evidence
that the Agent stopped: unconfirmed attempts, missing/corrupt journal history or
invalid dispatch evidence retain the envelope and become `recovery_blocked`.
The original failure message remains visible, with a bounded reason and next
action in recovery history. Confirmed outcomes and undispatched work still fail
normally; local-only graphs do not depend on journal availability. Before killing
the waiter, the runner captures its actual dispatch endpoints for best-effort
targeted cancellation. A cancellation acknowledgement does not settle the
dispatch observation or authorize replay. Local controlled-TCP tests cover the
deadline, late replies and recovery-component restart.

The [periodic watchdog follow-up](../reports/workflow-watchdog-finalization-20261013.md)
now routes queue/execution deadlines and stalled-job decisions through the owning
coordinator. The exact scanned job must still match; workflow recovery and job
failure commit together, retaining unresolved native evidence. Only a successful
commit permits local runner shutdown and best-effort cancellation of its actual
dispatch targets. Fresh heartbeats, terminal results, lost ownership or rejected
writes cannot be treated as successful expiry. An unrelated task on the same
controlled Agent continues normally. Plain jobs retain snapshot checks, but expiry
now also requires coordinator authority; unavailable/standby control planes report
an unavailable scan instead of claiming work was retired. A repo-restart registry
race is classified as storage unavailability without swallowing unrelated errors.
This does not repair historical inconsistent records or make cancellation durable.
SQLite and memory source tests are not new physical-Agent or power-loss acceptance.

The [bounded solve-artifact follow-up](../reports/workflow-solve-artifact-checkpoints-20261011.md)
retains successful small typed-solve results in the existing private recovery
record before dependent publication. Each parent retains at most 32 entries,
512 KiB per output and 2 MiB total encoded checkpoint bytes; decoded output and
container JSON budgets also apply. Original JSON number spelling is preserved
so restored results do not change downstream TaskIR identity. Later authorized
claims must match the parent, envelope, node and exact invocation digest.
Corruption, changed input and failed commits cannot become skip-on-error success.
Five fresh physical solve/diagnostic/scoring chains survive actual BEAM loss in
fifteen Agent executions without repeating solved nodes. Oversize output remains
uncached under the original replay policy, and ordinary terminal states prune
checkpoint payloads. Local ancestors still replay; unknown dispatches, side
effects, Agent-cache loss for native results and broad durable/scale qualification
remain outside this bounded artifact-reuse mechanism.

The [bounded branch-decision follow-up](../reports/workflow-branch-checkpoints-20261011.md)
retains the first successful condition decision before publishing its selected
port. Later owned claims reuse the decision only for the same parent, envelope,
node and lossless input/context digest; replayed upstream drift blocks before
either branch can dispatch, even with skip-on-error. The private ledger stores
no input/output payload, is bounded to 64 entries and 64 KiB per parent, does not
evict older decisions, and is pruned on ordinary terminal transitions. Two actual
BEAM restarts preserve true/false routes through thermal/electrostatic native
diagnosis and scoring in six Agent executions. Local transforms and joins still
rebuild, unretained conditions use the original replay policy, and ambiguous
dispatches or side-effect restrictions still deny recovery. This is bounded
branch consistency, not general durable continuation or authenticated provenance.

The [workflow input-binding follow-up](../reports/workflow-input-bindings-20261011.md)
rejects additional connected inputs to single-payload nodes in both Rust and
Orchestra, rather than executing only the first while claiming all as sources.
Input nodes cannot ignore upstream edges. Named multi-input composition,
first-available priority, spare declared ports and per-port output lineage remain
supported. The gate applies before synchronous execution, async job admission
and restored-graph checkpoint lookup; skip-on-error cannot bypass it. Two actual
BEAM restart rounds still complete true/false thermal/electrostatic routes in six
Agent executions. This tightens binding consistency without new snapshots,
general durable continuation or independent numerical qualification.

The [consumed-path recovery follow-up](../reports/workflow-consumed-path-recovery-20261011.md)
also binds retained solve results, condition decisions and native TaskIR receipts
to the actual consumed source path through Orchestra's rebuilt local nodes.
Equal final input values cannot attach an old successful execution to a different
fallback path. Each published artifact carries one ephemeral recursive digest;
first-available joins include only the selected source, so changes to unused
alternatives do not invalidate the same path. Computed bindings override caller
values, preserve routing authority and add no persistent scheduler snapshots or
artifact copies. Old unbound recovery evidence cannot be silently upgraded: a
matching node with a different invocation requires explicit fresh submission.
Controlled application restarts reproduce and reject source switches; two actual
Agent/BEAM restart rounds confirm unchanged-route recovery in six executions.
This is consumed-path consistency, not authenticated or complete data provenance,
general durable continuation, exactly-once execution or scale qualification.

The [native invocation precision follow-up](../reports/workflow-native-invocation-precision-20261011.md)
closes a separate numeric-alias path in Orchestra's native diagnostic/scoring
adapter. TaskIR v1 fixed-decimal encoding can assign the same digest to different
tiny input or config values. A computed lossless JSON invocation digest now
participates in native task identity without changing v1 encoding or rounding
the payload. Caller bindings are overwritten; changed or previously unbound
invocations block before receipt retrieval or redispatch. Same-name thermal
tasks distinguish tiny physical inputs, and five actual-domain restart rounds
retain solved outputs and original receipts without repeating work. This is an
adapter-owned recovery guard, not independently recomputed Agent integrity or
lossless TaskIR identity for all SDK/package authors. That broader protocol
scope remains an explicit open tensor requirement.

The [typed TaskIR digest follow-up](../reports/task-ir-typed-digest-20261011.md)
adds explicit `typed-ieee754-v2` encoding selection without changing TaskIR
structure or descriptor digests. Rust and Elixir authors hash typed binary64
values and bounded integers incrementally; control plane, Rust SDK and Agent
independently verify the selected digest. Shared vectors and a 1,024-value bit
corpus cover exact JSON round trips, with actual HTTP/Agent tests across five
diagnostic domains and both authors. Tiny drift, unknown encoding and marker
removal fail before computation. This is verified opt-in support, not default
rollout: installed typed workflow recovery, distributed package hosts and
installed-platform acceptance remain open. No old record is resealed.

The [typed routing follow-up](../reports/task-ir-typed-routing-20261011.md)
requires an explicit Agent encoding capability and RPC method, excludes old or
unprofiled fallbacks, and preserves derived requirements through the generic
RPC entry point. Ten real Rust/Elixir tasks execute through HTTP across five
diagnostic domains. An Orchestra restart recovers their original receipts
without execution; an Agent restart loses receipts and returns unknown outcomes
without replay or publication. This is source contract/read-only recovery
evidence, not default adoption, authenticated discovery or installed qualification.

The [typed workflow recovery follow-up](../reports/workflow-typed-execution-recovery-20261011.md)
adds explicit `defaults.native_task_digest_encoding` selection to native
diagnostic/scoring workflows, including typed Rust authoring. New-codec jobs
store one lossless execution JSON string rather than normalized numeric maps;
legacy jobs and default encoding are unchanged. Five solved physical-domain
chains retain solve outputs, branch decisions and original native receipts
across real Orchestra restarts with no repeated completed work. Scoring task
identity remains the same, including integral floats, signed zero and tiny
configuration values. A raw thermal input retains the same numeric types, while
a lost Agent receipt blocks without replay or publication. SQLite and JSON
regressions pass. The [PostgreSQL follow-up](../reports/workflow-typed-postgres-recovery-20261011.md)
adds seven source macOS workflow rounds with a remote JSONB store, checkpoint
rollback and selected-pool lease fencing. Four read APIs contain storage outages
and return unchanged data after restoration. Installed/remote-Agent, default
cold-start and package-host acceptance remain open; this does not qualify the
separate revision-two PWDT lifecycle. Terminal pruning retains no second payload archive.

The [collection outage follow-up](../reports/analysis-collection-outage-20261011.md)
extends read containment to job/result lists, workload discovery, database export
and project-bundle download. Known store failure returns an explicit unavailable
error, not empty data or a partial attachment; restoration reads unchanged records.
Successful response shapes, genuine empty collections, authorization-before-read,
project 404s and private recovery exclusion remain intact. This is source service
acceptance, not installed-GUI qualification.

The [SQL export snapshot follow-up](../reports/analysis-export-snapshot-20261011.md)
puts workload discovery, database export and project-bundle export inside one
SQL read transaction. Concurrent model/version edits, project removal, joint
job/result deletion and security-event publication cannot mix record generations.
PostgreSQL uses read-only repeatable read; SQLite uses a deferred WAL snapshot.
Ambient transactions are rejected rather than silently weakening isolation.
External artifacts and full backup remain separate.

The [JSON read consistency follow-up](../reports/analysis-memory-snapshot-20261011.md)
adds generation-validated views across the live project library, shared analysis
state and security-event store. The same three exports capture each accessed
store once per attempt, verify the captures coexisted, and discard conflicting
reads with at most three attempts. Sustained conflicts or store loss return the
unavailable error without partial downloads; restored analysis reads retain the
original records. ABA changes and restarts invalidate tokens; in-context writes
are rejected rather than retried. No new processes, persistent snapshots or
cross-file durability guarantees are introduced. The JSON library remains volatile.

The [independent history follow-up](../reports/history-read-reliability-20261011.md)
extends failure containment to project/model/version reads, model/version
checkpoint reconciliation and security-event history/JSON/CSV export. Known
storage failure returns opaque 503 JSON; real empty lists, absent records,
checkpoint `no-store`, filtering and authorization keep their existing behavior.
SQL project/model details and receipts read one generation per call, reusing only
an owned snapshot on the same dynamic repository. Nested query failures explicitly
roll back with the original error, including PostgreSQL aborted transactions.
Version and security query failures cannot publish partial composite exports.
Single-store memory reads add no full-store capture or persistent history, and
version-context failure cannot authorize task creation. Separate HTTP requests,
installed GUIs and large-history performance remain separate acceptance work.

The [Workbench security-history follow-up](../reports/workbench-security-history-recovery-20261011.md)
validates complete event responses before publication and preserves known records
on same-filter refresh failure. Pending/unavailable reads cannot claim empty
history or zero-data facets. Filter changes invalidate older requests before paint;
all overlapping success/failure combinations retain the latest read's records and
recovery feedback. Explicit GUI and PWDT refresh never replay writes or submissions.
Fourteen new Chromium controls and the 56-case recovery selection pass using real
client code with isolated mocked services. This is verified source client recovery,
not installed-WebView, remote-outage or scale acceptance.

The [Workbench result-history follow-up](../reports/workbench-result-history-recovery-20261011.md)
validates complete result lists and preserves known records, selection and editing
on read failure. Pending/unavailable reads do not claim empty history or a current
record count. The local Results refresh action and runtime Retry All both recover
reads without replaying computations or writes; only the latest request publishes
records or clears its warning. Successful read recovery also preserves unsaved
drafts for the same record, while explicit record selection loads verified data.
Confirmed save/delete and failed follow-up reads have separate feedback; confirmed
deletion removes its local record without hiding other results. This is source
client recovery with isolated mocked services, not installed or remote acceptance.

The [Workbench job-editing follow-up](../reports/workbench-job-draft-recovery-20261011.md)
extends draft protection to task metadata. Each dirty message/project/version/case
field survives verified refresh or read recovery while untouched fields follow
the latest values. Explicit record selection loads that record; authoritative
empty history clears selection. A slow save cannot replace later typing on the
same job or another selected job. Reconciled saved fields become clean again;
GUI Retry All and PWDT recovery do not replay writes. Job and result editing share
a small in-memory reconciliation rule, not a persistent draft cache. Installed
WebViews, backend-authority transitions and multi-user conflict handling remain
separate acceptance work.

The [Workbench job-record follow-up](../reports/workbench-job-record-recovery-20261011.md)
separates mutation confirmation from follow-up reads. Optional version clearing
sends explicit null; required project/case clearing is rejected rather than
silently omitted. Shared backend validation also rejects whitespace-only required
identities. Workbench validates job identity and lifecycle, plus explicit deletion
confirmation, before changing local records. Confirmed deletion removes only its
job/result pair and fences pre-confirmation reads; other selections and editing
survive. Job and result refresh start independently, and failed reads cannot
revoke a confirmed write. Ambiguous responses require read reconciliation, not
automatic write replay. Source Chromium controls with mocked services and real
JSON/SQLite API tests remain distinct from installed or remote acceptance.

The [Workbench result-record follow-up](../reports/workbench-result-record-recovery-20261011.md)
requires a matching result identity and object payload for write confirmation;
deletion additionally requires boolean true. Malformed success responses preserve
editing and do not start follow-up reads or replay writes. Save/export reject
nonobject drafts and numeric overflow before any write or download. Backend and
download failures are not blamed on valid draft JSON, and download activation
releases its temporary object URL even on failure. Export uses the working draft,
not necessarily the saved result. Slow-response and stale-read controls protect
other selection and newer typing. Real JSON/SQLite result APIs remain separate
from mocked-service source Chromium, installed WebViews and remote acceptance.

The [Workbench library-record follow-up](../reports/workbench-library-record-recovery-20261011.md)
requires a matching project/model/version identity and usable record before ordinary
writes can report success or refresh lists. Library deletion confirms the removed
record, not a separate boolean flag. Confirmed deletions remove only their local
records and invalidate older reads before any follow-up refresh; failed or delayed
reads cannot resurrect them. Deferred model/version lists filter against current
record identities, so a pending GUI operation cannot keep deleted rows visible
while catalog reads wait; new rows may still render later. The live modeling copy
and unrelated selection remain
intact, and late feedback cannot replace a newer workspace's message. Version and
catalog refresh start independently. Removing the latest version clears its local
latest pointer until a verified read identifies the replacement. GUI/PWDT read
recovery never replays ordinary writes; existing idempotent checkpoint saves are
unchanged. Source-browser controls with mocked services and real JSON/SQLite API
tests do not qualify installed WebViews, remote outages or ambiguous-write outcome
reconciliation.

The [Workbench task/result write follow-up](../reports/workbench-admin-record-flight-20261011.md)
adds one session-owned in-flight reservation per parent job ID to administrative
GUI Save/Delete. It starts before the async React transition and lasts through
follow-up reads. Duplicate or conflicting callbacks are not queued; paired-record
views and sidebar remount retain protection. Other records, draft editing,
navigation and result export remain available. Deferred task/result lists reuse
the current-identity membership filter so confirmed deletion cannot leave obsolete
rows visible while reads wait. This adds no persistent queue, backup or snapshot,
and does not establish cross-client serialization, late-message ownership,
backend-authority transitions or installed WebView acceptance.

The [Workbench administrative feedback follow-up](../reports/workbench-admin-feedback-recovery-20261011.md)
adds workspace revision ownership to Job/Result Save/Delete messages. Accepted
intent is captured before its queued write; duplicate callbacks cannot replace
it. New messages, navigation, manual selection and disposal invalidate old
feedback without cancelling confirmed writes or same-backend read recovery.
Effective endpoint URL and governed header comparison prevents old write replies
from removing same-ID records or starting reads on a newly selected backend.
Job/result reads discard old-authority replies even without a newer read. This is
source-client equality fencing, not continuous authority-history tracking,
authenticated deployment, ambiguous-write reconciliation or installed acceptance.

The [Workbench record-source follow-up](../reports/workbench-admin-record-source-20261012.md)
blocks task/result Save/Delete when the selected collection and working draft do
not share a verified current source. Retained old-backend data cannot authorize
a new write to a same-ID record elsewhere. Explicit GUI/PWDT refresh replaces
prior-source fields only after a valid new-source read; same-source refresh keeps
dirty editing. Weak metadata is per collection and stays outside serialized data.
The four-write warning and read-only refresh are localized for four built-in and
thirty catalogued languages. Unreadable connection settings display as unavailable
without crashing the retained-record GUI; HTTP rejects the unresolved target
instead of silently falling back to another backend. This is client admission,
not backend authorization,
signed provenance, general storage-fault recovery or installed acceptance.

The [selected-task cancellation follow-up](../reports/workbench-admin-job-cancellation-20261012.md)
closes the raw System/Data cancellation branch. It checks record source and a
matching valid cancelled receipt, shares the parent Save/Delete reservation
through history recovery, and suppresses old feedback or reads after a backend
change. A failed follow-up read does not revoke confirmed cancellation or replay
the request. Historical records never clear a live preview merely by matching
its job ID; the current observer independently polls backend status. This is
source-client recovery, not a cross-client lock, unified current-job cancel flight
or authenticated deployment acceptance.

The [linked-context recovery follow-up](../reports/workbench-linked-context-recovery-20261012.md)
unifies GUI and PWDT project/version/Apply Context routing. It requires current
task provenance and actual result membership for result links, checks linked
version/project association before loading geometry, and prevents superseded
payload/list reads from continuing dependent saves. Project-only navigation
preserves geometry but clears saved-model/version bindings instead of attaching
an unloaded model. Weak project-catalog source metadata survives confirmed
deletion transforms, and catalog/list reads discard obsolete backend replies.
This is bounded source-client recovery, not authenticated or installed acceptance.

The [saved navigation follow-up](../reports/workbench-saved-navigation-recovery-20261012.md)
extends source admission to ordinary project selection and direct saved-model/version
opening. GUI project selection now uses the same action as PWDT; explicit selection
detaches saved bindings, even for a reused project ID, while preserving geometry.
Canonical catalog membership and exact returned parent identities are checked before
loading, and version history retains known rows on malformed reads. Metadata/name
edits supersede older loads and errors. These are source-client controls, not
authenticated provenance, continuous authority history or installed acceptance.

The [Library write source follow-up](../reports/workbench-library-write-source-20261012.md)
guards GUI/PWDT project updates/deletion, model save/save-as/deletion and version
rename/deletion before dispatch. Verified canonical membership and current
selection are required. Old-backend receipts cannot prune same-ID new-backend
records or begin new-backend recovery reads; confirmed original writes remain
context-changed instead of being replayed. Version history now reaches the PWDT
gate canonically. This is client admission, not authorization, a revision lock
or proof that working geometry originated from the selected backend.

The [Library write coordination follow-up](../reports/workbench-library-mutation-flight-20261012.md)
adds a shared GUI/PWDT reservation for the same project or model, including
post-write catalog and version reads. Project mutations exclude their model
children; version mutations share their parent model's reservation. Conflicts
reject without queuing, merging success or advancing the first write's intent.
Completion or failure releases the reservation for an explicit new request;
different models, projects and client backend scopes remain independent. This
is one mounted Workbench context's coordination, not a backend, cross-window
or durable lock. Import integration is covered below; checkpoint-journal actions remain separate.

The [project import source follow-up](../reports/workbench-project-import-source-20261012.md)
captures the import's effective API and authentication scope before parsing.
Changed scope or a disposed owner stops subsequent project/model/version writes,
local preset/Store persistence, recovery reads and late feedback. Confirmed records
remain in the original backend. Same-source navigation still lets an accepted
import finish while preserving the newer workspace. Import now participates in
the existing GUI busy state through final version-history recovery, after synchronously
handing off the remapped project/model/version context. Success feedback waits for
that read; network idle alone does not identify completion. Import transactions
and resumption remain separate work.

The [project import admission follow-up](../reports/workbench-project-import-admission-20261012.md)
reuses that shared registry: one import per Workbench context and effective API
scope is admitted before parsing or changing intent. Overlapping file selections
reject with localized pending guidance, without queuing or borrowing success.
After creation, the new project and its model/version writes stay reserved through
final recovery, including when the user discovers the partial project via refresh.
Unrelated project editing, manual project creation and other backend scopes remain
available. Completion or failure releases both reservations for an explicit later
action. This is not persistent deduplication, cross-window locking or exactly-once
import; an explicit later re-import still creates a new project.

The [project creation receipt foundation](../reports/project-create-receipts-20261012.md)
adds opt-in `request_id` support to project creation. Same identity and normalized
content return the original receipt; changed content and deleted results reject
without creating another project. SQLite commits project and receipt atomically,
including concurrent connections, and retains the receipt across database reopen.
The data ledger advances to revision 3 through explicit candidate upgrade only;
revision-1/2 recipes and snapshot verification remain supported. No installed
database was modified. GUI import still needs durable identity allocation and
partial-import reconciliation, so this is not yet end-to-end import recovery.
The three diagnostic-task fixture failures from that sweep are addressed by the
[native thermal catalog follow-up](../reports/thermal-catalog-native-recovery-20261012.md).
Actual Rust diagnostic execution also exposed missing legacy metrics and unsafe
partial guard/benchmark decisions. Native metrics now align; missing required
evidence fails closed. Nine live diagnostic catalog controls pass, including stale
receipts, pending acknowledgements, missing flux and explicit fresh-job recovery.
The expanded service selection passes 550 tests with 47 conditional skips; its
project-receipt concurrency fixture now uses runtime WAL, not offline DELETE mode.
Solver frames in these catalog tests remain fixtures, not fresh numerical solves.

The [cross-domain decision follow-up](../reports/domain-decision-integrity-20261012.md)
extends fail-closed admission to structural, acoustic, modal, dynamic, transport,
electrostatic, magnetostatic and CFD guard/pair decisions. Missing required metrics,
malformed declarations, explicit nonconvergence, invalid choices and colliding
candidate labels cannot publish partial success. The 99 new controls include 64
small asynchronous jobs that reject bad evidence and recover through explicit fresh
submission without rewriting the old failure. Expanded service tests pass 1,466
with 71 conditional skips. This is not numerical or live-Agent qualification.

The [checked metric and CFD follow-up](../reports/checked-metric-cfd-recovery-20261012.md)
closes the selected-alias and partial-sample gaps in service decision resolution.
Corrupt earlier aliases, frequency/mode/transient rows and required CFD samples
cannot become favorable fallback values or fabricated zero summaries. CFD peaks
and means now resolve the same aliases, with stable means and checked totals/spans.
The expanded selection passes 1,518 tests with 71 conditional skips; the complete
API decision suite checks 128 rejected/corrected jobs across eight domains without
rewriting old failures. Existing native integrity controls pass 22 tests. These
remain controlled-data source tests, not new physical solves, installed acceptance
or general durable recovery. Other legacy raw diagnostic reducers and thermal
decision admission still require review.

The [electromagnetic diagnostic follow-up](../reports/electromagnetic-diagnostic-integrity-20261012.md)
replaces the permissive static electric/magnetic service reducers with shared
checked sample admission. Entirely absent optional groups remain absent, while
partial groups, invalid aliases, explicit nonconvergence and missing configured
fields cannot publish diagnostics. Native and service charge-density source
selection now agree for nodes/elements; no silent collection fallback remains.
Thirty-two controlled graph runs check failure isolation and corrected fresh runs
under both fail/skip policies. Expanded service tests pass 1,556 with 71 conditional
skips; the native diagnostic selection passes 69. These are controlled-data source
checks, not fresh physical solves, live Agent or durable recovery qualification.
Thermal/thermo raw reductions are covered by the subsequent follow-up below;
service quality scoring remains separate work.

The [thermal diagnostic follow-up](../reports/thermal-diagnostic-integrity-20261012.md)
removes permissive thermal/thermo service reductions. Missing selected samples,
invalid magnitudes, malformed configuration and explicit nonconvergence cannot
be hidden by healthy neighboring data. Component stress/strain peaks now compare
all axes; scalar strain follows the native signed-maximum contract rather than
the old service absolute maximum. Native displacement config honors legacy names
without falling through invalid primary mappings. The expanded service selection
passes 1,587 tests with 71 conditional skips, including 32 controlled graph runs
for rejected evidence and corrected fresh runs under both error policies.
Native thermo integrity passes 22 tests. These are raw-data contract and bounded
fresh-run recovery checks, not new physical solves or automatic durable recovery.
The subsequent quality/decision follow-up below addresses those admission gaps.

The [quality and thermal decision follow-up](../reports/quality-and-thermal-decision-integrity-20261012.md)
aligns all nine service domain scorers with strict native term/configuration
admission. Invalid or repeated selections, damaged metrics, explicit
nonconvergence and arithmetic overflow cannot publish scores. Truly absent
metrics still produce a blocking assessment; zero weights and small positive
targets remain explicit. Thermal guards/pairs now use shared decision checks
instead of permissive defaults, with field-context errors and unchanged normal
scoring formulas. Expanded service tests pass 1,693 with 71 conditional skips.
The quality graph suite exercises 162 controlled runs under fail/skip; the full
nine-domain API decision suite covers 144 rejected/corrected jobs. These are
source controlled-data checks, not fresh physics, installed acceptance or
automatic durable continuation. The bounded composition/ranking follow-up below
addresses independently supplied quality evidence.

The [objective and ranking follow-up](../reports/quality-objective-ranking-recovery-20261012.md)
rejects malformed summaries and configuration without filtering or defaulting,
preserves blockers, and accounts for every rejected candidate. Partial rankings
cannot stop research merely because the surviving best score meets the target;
unready low-score candidates also cannot stop. Next-round checks retain actionable
failure fields and distinguish completed jobs from complete research decisions.
76 new controls include 16 direct graph executions and eight actual asynchronous
service jobs using controlled summaries. The expanded suite passes 1,769 tests
with 71 conditional skips; 53 existing native quality tests pass unchanged.
This is explicit fresh-submission recovery, not physical accuracy or automatic
durable continuation. The bounded sweep/batch follow-up below extends admission
to the subsequent pre-execution chain.

The [sweep and execution-batch follow-up](../reports/quality-sweep-batch-recovery-20261012.md)
rejects malformed axes, counts, paths, case identities and configuration without
silent filtering. Budget blocks survive downstream limit changes; stopped or
incomplete research cannot issue executable batches. Mixed-radix expansion keeps
metadata and avoids intermediate Cartesian model lists. Batch routing hints now
serialize as JSON objects while verified TaskIR remains execution authority.
56 new controls include 14 direct graph runs and three asynchronous service jobs.
Exported batches round-trip through JSON and public batch preparation/execution;
six actual local thermal-shock assessments match independent formulas. Expanded
service tests pass 1,872 with 71 conditional skips; 75 existing native sweep tests
pass unchanged. These are bounded local analytic calculations and explicit fresh
submissions, not FEM accuracy, live Agent or automatic durable recovery evidence.
The result-collection follow-up below extends the service-side evidence.

The [sweep result-selection follow-up](../reports/sweep-result-selection-recovery-20261012.md)
removes positional fallback after keyed result misses, discards stale summaries
and preserves missing/rejected/unmatched diagnostics. Incomplete joins and metric
tables cannot enter scoring. Hard feasibility limits take precedence over score;
all-infeasible evaluations retain diagnostics without returning a best candidate.
56 new tests cover 29 direct graph runs and 16 asynchronous service jobs. Two
actual local batches supply four analytic assessments, shuffled before result
matching and independently checked before selection. Expanded service tests pass
1,928 with 71 conditional skips; 62 native sweep reference tests pass unchanged.
This is explicit fresh submission, not authenticated attempt provenance, physical
accuracy or durable continuation. The bounded native follow-up below extends
the scoring and score-to-quality mapping checks.

The [native sweep selection follow-up](../reports/native-sweep-selection-recovery-20261012.md)
rejects malformed, incomplete and contradictory scoring evidence before native
selection. Hard feasibility outranks score; all-infeasible results retain their
blockers and replan rather than stop. Positive objectives require an explicit
fixed mapping reference instead of becoming negative quality costs or being
rebased against the batch winner. 18 new Rust tests cover fourteen controlled
native graph executions and 100 fractional-weight JSON exchanges; 25 shared
scoring fixtures also run independently in Elixir. Expanded service regressions
pass 1,953 with 71 conditional skips; the full native Engine suite passes 677
with one ignored. These are source controlled-data checks,
not live Agent, physical accuracy, installed acceptance or durable continuation.
The collection follow-up below addresses identified native join/summary gaps;
broader quality admission and authenticated provenance remain separate work.

The [native collection follow-up](../reports/native-sweep-collection-recovery-20261012.md)
checks final convergence before selected-field reduction can hide it, rejects
contradictory incomplete-summary evidence and aligns reserved metric fields.
Both runtimes now prevent provenance fields from being used as summary
containers. Fresh native joins remove declared old custom summaries without
deep-cloning the discarded trees. Five new Rust tests and 43 Elixir tests include
41 shared fixture cases, sixteen controlled native graphs and four controlled
service graphs. The full native Engine suite passes 682 with one ignored;
expanded service regressions pass 1,996 with 71 conditional skips.
These are source contract and explicit fresh-run controls, not new physical
calculations, live Agent, installed, remote or durable-recovery qualification.
The following quality-decision audit extends native admission evidence.

The [native quality-decision follow-up](../reports/native-quality-decision-recovery-20261012.md)
requires same-domain score companions, rejects malformed present validation
fields and candidate identities, and validates all ranking entries, rejection
records, counts and best-candidate claims before the next round. Inner objective
blockers and readiness cannot be hidden by outer fields. Both runtimes pass 49
shared fixture cases; thirty new controlled native graphs distinguish malformed
input rejection, incomplete-evaluation pause and valid repair replanning, with
corrected fresh graphs expanding four cases. The full Engine suite passes 687
with one ignored; expanded service regressions pass 2,045 with 71 conditional
skips. These are source contract and explicit fresh-run controls, not physical
accuracy, live Agent or durable recovery. The next audit covers ranking order
and score-copy consistency; maturity grades are unchanged.

The [ranking consistency follow-up](../reports/quality-ranking-consistency-recovery-20261012.md)
checks ready-first/score/identity order without silently re-sorting, validates
optional one-based rank positions and requires present row/objective scores to
agree. Every nested objective is checked, including lower-ranked rows. Both
runtimes pass 31 shared fixture cases and permutation/JSON-exchange controls;
sixteen native and sixteen service graph executions reject incoherent decisions
and recover through corrected fresh inputs. The full Engine suite passes 690
with one ignored; expanded service regressions pass 2,078 with 71 conditional
skips. Canonical ordering is not independently recomputed physical optimality.
Objective-term recomputation, conflicting coupled-readiness representations,
downstream admission and durable recovery remain separate work.

The [coupled readiness follow-up](../reports/coupled-readiness-admission-recovery-20261012.md)
closes the next-round report masking gap. Null aliases cannot hide another report;
conflicting copies and malformed reports reject, including lower-ranked inputs.
False readiness or blocking/missing domains require repair, while warnings require
review rather than a successful stop. Service plans now retain repair strategy
and focus through expansion. Both runtimes pass 56 shared cases and 125 alias
placements each; twenty native graphs and sixteen service graphs plus four service
transform chains check rejection and explicit fresh recovery. The Engine suite
passes 695 with one ignored; service regressions pass 2,138 with 71 skips.
These are controlled source checks, not physical or durable-recovery qualification.
Direct request admission is addressed by the following scoped check; producer-side
readiness retention remains further work.

The [direct sweep request follow-up](../reports/quality-sweep-request-admission-recovery-20261012.md)
checks hint aliases and four readiness locations, including the seed objective,
before stopped/incomplete shortcuts. Blocked evidence requires explicit repair
intent; warning evidence requires review. Nonempty rejected-candidate lists also
disable native planning. Both runtimes pass 74 shared cases and 625 alias placements
each; 24 graph executions per runtime cover rejected requests and corrected fresh
repair runs. The Engine suite passes 699 with one ignored; service regressions pass
2,214 with 71 conditional skips. These are controlled source checks, not physical,
installed or durable-recovery qualification.
Direct plan materialization is addressed by the next scoped follow-up;
producer-side readiness retention remains open.

The [direct plan follow-up](../reports/quality-sweep-plan-admission-recovery-20261012.md)
checks enabled/action/readiness consistency and repair metadata before materializing
an existing plan. Explicit stale totals, per-axis counts and deferred-count overflow
reject rather than being trusted or silently normalized. Both runtimes pass 74
shared cases and 21 generated-plan mutations each; 24 graph executions per runtime
verify rejection and corrected fresh repair, and deferred billion-sample controls
expand no cases. Engine regressions pass 703 with one ignored; service regressions
pass 2,291 with 71 conditional skips. These are local controlled checks, not physical
or durable-recovery qualification. Direct expansion quality metadata is addressed
by the next scoped follow-up; producer-side readiness retention remains open.

The [direct expansion follow-up](../reports/quality-sweep-expansion-admission-recovery-20261012.md)
validates supplied quality metadata before generating cases, including raw calls
that bypass materialization. Payload and configuration aliases cannot erase or
contradict repair evidence; ordinary non-quality custom metadata keeps its existing
behavior. Both runtimes pass 70 shared cases, 125 alias placements and 18 generated
expansion mutations each. Each also completes 24 controlled graph executions:
rejection stops dependents, while corrected fresh runs preserve metadata through
native cases and service task exports. Engine regressions pass 707 with one
ignored; service regressions pass 2,364 with 71 conditional skips. These are local
source checks, not physical or durable-recovery qualification. Count/deferred-axis
consistency is addressed by the next follow-up; batch admission remains open.

The [expansion count follow-up](../reports/sweep-expansion-count-recovery-20261012.md)
checks supplied axis counts and count mirrors before stopped/budget-blocked returns.
Deferred axes cannot also carry concrete values, and changing a downstream limit
cannot release an upstream block. Both runtimes pass 85 shared cases and 125 count
placements each. Each also completes 32 controlled graph executions with rejected
counts and corrected fresh task exports. Generated-output mutations and a
billion-sample deferred control exercise the same boundary without allocating
samples. Engine regressions pass 711 with one ignored; service regressions pass
2,452 with 71 conditional skips. This is local source admission evidence, not
scale, physical or durable-recovery qualification. Caller-authored execution
batches are addressed for quality evidence and bounded structural admission by the
next scoped follow-ups; producer-side readiness retention remains separate work.

The [direct batch quality follow-up](../reports/quality-execution-batch-recovery-20261012.md)
checks evidence on the batch, each candidate and their metadata before compilation,
preparation or any execution dispatch. Twenty-six invalid variants cover 130
placements through the composer, executor and public envelope helpers. Explicit
repair/review controls retain evidence and complete local analytic tasks; 32
controlled fail/skip graphs check rejection and corrected fresh runs. Process-local
call tracing verifies zero task dispatch for a rejected first or last candidate,
with strict mode both on and off. Ordinary per-task calculation failures still
produce isolated failure receipts and targeted retry plans. Top-level quality
evidence now participates in the batch digest, without changing numerical TaskIR.
This is control-plane source evidence, not new native Engine, live Agent, physical,
installed or durable-recovery qualification. Expanded service regressions pass
2,487 with 71 conditional skips; an additional asynchronous API pair preserves
failed history and verifies corrected fresh execution. The next follow-up addresses
the direct-batch structural boundary without relabeling these earlier results.

The [batch structure and recovery identity follow-up](../reports/operator-task-batch-structure-recovery-20261012.md)
adds shared pre-dispatch validation for malformed task containers and unique case
and actual TaskIR IDs. Checkpoint entrypoints reject malformed summaries, foreign
or duplicate recovery targets and unsupported or independently rewritten policies.
Five public batch routes return actionable HTTP 422 errors; process-local tracing
confirms that a late structural or identity failure cannot dispatch earlier tasks.
Empty batches and optional-header shorthand remain supported, while ordinary
per-task digest/semantic failures stay isolated. Expanded service regressions pass
2,512 with 71 conditional skips. This is control-plane source evidence, not new
native Engine, physical, installed or durable-recovery qualification. Full summary
counter coherence and receipt provenance remain separate from structural validity;
the next follow-up adds bounded preparation/completion checks.

The [checkpoint evidence follow-up](../reports/operator-task-checkpoint-evidence-20261012.md)
requires batch-bound preparation rows with matching counters, identities and failure
reasons. Checkpoints retain compact task identity evidence, not numerical results,
and recheck completion markers and unresolved targets when read. Missing records
remain targets even alongside a known task failure. Foreign batch/phase summaries,
including error-only summaries, require inspection rather than direct retry.
Editing a completion flag and rehashing does not bypass the consistency checks.
Older preparation/execution summaries without compact evidence require explicit
regeneration from original records; nothing silently reruns. Expanded service
regressions pass 2,528 with 71 conditional skips. This is local control-plane
consistency, not authenticated provenance or physical qualification.

The [failure recovery follow-up](../reports/operator-task-batch-failure-recovery-20261012.md)
binds failed/blocked rows to their tasks, checks Agent inner/outer failure mirrors,
and reconciles disjoint success/failure/blocker/skipped partitions with counters.
Compact receipt fingerprints are rechecked after checkpoint read, even after a
caller recomputes the checksum. Nonretryable failures require repair, not direct
retry; unsafe-continuation receipts block progression. Plans separate explicit
retry candidates from repair targets and always deny automatic replay. Unverified
execution stays diagnostic, with no borrowed preparation or unchecked failure
actions. Whitespace-only actions no longer make a generated checkpoint unreadable.
Full result payloads are not copied. The controlled Agent protocol covers all four
retryability/continuation combinations, not a live Engine or durable restart.
Expanded service regressions pass 2,541 with 71 conditional skips; module maturity
and release qualification remain unchanged.

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

See `releases/snapshots/3.6.0.json`. Package, update, language-pack and book
targets are aligned, without changing protocol/schema versions, translation
content or historical grades. The installed baseline remains 3.4.0. The tensor
continues to have four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps;
release qualification remains blocked. The 3.6.x mainline continues reliable
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
