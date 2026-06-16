# Frontend Test Notes

The frontend now has a lightweight native unit-test layer built on Node's
`node:test` runner with `--experimental-strip-types`.

Current safety rails include:

- `npm run build`
- `npm run typecheck`
- `npm run test:unit`
- `npm run test:unit:workflow`

Organization rules:

- Put reusable fixtures and helpers under `test/support`.
- Group tests under `test/<domain>` so workflow, installer, and hub behavior can
  grow independently.
- Keep browser and smoke coverage separate from pure unit tests; integration
  flows still live under the repo-level `tests/integration`.
- The unit-test runner registers a local `@/` alias loader, so tests may import
  real frontend source modules directly instead of copying logic into test-only
  wrappers.
