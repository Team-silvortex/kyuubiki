# Store Native Publishing Interop, 2026-11-11

## Result And Scope

The new Silvortex submission backend interoperates with an actual macOS ARM64
operator package and the current Kyuubiki 3.7.8 Installer/Agent. No blocking
defect was observed in the scenarios below. This does not qualify the complete
GUI, identity, public download, or cross-platform release path.

- Backend publishing integration: 10 groups passed.
- Backend store/build/submission/review tool unit tests: 57 passed.
- Real-package interop: 6 groups passed, including one migration setup group.
- Returned-package native Installer/Agent lifecycle: 1 test passed.
- The native builder also passed template tests and the JSON C ABI probe.
- No production deployment, review approval of this package, publication,
  download quota consumption, model API call, or installed-runtime replacement.

Backend binaries were the supplied Linux builds: store dated 2026-11-11 and
migration runner dated 2026-11-04. The runner applies the current migration
directory to a new disposable database; migration replay passed.

## Provenance

The backend's fixed-source builder used commit
`3fe2254268345f66bfc7315f7e2ca7d2ea9264b5`, not the dirty working tree.
The existing native candidate preparer and final Agent test used the current
3.7.8 checkout. These are separate provenance claims.

- Compiler: Rust 1.88.0; explicit `aarch64-apple-darwin` release build.
- Binary: thin ARM64 Mach-O dylib; minimum macOS 11.0.0.
- Observed dependencies: `/usr/lib/libSystem.B.dylib`, `/usr/lib/libiconv.2.dylib`.
- Operator: `operator.template.summary`, package version `0.1.0`.
- SDK/ABI: `kyuubiki.operator-sdk/v1`, `kyuubiki.operator-json-c/v1`.
- Native ABI result for `[2, 4, 8]`: count 3, sum 14, max 8, mean 14/3.
- Package validation status remains `partial`.

OpenAPI SHA-256:
`480f2b23089bcca8996f4edcc0b7d064085d85a6f0c0b929de41e6699937707f`.

After the current Installer prepared the candidate, the uploaded files were:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `macos-aarch64/kyuubiki-operator.json` | 704 | `96d512d715b5e1a779a40ca2b985706925ccba12feb6f6af3e1216f8d4019729` |
| `macos-aarch64/operator.dylib` | 605616 | `5b5b772144224321ef752afa698ec9578a5fe995abd642b57122f30c9b685eca` |

The source library bytes were unchanged by repackaging; the entrypoint filename
and therefore the manifest bytes changed. The test declaration used the new
candidate index, not the original builder's manifest digest.

## Verified Behavior

1. Native candidate declarations and real API responses conform to the supplied
   OpenAPI using JSON Schema validation. Missing files prevent completion.
2. A successful manifest upload followed by a deliberately lost response and
   store restart resumes by transferring only the remaining library.
3. Full completion enters `pending_review / awaiting_review`. A further restart
   and replay transfer zero files and preserve the exact seal and binding.
4. Submitting the original filename layout under the same package/version
   returns `409 version_conflict`, even though the library bytes match.
5. Identical sealed-file PUTs replay successfully; different same-length bytes
   return conflict. Another subject cannot read the submission.
6. No public release, independent review row, or quota event is created by this
   real-package test. The backend never loads the uploaded library.
7. Files copied from the actual staging slots match both declared digests and
   lengths. They install into an isolated Mac store; a temporary local Agent
   executes the operator, rejects a wrong digest, and recovers for a valid task.
   Uninstallation prunes the managed test package store.

The separate backend suite additionally exercises concurrent creation/upload/
completion, ineligible and expired identities, real RSA test review receipts,
review-root revocation, tampered reviews, expiry, bounded upload concurrency and
interrupted-stream cleanup. Its binaries are transport fixtures, not additional
native platform qualifications.

## Reproduction

Use a trusted Silvortex backend checkout with its built `silvortex-store` and
`silvortex-migrate` binaries, `jsonschema`, OpenSSL, and the cached
`postgres:16-alpine` Docker image. No deployment dotenv or real account token is
needed. The interop runner reuses its disposable fixture and removes that
fixture's container, database, keys, and staging directory on exit.

On native Mac, run the backend's `scripts/build-kyuubiki-store-macos.py` against
the Kyuubiki repository to produce a new temporary bundle outside the checkout.
It extracts its fixed commit, builds with two Cargo jobs, and removes its source
and build workspace automatically. Preserve its `submission.json` and build
evidence while preparing the candidate:

```text
kyuubiki-installer prepare-operator-distribution \
  <bundle>/operator.template.summary/0.1.0/macos-aarch64/kyuubiki-operator.json \
  <bundle>/installer-candidate \
  aarch64-apple-darwin=<bundle>/operator.template.summary/0.1.0/macos-aarch64/libkyuubiki_operator_template.dylib
```

Transfer the bundle and this repository's opt-in interop test to an isolated
server directory. On the server:

```text
python3 -B <interop-test>/store-native-publishing-live.py \
  --backend <trusted-backend-checkout> \
  --bundle <native-bundle> --output <new-returned-directory>
```

Return only that test's output directory to the Mac. The following opt-in test
**executes native code**; never give it an unreviewed third-party package:

```text
KYUUBIKI_TEST_TRUSTED_STORE_CANDIDATE=<returned-directory> \
  cargo test --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-cli --test operator_release_live returned_store_candidate \
  -- --ignored --nocapture
```

The first local attempt was blocked by the environment's prohibition on opening
a local Agent port. The explicitly approved retry passed; no implementation
change or relaxed digest check was used to make it pass.

## Remaining Integration Work

- Supply the actual HTTPS test origin, public CA/trust material, registered
  desktop client/callback/scope, and a secure way to obtain a publisher-eligible
  shared-identity test session. Do not send private keys or access tokens in chat.
- This run exercised the Rust service over server-local HTTP and database-seeded
  ephemeral access tokens, not the Web gateway or interactive OIDC login.
- Backend submission ceilings are manifest 1 MiB, library 64 MiB, and all-target
  total 256 MiB. The generic native candidate builder allows a 512 MiB library.
  Keep local packaging capability separate; add store-specific preflight and
  expose negotiated upload limits rather than silently accepting oversize work.
- Backend target enumeration includes musl, GNU Windows and Windows ARM64 beyond
  the five currently accepted candidate targets. Gate these variants until ABI
  selection and native qualification are explicit; OS/CPU alone is insufficient.
- Port the submission mapping/recovery flow into the native Kyuubiki service and
  expose it through GUI/PWDT. The Python interop runner is test tooling only, not a
  new application runtime dependency or a completed user-facing publisher.
- Test controlled admission, signed public download and installation separately.
  Retrieving isolated staging files is not an authorized-download API test.
- Real package execution here is macOS ARM64 only; no Linux/Windows package,
  rendering, or application-shell test was performed in this round.

Only concise evidence and repeatable test source are retained. Temporary test
databases, keys, source/build copies and package transfers are disposable; no
server credentials or large binaries are added to Git.
