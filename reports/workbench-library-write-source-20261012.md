# Workbench project model and version write source protection

Existing-resource writes in Library now require verified canonical catalog
membership and the current project/model/version selection before dispatch.
GUI and PWDT share one source gate for project update/deletion, model checkpoint
save/save-as/deletion and version rename/deletion. Fresh project creation does
not require an existing catalog, but its receipt remains tied to its dispatch
backend. This is client recovery against source UI and isolated mocked services,
not backend authorization or installed-app acceptance.

## Reproduced failures

The initial browser baseline captured a model DELETE on backend B from a retained
backend A GUI row, followed by a B catalog read and detached local model binding.
A second baseline accepted a PWDT project update on B using the old project
catalog and reported success. Both assertions failed before the repair and
passed after the shared source gate was connected.

An expanded regression also exposed missing canonical version history in the
PWDT controller composition. Direct GUI version writes worked while PWDT
version writes were wrongly blocked before their request. The history now passes
through the flow composition, controller and invoker, with a typed invoker
requirement and a real invoker unit control. Destructive PWDT actions also emit
independent security audit POSTs; browser fixtures distinguish those exact
requests from Library writes and recovery reads rather than claiming all
network activity is suppressed.

## Admission and receipt behavior

Project operations and model Save As require a current project catalog member.
Updating or deleting an existing model additionally requires its identified
project association. Version rename/deletion requires a verified version history
member matching the selected project, model and version; a blocked project
catalog read does not invalidate independently verified version history.
Missing, copied/unmarked, stale or unreadable collection sources and foreign
parents fail before dispatch. Disposed owners and outdated selection closures
also fail closed. GUI model serialization occurs only after this admission.

The gate captures effective API authority and source ownership before each
write. Once a backend changes, an old confirmation cannot prune or detach a
same-ID new-backend record, publish a success message, start new-backend recovery
reads or continue a dependent PWDT write. A confirmed old-backend operation is
not undone: PWDT returns its original write outcome with `contextChanged: true`,
which existing sequential automation treats as a stop condition. Old-source
GUI errors do not replace the newer workspace's feedback. Fresh project
creation follows the same authority fence without borrowing cached row IDs.

Receipt cleanup and workspace ownership remain separate. A same-source confirmed
deletion still reconciles its removed record after the user changes selection;
it never falsely restores that record just because a follow-up read fails.
Existing atomic checkpoint identity/retry and unconfirmed-receipt checks are
unchanged. Read recovery does not automatically replay a write, and this change
does not introduce a backup, archive or workspace snapshot.

## Verification

The complete frontend unit suite passed 1,586 tests with zero failures or skips
in 26.2 seconds. Relative to the preceding 1,534-test suite, it adds 52 controls:
32 source/selection/authority controls, seven real PWDT confirmation controls,
twelve GUI late-confirmation/error controls and one real invoker forwarding
control. Existing composition coverage also checks canonical history forwarding.

The browser suite covers both surfaces for all seven existing-resource write
forms, late confirmations after a verified same-ID B catalog, late failures,
new-project response fencing and explicit read-only recovery before a new edit.
Mock counters check no Library replay; independent
security audit POSTs retain their separate routing contract. Adjacent checkpoint,
Library receipt, PWDT completion and ordinary navigation controls are included
in the expanded regression.

The final expanded source Chromium run passed 125 tests with zero failures or
skips in 335.6 seconds: 40 write-source controls, 32 Library receipt/recovery
controls, 30 ordinary navigation controls, 13 PWDT completion controls and ten
checkpoint recovery controls. An intermediate 95-case run passed 80 and failed
15, exposing the PWDT history forwarding defect and a fixture assumption that
counted independent security audit POSTs as Library writes. Both were repaired
without relaxing Library write/read assertions; the final combined run is clean.

Frontend types, storage security, frontend file limits, runtime API contracts,
module topology/matrix/extension, coverage tensor and self-test, desktop
validation self-test, documentation inventory/book, repository organization,
version contracts, Rust formatting and whitespace checks passed. Source and
document limits remain 800/2000 lines with zero tracked debt; all 224 exact
version contracts match 3.5.0. The tensor retains zero structural gaps, four
maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps; overall Daji
qualification remains blocked.
No matching test service processes or Workbench UI fixture directories remained
after completion.

Run the scoped browser controls from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-library-write-source.test.mjs
```

## Qualification limits

The source gate compares client catalog metadata, selection IDs, effective URLs
and governed headers. It is not authenticated server identity, continuous
authority/membership history, a revision lock, working-payload provenance or
same-parent write exclusion. A newly verified catalog permits an explicit edit;
it does not prove that the working geometry originated from that backend.
Unobserved backend away-and-back changes and independent audit routing remain
outside this gate. Project import, database restoration, exports and other
administrative operations retain their separate boundaries.

No backend schema, persisted format, Engine/Agent, operator, headless SDK or
numerical algorithm changes. No automatic write replay, production Node
dependency, snapshot, backup, version bump, Git commit/push, application rebuild
or installation is introduced. Tensor evidence stays at bounded source-client
contract/recovery/UX grade, not operational qualification.
