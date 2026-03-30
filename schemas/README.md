# Shared Schemas

These schemas capture the first cross-process contracts described in the root
README.

- `job.schema.json` is for durable job state
- `progress-event.schema.json` is for streamed runtime updates

They are intentionally lightweight and JSON-first for now. Once the Phoenix app
and Rust protocol crate exist, both sides should validate payloads against these
contracts in tests.
