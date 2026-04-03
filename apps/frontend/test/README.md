# Frontend Test Notes

The frontend currently leans on build/type validation more than a large native
unit-test suite.

Current safety rails include:

- `npm run build`
- `npx tsc --noEmit`

As frontend-specific test coverage grows, place browser/workbench tests under
this directory by domain rather than mixing them into generated or dependency
trees.
