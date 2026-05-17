# First-Line Troubleshooting: v0.9.0

Use this note when `v0.9.0` does not behave as expected during an initial
operator session.

This is not a full incident guide. It is the short “where do I look first?”
note for release readiness.

## Where to look first

Start in this order:

1. `Kyuubiki Hub`
2. `Runtimes`
3. `Observe`
4. Workbench `System > Runtime`
5. control-plane API health endpoints

## Hub checks

### Runtime state

In Hub, check:

- runtime status
- hot reload loop status, if you are in a dev session
- workload library sync state

If the local path is not running, stop there first. Do not debug Workbench
result behavior before runtime state is known-good.

### Runtime watch

Use Hub `Observe` to inspect:

- stack logs
- hot-loop logs
- follow/frozen state

The log copy path is sanitized on purpose. It is safe for first-line sharing,
but it is not the raw file.

## Workbench checks

Inside Workbench, start with:

- `System > Runtime`
- `System > Data`

Use these to answer:

- did the job submit?
- did the result persist?
- did a security or routing issue show up?

If the study opens but no result appears, inspect the job/result records before
assuming the solver failed.

## Control-plane checks

The first endpoints to check are:

- `/api/health`
- `/api/v1/protocol`
- `/api/v1/protocol/agents`
- `/api/v1/agents`

These tell you whether:

- the orchestrator is alive
- protocol descriptors are being served
- agents are visible
- agent capability metadata is present

## Common symptom map

### Hub cannot open a useful workload path

Check:

- workload catalog sync
- attached local bundle path
- recent workload history in Hub

If needed, re-sync the local control-plane catalog and reopen the sample from
there instead of relying on a stale local path.

### Workbench opens but run does not complete

Check:

- Hub runtime watch
- Workbench `System > Runtime`
- `/api/health`

Most first-line failures here are:

- local stack not actually running
- no solver agent available
- control-plane routing mismatch

### Result exists but looks incomplete

Check:

- the study family support level in the support matrix
- the study-specific first-run path in the study coverage table

Some study families are intentionally `minimal` in `v0.9.0`, so the right
question is sometimes “is this family supposed to be editing-complete yet?”
rather than “did the app break?”

### Direct mesh behaves differently from orchestrated

Check the study coverage table first.

For `v0.9.0`, direct mesh is part of the supported path for some studies, but
not the universal first-release path for all families.

## When to stop and escalate

Escalate beyond first-line troubleshooting when:

- Hub cannot observe any healthy local runtime state
- `/api/health` is down
- agent descriptors are missing expected methods/capabilities
- a fully supported study cannot complete its official sample path

That is the point where it is worth treating the issue as a release blocker
rather than a local usage mistake.

## Related docs

- [release-first-run-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-first-run-0.9.0.md)
- [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)
- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)
- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [operations.md](/Users/Shared/chroot/dev/kyuubiki/docs/operations.md)
- [security.md](/Users/Shared/chroot/dev/kyuubiki/docs/security.md)
