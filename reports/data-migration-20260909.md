# Offline Project Data Migration: 2026-09-09

## Scope

**Project copy-migration and independent verification: PASS.**
Base revision: `67a050ab` (`daji 3.1.7`) plus the retained source changes.
Application/Cargo versions are not bumped. Data schema remains independent:
`kyuubiki.project/v1` is migrated explicitly to `kyuubiki.project/v2` with
`kyuubiki.project-layout/v1`.

This is a first data-lifecycle increment, not an all-store migration framework.
No production projects, databases, credentials, installed runtimes or GUI shells
are modified. The public Rust project library and native command runner implement
the flow; the three official Headless SDK packages and GUI have not been wired
to a new data-management screen in this increment.

## Implemented Contract

- Read-only plan: real source schema, complete source-file SHA-256, root field
  changes, source/archive limits, compatibility status and explicit blockers.
- Copy migration: approve the source digest, snapshot into a private destination
  sibling, upgrade metadata, reconcile canonical record mirrors, preserve opaque
  attachments, and record migration provenance inside the archive.
- Verify before publication: compare every candidate entry digest and the entire
  target manifest, then check the source digest again. Sync the candidate and
  publish without overwriting an existing or concurrently created target.
- Independent verification: a separately retained receipt binds output bytes,
  embedded provenance and the target manifest without requiring original paths.
- Rollback: retain and reopen the original. No in-place downgrade or automatic
  merging of edits made after migration. Normal errors clean owned staging;
  cleanup failures are exposed rather than hidden.

Malformed optional arrays used to become empty arrays during normalization.
They now fail instead of silently discarding data. The migration reader also
rejects duplicate nested JSON keys, duplicate record IDs, broken references,
unknown layouts and ambiguous file paths.

Physical ZIP/ZIP64 directory counts, bounds and duplicate names are checked
before zip 2.x constructs its name-indexed map. A regression test demonstrates
that the upstream map hides exact duplicates; the new migration gate rejects
that archive before this can lose data. CRC, expanded-byte budgets, special
files, case collisions and file/directory aliases are checked as well.
The guard is scoped to migration/verification, not every legacy archive reader.

## Validation

The native project/automation suite passes on macOS ARM64 (Rust 1.88.0) and
Linux (Rust 1.95.0): **36 tests per platform**. This includes 29 project tests
(24 new) and seven existing project-automation tests. Three native command
adapter tests cover migration option boundaries, missing approval and blocked
plan exit status. Counts are test functions, not material models or every
parameter combination.

An additional Linux run selects project-bundle, solver, engine, protocol and
the in-workspace Headless SDK together, exercising the unified
`serde_json/float_roundtrip` feature set: **1,627 passed across 178 test targets,
zero failures, six explicitly opt-in tests ignored**. The ignored tests are
five solver probes/benchmarks and one external operator dynamic-library fixture.
These counts overlap the project suite and must not be added as unique coverage.
The first attempt was blocked by a missing frontend operator-template source
file in the remote test copy, not a numerical assertion; after copying the
original repository file, the entire group passed without skipping that test.

All 1,265 retained native-workspace source/test file digests match the remote
tree. The plan, receipt and relocated-verification output pass their JSON Schema
on the server. Clippy with warnings denied, Rustfmt, the 26-page HTML book,
documentation inventory, tensor structure/self-test and organization audit pass.
Source/document limits remain 800/2000 with zero tracked debt; the global tensor
still reports `daji status=blocked`.

The Linux command executable also completes a real process-level sequence:
v1 synthetic thermal project JSON -> read-only plan -> digest-approved v2
archive -> independent verification -> move archive -> verification again.
Reusing the existing destination returns exit 1 without modifying it. The
source digest remains unchanged. Filesystem directory sync returns true for
this run; no cleanup residue is reported.

| Artifact | SHA-256 |
| --- | --- |
| Original CLI fixture, unchanged | `5127f3a6c645c38fcb1edd2df3ffecaa29e1dfd751af2cf17a2a319626cb33d5` |
| Final migrated and relocated archive | `a9d9bc357d739018af614b17c26e689892e54e85b427d0175966751866b7e6fc` |

Tests cover original/result/extension preservation, signed zero and retained
floating-point fields, read-only inspection, same-schema copies and chained
receipts, CRC corruption, exact duplicate ZIP entries, ZIP64 metadata, malformed
counts/offsets, truncation, stale source approval, conflicting scientific mirrors,
output budgets, existing targets, symlinks, unknown versions and tampered receipts.
Migration is structural data work, not numerical/material certification.

Server evidence is retained under managed state
`research-runs/data-migration-20260909/evidence/`: native test logs, command
tests/build log, source fixture, plan, external receipt, relocated archive,
verification receipts and expected overwrite-refusal logs. Local diagnostics
remain ignored under `tmp/data-migration-*`. Server configuration and payloads
are not added to Git.

## Remaining Gates

See the [data lifecycle chapter](../docs/data-lifecycle.html) for the full
ownership and compatibility rollout. The immediate next gate is a **versioned
database migration ledger plus consistent backup/restore**, not more implicit
startup table creation. Kcore/research-archive evolution, settings/drafts,
credential re-binding, Installer activation preflight and GUI integration are
separate domains.

This run does not qualify live database copying, arbitrary concurrent writers,
ENOSPC fault injection, hard process kill or host power loss, Windows native
filesystem behavior, network filesystems, Unicode normalization equivalence,
or large/1M-node bundle performance. Unsupported encrypted/multi-volume/legacy
filename encodings require explicit conversion. Hashes are not signatures.
The global coverage/maturity gate is not promoted by this scoped result.
