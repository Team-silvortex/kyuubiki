# Dispatch observation transitions and failure isolation

Date: 2026-10-12. Daji 3.6.x local source follow-up based on `f45f1e1f`
and the existing uncommitted workflow/batch recovery work. No version change,
commit, push, packaging, installation or remote deployment.

## Findings and repair

`OperatorDispatchJournal.finish_dispatch` previously allowed any valid observation
to overwrite any retained state. A late completion could turn an unknown outcome
into success and clear the workflow replay fence. Repeating the same observation
also rewrote the file and timestamp, changing retention and recovery-selection
evidence. A malformed state failed file validation and poisoned the journal's
global error state, preventing unrelated tasks from dispatching. Observation time
could move backwards after a wall-clock correction or restored future timestamp.

- An attempt moves from `dispatch_boundary_unconfirmed` to one of five observations:
  `outcome_unknown`, `observed_executed`, `observed_failed`, `observed_blocked` or
  `not_dispatched`. That observation is sealed. A different later state, including
  a reset to unconfirmed, returns `operator_task_dispatch_transition_rejected`
  without mutation or poisoning other attempts.
- Identical repeats, including an initial-state repeat, verify the original file
  generation and absence of an incomplete `.next` replacement. They do not write,
  update timestamps or refresh retention. The check uses a bounded single-record
  read, not a full directory scan. Missing/corrupt/replaced/symlinked records,
  partial replacement or missing/symlinked journal directories fail closed and
  quarantine further dispatch. No automatic rollback or recreation occurs.
- Malformed states return `operator_task_dispatch_state_invalid` without touching
  files or setting the journal-wide error. This is request rejection, not storage
  recovery. A journal already quarantined for storage failure stays unavailable.
- The first observation timestamp is at least the previous recorded timestamp.
  Legacy v1 records obey the same transitions without schema migration or guessed
  parent identity. Missing/evicted attempts cannot be recreated by a late finish.
- The production dispatch adapter already converts an unrecordable finish into
  `operator_task_dispatch_outcome_unknown`. A controlled delayed TCP success now
  exercises that path when the attempt was already sealed unknown. A duplicate
  success returns normally without rewriting, and execution capacity is released.
- Explicit retained-receipt retrieval can still return a verified receipt for an
  unknown attempt, but it never changes that observation, publishes a result or
  authorizes automatic replay. Native recovery retains its post-read revalidation.
  Its older observation-replacement tests now inject an internal fault explicitly;
  ordinary public finish calls can no longer perform those replacements.

Read-only inspection exposes the transition, repeat and invalid-transition
policies. The published sample/schema agree with those values. New policy fields
are optional in the existing inspection schema to accept older responses. Record
versions and limits are unchanged: 512 records, 4,096 bytes per record, 128 retained
terminal observations; unresolved attempts are never automatically pruned. No new
archive, backup, result payload or runtime dependency is introduced.

## Verification

The initial 23-case transition selection failed 16 tests before repair. These
were related behavioral failures, not sixteen independent security vulnerabilities.
The final transition suite contains 25 tests, including two directory-loss cases
and a deterministic retention-order control. Two additional tests exercise the
real local TCP transport and production dispatch adapter with a controlled peer.
The original-receipt and native-recovery suites each add an isolation control.

Coverage includes all five sealed states against every conflicting state,
concurrent duplicates with byte/inode/timestamp preservation, concurrent opposing
finishes with exactly one accepted observation, process restart, malformed states,
legacy records, clock rollback, retention eviction and eight storage faults. The
retention test seeds old timestamps offline to prove duplicate acknowledgements
cannot move an attempt ahead of newer observations. All fixtures are disposable.

From `apps/web`:

```sh
mix test test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/workflow_native*test.exs test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs --seed 68126
mix test test/kyuubiki_web/orchestra/operator_dispatch_transition_test.exs test/kyuubiki_web/playground/agent_task_dispatch_observation_test.exs --seed 68127
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs test/kyuubiki_web/playground test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68128
```

The first focused selection passed 155 tests before the additional directory and
TCP controls. The final targeted selection passed 27 tests. Expanded service
result: 2,810 total, 2,739 passed, 71 conditional skips, zero failures. Conditional
skips are not executed evidence. Mix required local test sockets; a sandbox socket
denial was retried with approved local test permissions, not remote access.

Formatting, whitespace, document-book/inventory, tensor self-test/validation and
project organization checks pass. No tracked size debt is reported under the
800-line source and 2,000-line document limits; the SDK guide stays at its limit
without adding lines. The tensor remains at 13 modules, 11 paradigms, zero
structural gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps.
Release qualification remains blocked.

## Evidence boundary

This is local Orchestra contract/recovery evidence. TCP peers supply controlled
receipts; no new live Rust Agent computation or numerical accuracy qualification
is claimed. No installed-GUI, physical-server, power-loss or all-races acceptance
is implied. Engine/Solver implementation and SDK authority boundaries are unchanged.

Sealing is enforced by the single-writer journal API, not cryptographic provenance.
It does not authenticate a consistently rewritten file, prove the observation's
physical truth, or protect against hostile same-process mutation. Reads remain
point-in-time checks; the Agent, journal and job publication are not one atomic
transaction. Explicit node-to-attempt lineage, evidence already aged out before
selection and cross-store publication fencing remain separate work. A conflicting
message is rejected, not silently reconciled or treated as an authorized correction.

The tensor receives only a bounded Orchestra contract/recovery claim. No module
maturity, Engine coverage, release qualification or storage budget is increased.
