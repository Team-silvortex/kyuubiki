# Kyuubiki Core Exchange

`kyuubiki-kcore` is the native reference implementation of the `.kcore`
exchange format. It exports frozen simulation and research results, validates
their cross-operator contracts and content digests, and safely extracts them.

`.kyuubiki` remains the editable project format. `.kcore` is a portable,
read-only exchange artifact intended for runtimes, SDKs, research archives,
store distribution, and third-party tools.

```text
kyuubiki-kcore export export.json --out result.kcore
kyuubiki-kcore inspect result.kcore
kyuubiki-kcore verify result.kcore
kyuubiki-kcore extract result.kcore --out restored-core
```

See `docs/kcore-exchange-format.md` and
`schemas/kcore-manifest.schema.json` for the normative v1 contract.
