# Material Result Persistence Round Trip

Date: 2026-09-09. Base revision: `695bd4fc` (`daji 3.1.9`) plus the working tree.
No version bump, Git commit, installed application update or production data mutation.

## Result

**Result persistence key round trip: PASS.**

The earlier expanded memory-profile failure is a real serialization bug, not a solver
failure or a reason to disable integrity verification. A completed
`workflow.material-study-envelope-ranking-json` result contained
`bundle_domain_counts` with the native key `nil`. Its digest path used `to_string(nil)`,
which produces an empty string; Jason instead serialized that object key as `"nil"`.
The next write therefore rejected a generation the application itself had written.

Reconstructing only that one key from the retained failed result reproduced the exact
stored digest `d420aa538f3fa3bb6af58da58fd76ad9360387b02567e2b6688c12869c253b32`.
Ordinary JSON decoding and re-encoding reproduced all 34,863 file bytes exactly, so
this diagnosis does not depend on guessed float rounding or a changed file body.

## Changes And Boundaries

- `Storage.DurableJson` shares consistent, sorted JSON key encoding between result
  persistence and workflow recovery. It preserves Jason's existing number spelling.
- Native nil, boolean, atom and integer object keys survive JSON round trips. Nil and
  the empty string remain distinct. Aliases such as `nil`/`"nil"`, `:queued`/`"queued"`
  and `7`/`"7"` are rejected before replacing a persistence generation.
- Workflow recovery preparation and verification return controlled errors for invalid
  native JSON values. Payload/policy tampering remains refused. No schema revision or
  automatic approval of historically inconsistent digests is introduced.
- Generic ranking/Pareto bundle sources retain their payloads but no longer create a
  nil diagnostic-domain counter. Domain counts cover declared domains only.
- The real material-envelope API test now publishes the stored result into an isolated
  JSON generation, reads it back independently, checks that no quarantine/recovery was
  needed, writes the next generation and verifies the previous one.
- The two tests overriding `KYUUBIKI_DATA_DIR` restore its prior value and remove only
  their own directory. A SQL foreign-key assertion is explicitly SQL-only; the memory
  compare-and-swap test reuses the already supervised backend rather than starting a
  duplicate named process. The memory backend is not claimed to implement SQL FKs.

## Verification

Seven new regression cases were added to the existing persistence, workflow-recovery
and diagnostics groups. Before implementation the 20-test focused selection reproduced
five failures; after implementation all 20 pass. The API regression is an additional
assertion chain in an existing test, not a separately counted test.

macOS ARM64, Elixir 1.19.5 / OTP 28:

| Profile | Seed | Selected | Passed | Skipped | Failed |
| --- | --- | ---: | ---: | ---: | ---: |
| JSON/memory | 615405 | 327 | 321 | 6 | 0 |
| SQLite | 0 | 327 | 322 | 5 | 0 |

Both selections include storage, jobs, Orchestra, API and diagnostics bundle tests.
Five skips require an isolated PostgreSQL endpoint not supplied on the Mac. Memory
also skips the SQL-only foreign-key case, which passes in the SQLite selection.
The counts overlap and must not be added as independent feature coverage.

```text
MIX_ENV=test mix test test/kyuubiki_web/storage test/kyuubiki_web/jobs \
  test/kyuubiki_web/orchestra test/kyuubiki_web/api \
  test/kyuubiki_web/workflow_diagnostics_bundle_runtime_test.exs --seed 615405
```

Select `KYUUBIKI_STORAGE_BACKEND=memory` or `sqlite` and set an isolated
`KYUUBIKI_DATA_DIR`; SQLite additionally needs a fresh `SQLITE_DATABASE_PATH`.
Stop writers before removing those disposable test directories.

Physical Ubuntu x86_64: the same 20 focused tests pass in the cached `elixir:1.19`
container, using the four current runtime source files and three current test files.
Unchanged support modules/dependencies come from an existing compiled test tree.
All seven current source/test file digests match the transferred remote files.
Source mounts are read-only, networking is disabled, and generated fixtures live in
an ephemeral `/tmp` mount. This is focused Linux regression evidence, not a new
installed runtime or a full current-source Linux application qualification.

An intermediate local broad run caught a helper naming collision introduced while
wiring this change. The existing `CanonicalJson` TaskIR encoder has been restored
unchanged; persistence uses the distinctly named `Storage.DurableJson`. Both complete
327-test profile results above are from the corrected wiring.

## Retention And Limits

The small report and source regressions remain, not another source/database snapshot.
The temporary verification overlay, resolved failure artifact and generated test stores
were removed after verification. The container was removed on exit; existing dependency
and build caches were left intact. No backup rotation or retention service was added.

HTML book, documentation inventory, formatting, warnings-as-errors compilation,
800/2000-line organization audit, tensor structure and tensor self-test passed. The
temporary full tensor JSON was removed after checking it; global `daji status=blocked`
remains unchanged.

Still unqualified here: PostgreSQL execution in this round, multi-process concurrent
file writers, ENOSPC/power loss, packaged GUI migration and native Windows filesystems.
The tensor claim is scoped `verified` evidence; it does not promote the global release gate.
