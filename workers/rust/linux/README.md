# Linux Layout Skeleton

This directory keeps the expected Linux runtime layout visible in the
repository.

Treat it as a path contract, not as a built artifact cache.

Subdirectories map to the standard install/runtime shape used by packaging,
repair, and integrity-check flows:

- `bin/`
  installed executables and helper launch binaries
- `config/`
  visible runtime configuration files
- `data/`
  writable product data roots
- `desktop/`
  desktop-shell specific layout anchors
- `exports/`
  user-visible bundle or export output roots
- `logs/`
  runtime log roots
- `manifests/`
  install/update/integrity manifest material
- `scripts/`
  packaged helper scripts

If a future installer or packager starts writing into this tree directly, keep
the path meanings stable and document the behavior before expanding it.
