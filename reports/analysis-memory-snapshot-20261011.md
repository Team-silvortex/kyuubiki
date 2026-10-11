# JSON export read consistency

Project-bundle export, database export and workload discovery now validate one
coherent view across the live lightweight stores they access. Concurrent edits
cannot silently mix record generations. Conflicting reads are discarded with
at most three total attempts; sustained conflict or store loss returns an opaque
unavailable response without a partial attachment.

This is in-process read consistency, not a new durable snapshot format.
The lightweight project library remains volatile. Analysis and security persistence
retain their existing formats and restart behavior.

## Reproduced failures

Three deterministic cases failed before the production change: model/version
publication changed the latest pointer between reads; joint job/result deletion
left a fetched job alongside a missing result; and updates to separate stores
combined an old project record with a later result. Actual write calls ran on
another task between reads. The controls use synthetic stored records, not
solver-accuracy or scientific-qualification evidence.

## Generation validation

`Storage.MemorySnapshot` wraps the existing Agent start/read/write calls for
the project library, shared analysis state and security-event store. It adds no
new storage process. Each store keeps a unique reference in its process dictionary.
Every completed write attempt replaces that reference, including rejected/no-op
attempts. A new process gets a fresh reference even when it reloads identical data.
This conservative policy can invalidate a read whose visible data did not change;
it does not mistake ABA content restoration for an unchanged generation.

`ReadSnapshot.run/1` captures each accessed store's state and token on its first
read. Later reads of that store reuse the immutable captured state, with no new
Agent transfer for each result lookup. Before returning, it checks every captured
token against the live store. All captures precede those checks. If every token
is unchanged, the captured states coexisted at the last capture: each state stayed
unchanged from its own capture through its later validation. This argument relies
on the registered write wrappers and does not authorize raw Agent or system-state
mutation that bypasses them.

Changed tokens discard the projection and retry the pure read callback, up to
three attempts including the first. No write, task execution, result publication
or ambiguous dispatch is automatically retried. Writes in the read context are
refused before execution; nested reads reuse the outer context. Success, conflict,
exceptions and unavailable-store exits release the context.

Only accessed stores participate. Bundles and catalogs do not require the
security-event store; database export does. Capture telemetry includes only
the store module and attempt number, not payloads, tokens or credentials. The
captures and tokens are not persisted, and add no retained history or backup
directory. Captured state still occupies temporary request memory; this is not
a large-export memory bound or streaming implementation.

## Executed controls

Nine read-boundary cases cover concurrent model/version publication, joint
job/result deletion, separate-store updates, exact ABA content restoration,
analysis-store restart with unchanged data, the three-attempt limit, rejected
in-context writes, nested reads and one capture per accessed store. Repeated
lookups do not recapture the analysis payload, and read-only captures do not
rewrite its persisted file. Existing restart, corruption, integer/float identity
and failed-publication cases also remain in the regression selection.

Twenty-one HTTP/composite cases cover all three exports during model/version
publication, joint deletion, project deletion, sustained conflicts, library outage
and mid-read analysis outage. Conflicts return HTTP 503 with only the public
unavailable error and no attachment header. Concurrent deletion is reflected by
the later valid projection, including the bundle's genuine project-not-found 404.
Restoring the durable analysis store returns the original job/result records,
without resubmission. Security-event publication, independent security outage,
its absence from bundle/catalog dependencies and direct headless-helper failure
replies are checked separately.

These controls run the source router, not installed WebViews. They are not a
claim of new library persistence: restarting the volatile library loses its
in-memory records under its existing behavior.

## Verification

- JSON recovery/API/library selection: 692 listed, 657 passed, 35
  backend-specific cases skipped, zero failures in 10.6 seconds.
- SQLite regression selection: 989 listed, 917 passed, 72 backend-specific cases
  skipped, zero failures in 34.4 seconds. The JSON-specific controls skip here;
  existing SQL snapshot, project/version and workflow-recovery controls pass.
- Different-order JSON acceptance with seed 2718: all 30 new cases passed,
  zero failures in 0.3 seconds.
- Remote PostgreSQL regression: 69 listed, 39 passed, 30 JSON-only cases skipped,
  zero failures in 50.2 seconds. This exercises SQL snapshot isolation, outage
  recovery, the extended failure boundary and joint job/result commits. The
  database runs on the Ubuntu lab host; the control plane uses macOS source
  through a private loopback tunnel. This is not Linux control-plane qualification.
- Strict test compilation, formatting and diff checks passed. Runtime API,
  topology, module/function matrix, extension-standard, documentation inventory,
  book, project organization and version-line checks passed. The tensor and its
  self-test passed structurally: zero structural gaps, four maturity gaps,
  20 evidence-grade gaps and 15 P0 gaps. Overall Daji qualification stays blocked.

Run the focused JSON cases from `apps/web` with a fresh temporary
`KYUUBIKI_DATA_DIR` and `KYUUBIKI_STORAGE_BACKEND=json`:

```text
mix test test/kyuubiki_web/storage/memory_read_snapshot_test.exs test/kyuubiki_web/storage/memory_export_snapshot_test.exs test/kyuubiki_web/api/analysis_collection_outage_test.exs test/kyuubiki_web/storage/analysis_memory_state_test.exs --seed 0 --warnings-as-errors
```

Use only disposable data for these tests. PostgreSQL regressions require an
isolated `DATABASE_URL`; host addresses and server configuration must stay out
of Git. The temporary remote database uses 256 MiB tmpfs storage, 512 MiB memory,
two CPUs and a 128-process limit, creates no volume and binds only to loopback.
The test container, its private port forward and the local disposable JSON
directory were removed after verification. The container label query returned
no remaining test containers, the forwarded port had no listener and the local
directory was absent. No shared SSH connection or unrelated service was stopped.

## Qualification boundary

The guarantee covers these three source composite reads, participating live
stores and registered write paths in one BEAM runtime. It preserves their public
projection and private-recovery exclusion. Runtime timing fields are derived
when serialized, not frozen stored columns. Capture validation is not a
cross-process transaction, distributed snapshot or cross-file power-loss receipt.
It does not make separate store writes atomic or persist the volatile library.

These controls do not qualify standalone history. The later
[history follow-up](history-read-reliability-20261011.md) covers independent
project/model/version and security reads without adding volatile-library persistence.
Installed GUIs, external result files, Kcore artifacts,
distributed agent state, large-export memory/latency, and sustained contention
performance remain separate acceptance work. No app package, version metadata
or Git history changes.
