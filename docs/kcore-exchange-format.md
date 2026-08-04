# Kcore Exchange Format

Status: normative v1 baseline for moxi 2.9.0.

## Purpose

`.kcore` is Kyuubiki's frozen computation exchange format. It carries the
portable result of a simulation or research workflow across Kyuubiki runtimes,
SDKs, archives, stores, and third-party systems.

It is not an editable project format:

| Concern | `.kyuubiki` | `.kcore` |
| --- | --- | --- |
| Primary role | Authoring and workspace persistence | Computation exchange |
| Mutability | Editable, versioned project state | Frozen, content-addressed artifact |
| Local settings | Allowed | Forbidden |
| Runtime-private state | Allowed when project-owned | Forbidden |
| Results | May retain mutable job history | Declared portable artifacts |
| Integrity | Project validation | Manifest and payload SHA-256 |
| Consumer | Hub and Workbench | Any conforming reader |

The relationship is similar to an editable source document and its portable
rendered output. A `.kyuubiki` project may export many `.kcore` results. A
`.kcore` reader does not need to understand the source project layout.

## Design Principles

1. Language neutral. Elixir descriptors, Rust engines, Python clients, and
   external tools consume the same manifest.
2. Contract first. TaskIR, workflow graph, workflow dataset, model, material,
   mesh, result, and evidence schemas remain explicit artifacts.
3. Path independent. Build-time source paths never enter the final manifest.
4. Content addressed. Embedded payloads are keyed by SHA-256 and deduplicated.
5. Streamable. Export and verification process payloads in bounded chunks.
6. Fail closed. Missing, extra, duplicated, oversized, or modified entries
   invalidate the package.
7. Evolutionary. New artifact roles and schemas can be added without changing
   the container version. Breaking container changes require a new version.

## Media Type And Extension

- File extension: `.kcore`
- Media type: `application/vnd.kyuubiki.kcore`
- Manifest schema: `kyuubiki.kcore/v1`
- Export specification schema: `kyuubiki.kcore-export/v1`

The file is a ZIP-compatible container, but generic ZIP acceptance is not
enough. A conforming reader must enforce every invariant in this document.

## Container Layout

```text
mimetype
manifest.json
objects/<first-two-sha256-characters>/<full-sha256>
```

`mimetype` is the first entry and contains the exact media type without a line
ending. `manifest.json` describes every object. Object names carry no host path,
display name, extension, or user identifier.

No other entries are accepted in v1. Directory entries, symbolic links,
duplicate names, path traversal, and unreferenced objects are invalid.

## Manifest

The normative machine-readable definition is
`schemas/kcore-manifest.schema.json`. Its top-level fields are:

- `schema_version`: exact manifest contract version.
- `format` and `format_version`: container identity and compatibility line.
- `core_id`, `title`, and `kind`: portable research-result identity.
- `producer`: product, version, and optional runtime that produced the core.
- `artifacts`: semantic descriptors for embedded content-addressed payloads.
- `contracts`: named schema bindings that make cross-operator meaning explicit.
- `entrypoints`: artifact ids a consumer should open first.
- `provenance`: source digest, execution lineage, solver posture, or authority
  evidence that is safe to exchange.
- `metadata`: domain-specific portable metadata.
- `integrity`: manifest-level integrity algorithm and core digest.

`created_at` is optional so reproducible builds can omit wall-clock state.

## Artifact Contract

Each artifact declares:

- a unique portable `id`;
- a semantic `role`;
- a `media_type`;
- a content-addressed `object_path`;
- exact `byte_length` and lowercase `sha256`;
- optional `schema_ref`, encoding, shape, unit, display name, and metadata.

Recommended standard roles are:

| Role | Meaning |
| --- | --- |
| `model.geometry` | Geometry or topology input |
| `model.mesh` | Mesh, connectivity, and element data |
| `material.catalog` | Material definitions or assignments |
| `operator.task-ir` | Language-neutral executable task description |
| `workflow.graph` | Multi-operator execution topology |
| `workflow.dataset-contract` | Cross-operator value contract |
| `result.field` | Nodal, elemental, voxel, or sampled field |
| `result.table` | Structured scalar or tabular result |
| `result.report` | Human-readable or machine-readable conclusion |
| `evidence.validation` | Accuracy, convergence, or qualification evidence |
| `evidence.provenance` | Producing runtime and lineage evidence |
| `preview.image` | Portable visual preview |
| `preview.scene` | Portable 3D scene or visualization state |

Roles describe purpose. Media types describe representation. Schema references
describe semantics. Consumers must not infer all three from a file extension.

Vendor roles should use a stable namespace such as `vendorname.role-name`.

## Cross-Operator Contracts

`contracts` binds a named contract and schema version to an artifact. A typical
re-runnable workflow result includes at least:

- `kyuubiki.workflow-graph/v1`;
- `kyuubiki.workflow-dataset/v1`;
- `kyuubiki.operator-task-ir/v1` for each executable task family;
- domain schemas for model, material, mesh, and result artifacts.

A result-only export may omit executable TaskIR. Presence of TaskIR never grants
execution authority. Runtimes must still apply package, capability, signature,
resource, and placement policy before execution.

## Integrity

Each payload digest is SHA-256 over its exact uncompressed bytes.

`integrity.core_digest_sha256` is computed as follows:

1. Serialize the manifest to JSON data.
2. Replace `integrity.core_digest_sha256` with an empty string.
3. Canonicalize JSON by sorting every object key recursively, preserving array
   order, and using compact JSON scalar encoding.
4. Compute lowercase SHA-256 over the UTF-8 canonical JSON bytes.

This binds artifact descriptors, contracts, entrypoints, provenance, metadata,
and payload digests without creating a recursive file hash.

Digital signatures are intentionally separate from the v1 core digest. A
future signature envelope can sign the core digest without changing object
identity.

## Export Specification

`schemas/kcore-export.schema.json` is a build-time contract. It resembles the
manifest but each artifact has a `source` path instead of object path, size, and
digest. Relative paths resolve from the export specification directory.

The native exporter performs two logical passes:

1. Resolve regular non-symlink files, stream their SHA-256 and build a sealed
   path-free manifest.
2. Stream objects into the container and recompute each digest while writing.

The source field is discarded. Absolute source paths are allowed only as local
build inputs and are never serialized into `.kcore`.

## Native Commands

```text
kyuubiki kcore export schemas/examples.kcore-export.json --out result.kcore
kyuubiki kcore inspect result.kcore
kyuubiki kcore verify result.kcore
kyuubiki kcore extract result.kcore --out restored-core
```

The standalone Rust binary exposes the same commands as `kyuubiki-kcore`.
SDKs and GUI native bridges can call `export_spec` with the public
`ExportSpec` and `ExportArtifact` types, rather than writing an intermediate
specification file. They should bind the `kyuubiki-kcore` library rather than
reimplementing ZIP, digest, path, or validation behavior.

`inspect` validates the container table, marker, manifest contract, references,
and core digest. `verify` additionally streams every object and validates byte
length and payload digest. `extract` verifies first and then writes only safe,
declared entries into a new directory.

## Security And Resource Limits

The reference reader enforces:

- at most 100,000 archive entries;
- at most 16 MiB for `manifest.json`;
- at most 1 TiB per uncompressed artifact;
- at most 4 TiB total verified payload bytes;
- no host-absolute paths in the final manifest;
- no symlinks, duplicate entries, traversal, undeclared entries, or overwrite;
- no implicit execution after inspect, verify, or extract.

Applications may choose stricter `ReaderLimits`. They must not choose limits
that bypass structural or cryptographic checks.

## Versioning

The v1 reader accepts only `kyuubiki.kcore/v1` and `format_version: 1`.

Compatible evolution uses new artifact roles, media types, schema versions, or
optional metadata. Breaking changes to path layout, digest semantics, required
entries, or trust behavior require `kyuubiki.kcore/v2` and a new format version.

`.kcore` does not replace workflow datasets, TaskIR, or research bundles. It is
the exchange envelope that carries those contracts and their payloads together.

The Orchestra model/result artifact stores are live transport and retention
facilities, not another exchange format. Their SHA-256 references can be bound
directly to `.kcore` export artifacts: export resolves the immutable bytes,
verifies the declared digest and length, then embeds them under `objects/`.
This keeps million-node execution bounded during research while `.kcore`
remains the portable, self-contained handoff artifact.
