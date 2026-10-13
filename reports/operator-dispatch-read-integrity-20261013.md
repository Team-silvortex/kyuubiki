# Live dispatch read integrity and recovery fencing

Date: 2026-10-13. Daji 3.6.x local source follow-up to observation sealing.
This closes a live-cache integrity gap in Orchestra: disk evidence is checked
before returning retained history or permitting native recovery. It does not add
an archive, change record versions, increase storage budgets or move computation
into the control plane. Existing uncommitted recovery repairs are preserved.

## Failure and repair

Journal lookups and workflow replay fences previously used only the records
loaded into memory. Deleting or corrupting a file, replacing it with another
valid generation or leaving an incomplete replacement did not invalidate a cached
success. Empty queries could report no retained work even after the nonempty
journal directory disappeared. A new dispatch could recreate that directory and
continue writing while historical evidence was absent.

- History lookup by task/digest and by task now checks directory membership against
  the cached attempt IDs, including empty selections. Missing files, unexpected
  files, incomplete `.next` replacements or a missing/symlinked nonempty root fail
  closed. New reservation checks that same inventory before pruning or writing.
- Selected records receive the existing bounded file/envelope/digest validation
  and must equal their cached generation. A valid but different on-disk record is
  not silently adopted. Reads perform no mkdir, chmod, replacement, timestamp
  refresh or retention renewal.
- Workflow replay fences verify all relevant records, including terminal states,
  actual-parent records with different child names, and legacy/unbound exact-task
  records. A cached terminal state cannot hide its replacement by an unknown state.
- Native receipt reuse benefits both before probing and during its existing
  post-retrieval revalidation. Storage drift blocks with `journal_unavailable`,
  rather than returning stale output or suggesting a fresh execution.
- Inspection, original-receipt retrieval and exact cancellation reject damaged
  selected evidence before an Agent RPC. Existing HTTP error/auth contracts remain
  unchanged; responses do not expose the damaged payload or local path.
- Detected failures quarantine the journal, preserving the files and keeping its
  process alive. Restoring files alone does not silently clear quarantine. Explicit
  repair plus reload verifies the original generation again. A genuinely empty
  fresh journal remains usable without creating files during reads.

The read-only inspection policy exposes
`read_integrity_policy: verify_inventory_and_selected_records`; the current sample
and schema agree. The field is optional for older inspection responses. File
format, 512-record limit, 4,096-byte record limit and 128 terminal-history retention
are unchanged. No new Node/shell runtime, result cache or backup is introduced.

## Verification

The initial 42 read-integrity tests failed 40 cases before repair. Those failures
exercise one cache-validation gap across multiple operations and storage faults,
not forty independent vulnerabilities. The final addition includes:

- 44 journal tests covering deletion, corruption, oversized and replaced payloads,
  symlinks, root loss, incomplete writes, unexpected membership, absent/empty
  controls, legacy records, unchanged bytes/inodes/permissions, repair/reload and
  a full 512-record inventory. The full inventory exercises one selected success,
  511 selected unresolved records, parent isolation and unchanged capacity refusal.
- Six native-recovery tests inject deletion, valid replacement or partial storage
  before and during original-receipt retrieval. No alternate probe or stale result
  is published. Earlier consistent-observation replacement tests explicitly inject
  both memory and disk changes outside the API, retaining their separate
  `dispatch_observation_changed` coverage.
- Three coordinator tests change live terminal evidence, then attempt recovery.
  Recovery stays blocked without a new claim, preserving the envelope, generation
  and attempt; the coordinator remains alive and the job runner does not start.
- Nine HTTP tests cross inspection, receipt retrieval and exact cancellation with
  deleted, corrupt and validly replaced records. Each rejects before any controlled
  Agent RPC, preserves the altered files and omits paths and raw diagnostics.

From `apps/web`:

```sh
mix test test/kyuubiki_web/orchestra/operator_dispatch_read_integrity_test.exs --seed 68129
mix test test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/workflow_native_attempt_recovery_test.exs test/kyuubiki_web/orchestra/workflow_dispatch*test.exs --seed 68130
mix test test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/workflow_native_attempt_recovery_test.exs test/kyuubiki_web/orchestra/workflow_dispatch*test.exs test/kyuubiki_web/api/operator_dispatch*test.exs --seed 68131
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs test/kyuubiki_web/playground test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68132
```

The first command is the pre-repair reproduction. The repaired intermediate
selections passed 164 and 194 tests. After the full-inventory and repair/reload
controls, expanded service regression passed: 2,872 total, 2,801 passed,
71 conditional skips, zero failures. Skips are not executed evidence. Tests use
disposable local files and controlled receipts, not fresh native calculations.
Mix's local TCP socket restriction required an approved local-test retry.

Formatting, whitespace, document-book/inventory, tensor self-test/validation and
project organization checks pass. The 800-line source and 2,000-line document
limits have no tracked debt. The tensor remains at 13 modules, 11 paradigms,
zero structural gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps;
release qualification remains blocked.

## Scope and remaining boundaries

Each read lists the directory inventory but decodes only selected evidence. It is
not a full payload scan of every unrelated record. Corruption confined to an
unselected payload is discovered when selected or at full startup validation;
once discovered, the journal stays quarantined. The tests preserve this scope
explicitly. Directory listing under hostile unbounded file injection, filesystem
latency limits and performance qualification are not established by these tests.

This is a live single-writer cache-to-disk consistency check, not cryptographic
provenance or a transaction spanning Agent, journal and result publication. A
concurrent external edit after a check, a consistent rewrite of both memory and
disk, or evidence already aged out is not comprehensively fenced here. The
previous receipt/claim gates remain necessary. No general exactly-once claim is made.

If the entire directory is already missing before process startup, the existing
loader still treats it as a fresh empty journal. This round does not provide the
independent durable attempt lineage needed to distinguish those cases. Do not
delete historical records or restart with an empty directory to resolve a blocked
recovery. Repair/reload tests restore the original evidence, not erase it.

No installed GUI, physical-server, power-loss, multi-writer or numerical accuracy
qualification is claimed. The tensor claim is limited to Orchestra contract and
recovery, without raising module maturity or release qualification. No version
bump, commit, push, packaging, installation or remote execution was performed.
