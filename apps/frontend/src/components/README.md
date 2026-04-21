# Frontend Components

Component code is split by role rather than by route.

- `workbench/`
  Main modeling, viewport, inspector, and report surfaces for the browser FEM
  workbench.
- `ui/`
  Generic reusable UI primitives that are not specific to the FEM workbench.

Keep domain-specific stateful surfaces in `workbench/`. Only promote code into
`ui/` when it is genuinely generic.

For component splitting, ownership, and state placement rules, see:

- [docs/frontend-implementation.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-implementation.md)
