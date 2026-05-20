# Release Decision: v0.9.0

## Status

`v0.9.0` is `ready with warnings` for an initial usable release.

This is a ship decision, not a claim that every polish item is done. The
current evidence says the release baseline is real, repeatable, and broad
enough to support a first usable cut.

## Why it is ready

- the release validation baseline is green:
  - `make test-web`
  - `make test-rust`
  - `make test-frontend`
  - `make test-sdk`
  - `make test-integration`
  - `make test-hub-gui`
  - `make test-installer-gui`
  - `make test-workbench-gui`
  - `make desktop-status PLATFORM=all`
- supported study families have real smoke evidence across:
  - sample open in Workbench
  - orchestrated run
  - direct-mesh where documented
  - report usability
  - export usability
- supported `thermal -> thermo-mechanical` bridge workflows have real workflow
  smoke
- Hub, Workbench, control plane, agents, and desktop shells now share a
  coherent first-run and troubleshooting story

## Why it is not “fully clean”

Current release warnings are still real:

- macOS host bundles are not yet built on this machine
- `minimal` families still have intentionally narrower guarantees than the
  `fully supported` set
- some polish and onboarding items remain in the `should` bucket

None of these currently contradict the `v0.9.0` support matrix.

## Blockers

Treat `v0.9.0` as blocked if any of the following become false:

- the validation baseline above stops passing
- an officially supported study loses:
  - `sample opens`
  - `orchestrated run`
  - `report usable`
  - `export usable`
- the documented `heat -> thermo-mechanical` bridge workflows regress
- desktop scaffolds, manifests, or icon inputs break for a supported target

## Recommended release call

Use this wording for the current state:

- `ship as initial usable release`
- `accept current warnings`
- `do not broaden the support matrix without new smoke evidence`

## Next best actions after release

1. Build host desktop bundles on the release machine.
2. Keep extending smoke coverage before expanding the official support matrix.
3. Continue improving onboarding and operator polish without reopening the
   current release decision.

## Related docs

- [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)
- [release-smoke-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-smoke-matrix-0.9.0.md)
- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [release-first-run-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-first-run-0.9.0.md)
- [release-troubleshooting-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-troubleshooting-0.9.0.md)
