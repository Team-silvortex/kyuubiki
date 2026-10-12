# Workbench project import admission and write protection

Project import now participates in the existing Workbench Library reservation
registry. Overlapping file selections cannot create another import chain, cancel
the accepted import's intent, or borrow its success. Once creation returns, the
new project and its child model/version mutations remain reserved through final
catalog and history recovery.

## Confirmed failures

Before this change, all 33 new direct import controls failed while the preceding
70 archive/import/persisted-load controls passed. Twenty-eight cases covered
seven asynchronous stages, the same or a different File object, and a successful
or failed first import. The second invocation parsed its file and advanced intent
instead of rejecting at admission. Five further controls showed that project
and child-model writes could run during the import's remaining work.

The final directly affected suite passed all 106 controls, including localized
pending feedback and feedback ownership after a newer intent or backend switch.
The shared reservation matrix now covers the import
slot alongside manual project creation, project writes and model writes.

## Admission rules

The import slot is reserved synchronously before the transition, parsing and
project-context begin. It is scoped to one Workbench context and the effective
API/authentication scope. A second import of any file rejects with localized
`WORKBENCH_LIBRARY_WRITE_PENDING` guidance. No file payload, hash, durable journal
or global mutex is introduced. The existing 34-locale pending guidance is reused.

After the original project creation is confirmed and its source checked, the
import also acquires that project's ordinary reservation. GUI and PWDT project
update/deletion, model save/save-as/deletion and version rename/deletion therefore
share the same exclusion path. A user discovering the partial project through a
catalog refresh cannot delete it during child creation. Ordinary manual project
creation and unrelated project writes remain independent.

Navigation is not a write. It can still change the current workspace while an
accepted import completes on the same backend; the import does not overwrite the
newer workspace. Source changes retain the continuation fence from the
[source recovery follow-up](workbench-project-import-source-20261012.md).
Both reservations release on settlement, including parse, write and read failures.
Rejected actions are not queued or replayed. An explicit subsequent import is a
new operation and creates another project, even for the same file.

## Browser controls

The first isolated Chromium run passed all 17 cases: ten duplicate-import cases
covering five remote write/read stages and success/failure, six GUI/PWDT mutation
conflicts during imported history recovery, and unrelated project editing while
child creation is held. Each duplicate case also explicitly selects the same
file after settlement and checks that exactly one new import is then created.
Two additional GUI/PWDT cases discover a partial project via catalog refresh,
attempt deletion while model creation is held, then explicitly delete it after
import completion. Both preserve the newer project-only navigation.

The browser controls inspect outgoing writes, accepted project/model/version
counts, visible busy/ready state and feedback. Recovery checks use the original
source modules with a temporary mocked backend, not copied controller logic.

## Verification

The complete frontend unit suite passed 1,735 tests with zero failures or skips
in 22.4 seconds. Relative to the preceding 1,685-test suite, this adds 36 import
admission/feedback controls, thirteen extra target-matrix combinations and one
additional cross-backend import-slot control. The shared target matrix now has
49 cases. Guidance availability covers 34 locales, not linguistic certification.

The final combined Chromium run passed 105 tests with zero failures or skips
in 316.9 seconds: 19 import-admission controls, 12 import-source/recovery controls,
22 shared Library reservation controls and 52 project-boundary controls. No
temporary Workbench browser workspace remained after the run.

Frontend type checking, browser-storage security, the source/document line audit,
documentation book/inventory, module topology/matrix/extension and runtime API
surface checks passed. The native test runner rebuilt successfully; Rust
formatting and whitespace checks passed. Desktop UI validation self-test and
tensor check/self-test passed. All 224 exact version contracts remain at 3.5.0.
Tensor structure has zero gaps; four maturity gaps, 20 evidence-grade gaps and
15 P0 gaps remain, so Daji readiness remains blocked.

## Boundaries

This is client-side coordination within one mounted Workbench context and
effective scope, not a backend authorization mechanism, cross-window/client lock,
continuous server-identity history or durable duplicate detector. Separate
windows can still import independently. Already confirmed partial records are
retained; they are not automatically deleted on failure. Import transactions,
resumption and exactly-once delivery remain separate work.

This evidence concerns contract, recovery and UX closure. It is not an installed
WebView, mobile, physical-server or numerical qualification, and does not promote
operational evidence. No production Node dependency or backup is added.
