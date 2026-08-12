# Kyuubiki Core Exchange

`kyuubiki-kcore` is the native reference implementation of the `.kcore`
exchange format. It exports frozen simulation and research results, validates
their cross-operator contracts and content digests, and safely extracts them.

An export that binds the `headless-research-round` contract receives an
additional semantic verification pass. Every retained round must include its
effective batch and real service run; later rounds must also include the exact
guarded parameter patch that reconstructs the next batch. Export and verify
both fail closed on broken lineage, stale patches, mismatched metrics, or
orphaned research artifacts.

`.kyuubiki` remains the editable project format. `.kcore` is a portable,
read-only exchange artifact intended for runtimes, SDKs, research archives,
store distribution, and third-party tools.

```text
kyuubiki-kcore export export.json --out result.kcore
kyuubiki-kcore research-export research-series.json --out research.kcore
kyuubiki-kcore inspect result.kcore
kyuubiki-kcore verify result.kcore
kyuubiki-kcore extract result.kcore --out restored-core
```

See `docs/kcore-exchange-format.md` and
`schemas/kcore-manifest.schema.json` for the normative v1 contract. The
self-contained research-series profile is defined by
`schemas/kcore-headless-research-profile.schema.json`. Use
`schemas/kcore-headless-research-series.schema.json` as the smaller native
packaging input; Rust assigns all artifact roles, schema references, contract
bindings, and entrypoints.
