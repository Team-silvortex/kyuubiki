# Workbench record and draft source admission

Administrative task and result Save/Delete now require the selected record and
its working draft to share a verified, current source. Retained history from a
previous backend remains read-only for these four writes until an explicit valid
refresh. Same-source reads keep dirty editing; a verified new-source read replaces
prior-source fields even when the record ID is identical. This is source-client
protection with isolated mocked services, not backend authorization or installed
WebView acceptance.

## Reproduced cross backend write

The browser baseline loaded a task from backend A, edited its message, switched
the effective API target to backend B without refreshing and activated Save.
B received a PATCH containing `unsaved from backend A`, the old model-version
and case fields, and the same task ID. The GUI then reported a successful update.
Late-response fencing alone could not prevent this new request.

## Record and draft boundary

Validated task/result collection reads now receive an opaque current-source
predicate in a weak metadata registry. There is one entry per collection rather
than one per record; marking and lookup do not scan the list or copy its payload.
Record objects, JSON serialization, public schemas and project formats are
unchanged. Confirmed-deletion filters retain the collection's original source
instead of silently retagging the new array as the latest backend.

The editor records its source alongside the existing field baselines. Ordinary
reads of the same source retain unsaved values and update clean fields. A verified
different-source read resets all task metadata fields or the result JSON to the
new source's values. Failed or obsolete reads cannot authorize that reset. Writes
also require the selected record's source and the committed editor source to
match, so new data cannot briefly authorize an old draft while reconciliation
is pending. Unknown source metadata never authorizes a write.

The source check runs in the actual Save/Delete controllers before endpoint
capture or backend work, independently of the GUI disabled state. A missing,
stale, mismatched or unreadable source returns a localized blocked-operation
message and releases the existing write reservation without sending or replaying
a mutation. Source-inspection exceptions do not escape through the data panel's
rendering or retained-record write handlers.

Target inspection also separates UI display from HTTP admission. Unreadable
connection storage yields an unavailable descriptor for rendering instead of
throwing through React. Network resolution rejects that descriptor before fetch;
it cannot silently choose environment or same-origin fallback while a
higher-priority setting is unreadable. A valid explicit query target keeps its
normal precedence over lower-priority storage. Internal storage diagnostics are
not exposed in the generic connection error.

The data panel disables those four writes and provides an explicit read-only
task/result refresh. Its warning explains that refreshing replaces drafts from
the previous source. The guidance has translations for four built-in and thirty
catalogued languages; unknown custom languages use the explicit English fallback.
While the old result remains selected, local JSON export still uses the working
draft without writing to either backend. No draft archive or automatic backup is
introduced. Source checks for disabled controls run only while the data panel is
visible, rather than on every modeling-view update.

## Executed controls

- Final frontend unit suite: 1,405 passed, zero failures or skips in 21.2 seconds.
  The 70 added controls comprise eight metadata/admission cases, 34 locale cases,
  one unknown-language fallback, three source-aware draft cases and twenty
  action-controller cases, plus four backend-target fault/precedence cases. Flow
  composition also verifies all four source inputs reach the write handlers
  unchanged.
- Expanded source Chromium run: 176 passed, zero failures or skips in 561.0
  seconds. Its thirteen new cases cover eight cross-backend Save/Delete paths
  with optional failed refresh, four same-backend read/dirty-draft controls and
  local export from a stale result. Existing feedback, write-flight, checkpoint,
  task/result history, library-record and Data/Library action controls remain.
- Final core Chromium rerun after the connection-setting repair: 100 passed,
  zero failures or skips in 306.4 seconds. This includes all fifteen final
  source controls plus existing feedback, write-flight and task/result receipt
  recovery controls. The earlier 176-test run and this overlapping 100-test run
  are separate executions, not a single combined 276-test coverage claim.

The first 100-test run exposed two GUI failures rather than merely blocked
writes: injected target-setting read exceptions escaped rendering and React
reported a changed hook count. That run passed 98 and failed two. After fixing
the underlying target lookup, both original fault cases passed independently,
then the final 100-test rerun passed without weakening those assertions. These
two cases verify no GUI exception, zero writes while unavailable, and explicit
refresh/save recovery after restoring the same source.

The new browser suite activates actual GUI controls and uses both the source
refresh button and built-in PWDT read recovery. New-backend write counters stay
at zero until a verified read and a new explicit action. All task metadata fields
and result JSON are checked after source replacement; same-source failures keep
working values and permit an explicit save. Independent project/library and
workflow-submission counters detect unintended work.

Final static checks passed: frontend type checking, frontend sensitive-storage
guard, the 800-line source guard, repository source/document organization
(800/2000 lines, zero tracked debt), runtime API surface, module topology,
module/function matrix, module extension standard, coverage tensor and its
self-test, desktop validation self-test, documentation inventory/book, version
contracts, Rust formatting and whitespace checks. The tensor still reports four
maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps, with overall Daji
qualification blocked; successful source controls do not close those gates.
After browser completion, process inspection found no matching test services,
and the temporary-directory check found no Workbench UI fixture directories.

Run the source suite from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-admin-record-source.test.mjs
```

## Qualification boundary

Source identity follows effective API URLs and governed authentication headers,
not authenticated server identity or signed result provenance. Verified A-to-B-to-A
reads allocate a fresh source, but an unobserved hop back to an identical effective
scope is not continuous authority-history proof. Cross-client conflicts, mutable
server revisions, other administrative actions, whole history admission and
physical result validation retain their own boundaries. The setting-read fault
controls apply to retained-record rendering/writes, not general cold-start
browser-storage recovery.

Engine/Agent execution, operators, headless SDKs, backend permissions and data
schemas are unchanged. There is no persistent cache, snapshot, write retry,
production Node dependency, version bump, Git commit, app rebuild or installation
in this round. The new file is registered with the native integration runner,
desktop validation contract and coverage tensor as verified source-client
evidence, without promoting operational acceptance.
