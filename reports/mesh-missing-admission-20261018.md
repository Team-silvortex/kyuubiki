# Mesh Missing Admission Recovery

Date: 2026-10-18. Source metadata: daji 3.6.0. Git base: `3a5ef43f`
(`daji 3.6.5`), with existing working-tree changes. Tests ran on local macOS
native binaries, not installed applications or remote hosts.

## Confirmed Gaps

Removing an admission file while its Agent remained alive allowed the same
execution ID to pass the new-admission path again. The prior active set was not
consulted at that boundary; abandoning an execution also removed it from that
set. A resource reservation could therefore be attempted again for an identity
whose original execution might already have started.

Fetch treated a missing admission as `not_found`, including when a receipt still
existed. Listing enumerated only current admission files, so deleted admissions
and orphan receipts could silently disappear from a successful recovery index.
The existing orphan-receipt check blocked submission but not fetch or listing.

Four of five new unit regressions failed before the fix. The failing duplicate
test observed a second reservation callback, rather than merely checking an
error-message difference. The never-admitted negative control already passed.

## Repair

- Keep a bounded set of known admission IDs, populated by startup scanning and
  successful live admissions. Missing known IDs fail before resource reservation
  even after their active execution has ended. No inputs or results are cached.
- Centralize missing-record checks in the record reader. An orphan receipt blocks
  absence claims and ID reuse without reading its potentially large payload.
  Genuine unused IDs retain the explicit `not_found` state with no replay authority.
- Validate index membership before applying the cursor and limit. A missing known
  admission or orphan receipt returns an error rather than a shortened or empty
  successful page. Normal ordering, page bounds and receipt-free indexing remain.
- Publish the `missing_admission` policy and its lifetime limits. No file format
  change, backup, automatic repair, deletion or migration is introduced. Intact
  sibling tasks and other Agents retain their existing execution paths.

## Native Mesh Scenarios

`agent_mesh_missing_admission_live` contains two scenarios using real native Agent
processes over local TCP:

1. A two-slot Agent holds one task while its admission is moved out of the store.
   Async/synchronous duplicates, fetches, listing and cancellation reject the
   missing record. The active count remains one, and another task completes on
   its spare slot; a neighboring Mesh Agent also computes successfully. Letting
   the original worker finish creates no receipt from missing evidence. Restoring
   the original test fixture while stopped and restarting yields `outcome_unknown`
   with the original identity, never a replay. The sibling receipt remains exact.
2. A completed task loses its admission but retains its receipt. Fetch, index and
   duplicate submission remain errors through two forced restarts; receipt bytes
   remain unchanged. Healthy tasks still finish. After explicit restoration while
   stopped, the exact completed receipt and complete index are recovered without
   running the old task again.

## Validation

- CLI unit suite: 289 passed, including 38 execution-store tests.
- Eight native suites: 48 passing test executions, comprising 32 scenario tests
  and 16 repeated shared readiness checks. Suites cover missing admission, live
  store binding, Mesh owner fencing, Mesh cancellation, durable async/execution/
  recovery and store-lock contention.
- Desktop runtime direct/Mesh tests: 50 passed. Strict CLI Clippy across all
  targets, formatting and `git diff --check` passed.
- Documentation book/inventory and project-organization audits passed. Tensor
  audit still reports zero structural gaps, two maturity gaps, 22 evidence-grade
  gaps and 17 P0 gaps, without changing release qualification.

## Limits

The ID set only retains knowledge for the current process. Restart reconstruction
uses actual admission files; an orphan receipt provides surviving evidence even
when that admission is missing. If all evidence for an execution is erased before
restart, this guard cannot distinguish it from a never-admitted identity. It is
not an additional durable journal or a power-loss recovery guarantee.

Checks occur at store operation boundaries, not under a lock covering arbitrary
external filesystem mutation. Restore records with the Agent stopped. These local
tests do not qualify authenticated multi-host Mesh, installed GUI, Linux/Windows
or physical power loss. The existing local-isolation tensor evidence is extended
without a grade promotion.
