# Frontend Test Notes

The frontend now has a lightweight native unit-test layer built on Node's
`node:test` runner with `--experimental-strip-types`.

Current safety rails include:

- `npm run build`
- `npm run typecheck`
- `npm run test:unit`
- `npm run test:unit:workflow`

Organization rules:

- Put reusable fixtures and helpers under `test/support`.
- Group tests under `test/<domain>` so workflow, installer, and hub behavior can
  grow independently.
- Keep browser and smoke coverage separate from pure unit tests; integration
  flows still live under the repo-level `tests/integration`.
- The unit-test runner registers a local `@/` alias loader, so tests may import
  real frontend source modules directly instead of copying logic into test-only
  wrappers.

## Modeling performance

The `workbench-model-batch` filter covers grouped selection, topology-preserving
arrays, load distribution, member assignment, deletion and atomic history edits.
The `workbench-model-transform` filter covers right-hand rotation, selection and
explicit pivots, scaling, reflection, grid snapping, copy identity/boundary rules,
72 rotation round trips and bounded 200k-node transforms. These are geometry
invariants and operation-count checks, not solver or interactive FPS benchmarks.
The real-browser GUI/PWDT boundary test is
`tests/integration/workbench-ui-model-batch.test.mjs` (run from the repository
root with `node --test`). Its backend is isolated and mocked; it is not a solver
accuracy or native WebView qualification. See the
[batch modeling tutorial](../../../docs/tutorial-pwdt-automation.html#batch-modeling).

`tests/integration/workbench-ui-immersive-modeling.test.mjs` exercises real Chromium
fullscreen, sidebar/dock draft transfer, legacy quick edits, multi-selection,
undo/redo, study navigation, rejected fullscreen requests and five viewport sizes.
It also covers query-to-canvas selection, fullscreen save/save-as/retry with mocked
storage, duplicate-submit prevention across tool-tab remounts, and bounded pointer /
keyboard dock resizing, cancellation and reset. Pure layout and selection tests
cover invalid dimensions and 200k-node selection without scanning connectivity.
Its storage service is mocked. `workbench-model-batch-draft` and
`workbench-script-state-controller` add draft-scope and failure-atomicity checks.
See the [fullscreen editing contract](../../../docs/tutorial-pwdt-automation.html#immersive-modeling).

Missing-API fallback, editable-form shortcuts and cold-load exit ownership are
covered separately from actual browser fullscreen. Projection tests keep default
3D presets inside the canvas and verify proportional fitting and drag inversion.
For native acceptance, run `tests/manual/pwdt-immersive-truss3d.py` in Workbench PWDT,
then follow the [installed macOS acceptance record](../../../docs/acceptance-immersive-modeling.html).
After saving the tall variant and restarting the managed runtime, reopen it and
run `tests/manual/pwdt-immersive-recovery.py`. These use real services and create a
small isolated project; they are not standalone Python/headless-SDK tests.

`tests/integration/workbench-ui-checkpoint-recovery.test.mjs` covers checkpoint
publication versus history-read failures, read-only recovery without duplicate
writes, model-scoped history, and delayed responses after navigation. The installed
backend's checkpoint endpoint already commits model metadata, geometry, latest
pointer and version together; neither GUI nor PWDT may precede it with a model
PATCH. SQLite rollback injection is covered independently in
`apps/web/test/kyuubiki_web/library/checkpoint_transaction_test.exs`.

The same browser suite aborts responses after committed Save and Save As operations,
then exercises GUI buttons and PWDT retries without duplicate versions. Unit checks
in `workbench-checkpoint-retry` cover request-key retention, concurrent submissions,
authority changes, explicit keys and bounded-cache refusal. The project-library
service also rejects malformed success envelopes without discarding the retry key.
Backend request/conflict/deletion and SQL receipt-transaction tests live in
`apps/web/test/kyuubiki_web/library/checkpoint_request_test.exs` and
`apps/web/test/kyuubiki_web/library/checkpoint_receipt_transaction_test.exs`.
These require data revision 2 for SQL stores; use disposable test databases, never
the installed application's state database.

`workbench-modeling-performance` and `workbench-truss3d-webgl` unit filters cover
immutable batch edits, linear selection/adjacency work, 200k-node bounds, scene
colors, and GPU-resource lifecycle. Run the opt-in CPU benchmark from this directory:

```sh
node --import ./test/support/register-alias-loader.mjs ./test/benchmarks/workbench-modeling.bench.ts
```

The benchmark emits a small JSON summary, never generated mesh files. Timing is
informational; deterministic operation-count tests provide the CI regression gate.
The real-browser counterpart is
`tests/integration/workbench-ui-modeling-performance.test.mjs` at repository root.
See [measured scope and limitations](../../../docs/rendering-roadmap.html#modeling-performance).
