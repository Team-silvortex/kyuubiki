# Mesh Live Store Binding

Date: 2026-10-18. Source metadata: daji 3.6.0. Git base: `3a5ef43f`
(`daji 3.6.5`), with the existing working-tree changes. Evidence is from local
macOS source tests, not an installed application or remote deployment.

## Confirmed Gap

The Agent held an exclusive owner file and cached its stable store ID, but all
later reads and commits used the configured directory path. The caller's
`expected_store_id` was compared only with the cached ID. Moving the directory
while the process remained alive could detach that path from its held lock.
Replacing `owner.lock` with identical bytes also left the cached owner check
passing while the path named a different inode.

Three new unit regressions failed before the repair: live identity corruption,
directory replacement, and replacement of the lock with an identical copy.
The directory case includes a copied admission record, so receipt binding to
the task alone cannot establish that the destination is still the owned store.

## Repair

- Unix store operations validate the private, non-symlink root and owner file,
  compare the current file's device/inode with the still-held lock, and reread
  at most 161 bytes to validate the checksummed identity. Empty or malformed live
  identity files fail; only startup retains the existing legacy initialization.
- Admission, explicit lookup, list and cancellation owner checks reject a detached
  store. Admission persistence and terminal receipt commit recheck their boundary.
  Lookup failures are not converted into `not_found`; no replacement is adopted.
- Agent discovery remains available, but its durable-storage descriptor reports
  `available: false` and `store_binding_unavailable`. Workbench can use its existing
  unavailable-store path rather than receiving a false healthy-store declaration.
- The solver and engine contracts are unchanged. No automatic migration, replay,
  eviction, repair write, store recreation or additional backup is introduced.

## Native Process Evidence

`agent_mesh_store_binding_live` starts two real native Agents in the same local
Mesh. One commits an axial-bar result, then holds a second execution. Its execution
directory is renamed and the configured path is replaced with another owner's
identity plus the held task's exact admission bytes.

The old Agent rejects asynchronous/synchronous submissions, fenced and legacy
fetches, listing and cancellation. Its descriptor remains readable and advertises
the storage failure. Releasing the computation does not publish a terminal result
or change the replacement's file count or contents. The neighboring Agent still
finishes its own computation.

After force termination, restoration of the original directory and restart, the
previously completed receipt is exactly retained. The interrupted task is
`outcome_unknown`; resubmitting its identity returns that same observation without
running it again. An explicitly new execution completes normally.

## Validation

- CLI unit suite: 284 passed, including 33 execution-store tests.
- Six native process suites: 41 test executions passed, consisting of 29 scenario
  tests and 12 repeated shared readiness-fixture checks. Suites: durable async,
  durable execution, durable recovery, Mesh cancellation recovery, Mesh owner
  replacement and live store binding.
- The first new live run used the incorrect lifecycle label `ready`; the fixture
  was corrected to `accepting` and rerun successfully. That failed run is not
  counted as passing evidence.
- Strict CLI Clippy across all targets, Rust formatting and `git diff --check`
  passed. Documentation book/inventory and project-organization audits passed.
- Tensor audit: zero structural gaps, two maturity gaps, 22 evidence-grade gaps
  and 17 P0 gaps; release qualification remains blocked.

## Boundaries

The owner-file device/inode check is Unix-specific. Non-Unix builds validate the
root metadata only and expose `owner_file_binding_checked: false`; reopening a
Windows byte-range-locked file is deliberately not assumed to behave like flock.
Windows native handle identity remains open, and Linux was not rerun this round.

These are checks at operation boundaries, not directory-handle-relative I/O or
protection against arbitrary concurrent filesystem mutation. Local filesystem
access is not cryptographic membership or task authentication. Host power loss,
authenticated multi-host recovery and installed WebView qualification remain open.
The existing `rust-agent-lan-mesh-local-isolation` evidence is extended without
promoting its grade or changing release qualification.
