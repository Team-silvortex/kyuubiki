# Native Project Bundle

`kyuubiki-project-bundle` is the shared native implementation of the
`.kyuubiki` project container contract. Hub and the top-level native project
command use this crate so project storage behavior cannot drift between GUI and
automation surfaces.

The crate owns:

- safe create-without-overwrite behavior
- JSON, directory, and `.kyuubiki` input inspection
- project validation and summary rendering
- normalize, pack, unpack, and diff operations
- zip entry path containment during extraction

It does not own Workbench UI state, Pwdt, headless SDK transport, operator
execution, or orchestra scheduling.
