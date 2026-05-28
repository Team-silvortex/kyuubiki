# tamamono 1.x

This is the single entrypoint for the current Kyuubiki product line.

Use it when you want the shortest answer to:

- what `tamamono 1.x` means
- what this line is optimizing for
- where to go next inside the current `1.x` documentation set

## What defines this line

`tamamono 1.x` is the point where Kyuubiki stops defining progress mainly by
new operator families and starts defining it by industrial qualities:

- numerical trust
- repeatable validation
- bug fixing and consistency
- smoother operator and modeling experience

## What not to expect

This line should not grow by default through feature-count inflation.

The default posture is:

- keep the major version at `1`
- improve confidence before widening scope
- only add new capability when it strengthens the industrial baseline

The line opens at `tamamono 1.0.0`, but these expectations are meant to remain
true across later `1.x` releases too.

## Current backend momentum

Recent operator work is following the `tamamono 1.x` rule in the right order:

- add the solver and protocol path
- add agent/runtime support
- add sample-backed orchestrated smoke
- then decide whether wider UI exposure is worth it

The current example is the now-verified `frame_3d` / `thermal_frame_3d` / `thermal_truss_3d` backend line:

- Rust solver support exists
- protocol and engine paths exist
- agent RPC handling is wired through
- formal accuracy baselines exist
- official-sample orchestrated API smoke exists for all three studies

That is the kind of operator growth this line should prefer: narrower, more
verified, and less speculative.

## Current reading path

1. [version-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/version-line.md)
   Formal version-line note, codename, and major-version policy.
2. [tamamono-minor-lines.md](/Users/Shared/chroot/dev/kyuubiki/docs/tamamono-minor-lines.md)
   Suggested long-range grouping for the `1.x` minors.
3. [accuracy-plan.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-plan.md)
   Accuracy roadmap, benchmark targets, and verification priorities.
4. [accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)
   Concrete benchmark baselines already enforced in automation.
5. [language-packs.md](/Users/Shared/chroot/dev/kyuubiki/docs/language-packs.md)
   Local-first multilingual extension path for the Workbench UI, with a stable
   schema ready before remote delivery lands.

## Historical handoff

If you need the last release-hardening pack instead of the current product
line, use:

- [release-archive-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-archive-0.9.0.md)
