# Headless Native Request Buffer Bounds

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Existing uncommitted changes and historical evidence are preserved.

## Repair

Native JSON sending previously serialized an entire body into a string before
checking the 8,000,000-byte inline limit. It then copied that body into another
complete HTTP request string. Both buffers remained in scope during response
reading. Oversized values could allocate their entire encoded representation
before rejection, despite never being eligible for inline transmission.

An internal bounded writer now serializes directly into one byte buffer. Buffer
growth requests do not exceed the existing inline limit. Once serialization
crosses the limit, the partial buffer is released and remaining bytes are only
counted. The rejection retains its exact encoded `size_bytes`, path, limit and
artifact/reference advice, and still occurs before connection or submission.
Checked counting and fallible reservation reject overflow/allocation failure.
JSON values, UTF-8 encoding, escaping and boundary inclusivity are unchanged.

HTTP headers contain the byte length but no body. The existing sender uses
borrowed `IoSlice` buffers and `write_vectored`, advancing slices on partial
writes without constructing another complete request. This also avoids forcing
a separate header-only write before each small JSON body. The same absolute
deadline covers all write attempts and response reads, including empty buffers;
interrupt handling and failure messages remain unchanged. The encoded body is
explicitly released before receiving the response. Artifact upload retains its
file streaming path and uses the same sender through a one-slice adapter.

Routes, public result/report shapes, completion/context/risk gates and response
budgets do not change. Faults after sending a mutating request remain unknown,
with no automatic replay or permission for later steps. Serialization/sanitization
failures remain pre-submission failures. Algorithms remain owned by Solver;
Engine/Agent continue to execute admitted tasks. No GUI or backend implementation
is replaced by JavaScript, and no new public SDK abstraction is introduced.

## Tests

Fifteen new native SDK tests verify:

- Byte equality against serde_json across scalars, arrays, objects, unusual keys,
  UTF-8 and escaped/control strings; eight small writer limits per value shape.
- Bounded growth, release on overflow, continued exact counting without storage,
  checked arithmetic, the exact 8,000,000-byte boundary and one-byte overflow.
- Escaped Unicode oversize diagnostics count the entire 8,800,002-byte encoding
  without retaining it or echoing request contents into the error.
- Headers remain small and body-free; actual GET/HEAD/OPTIONS/POST/PUT/PATCH/DELETE
  requests preserve absent/null and escaped/Unicode body bytes and byte lengths.
- An exact-limit Unicode body arrives complete across many socket reads.
- Oversize bodies, injected paths/tokens and expired deadlines never connect;
  tokens are not exposed in their errors.
- An unacknowledged body exhausts its original deadline; header-only peer
  disconnects remain unknown. Neither path opens a replay connection.
- A lost Unicode project-write reply stops following steps with no successful
  binding; vectored writes preserve borrowed buffers and handle empty slices.
- Expired empty/nonempty vector writes send nothing and cannot bypass the deadline.

Old request fixtures remain test-only complete-wire compositions. Their socket
readers now collect declared bodies instead of assuming a single read returns
the full request. No global allocator hooks or retained benchmark payloads are
added; these are bounded transport/ownership tests, not an RSS benchmark.

The two real SDK/CLI result-persistence chains now include an 8192-repeat research
note with Chinese/Arabic text, newlines, tabs, quotes, backslashes and literal
template-looking data. Each requested model version preserves the entire note
and original 513-node/512-element result. Independent `FL/EA` checks retain
relative `1e-12` tolerance. The fanout chain creates one calculation total and
seven versions; the combined chain creates exactly two caller-requested
calculations and three versions. CLI stdout/report equality and empty stderr
remain checked. Temporary Orchestra/SQLite, two Agents and files are cleaned up.

The first expanded live run exposed a test-harness issue: its auxiliary HTTP
reader silently stopped at 2 MiB, truncating the full version list. The fixture
was not reduced. The helper now has an explicit 16 MiB test-only wire cap and
rejects over-cap data rather than parsing truncation. The expanded regression
asserts the version list really exceeds the former 2 MiB cap. Production response
limits are unchanged. The final complete live rerun passes.

## Verification

macOS ARM64 source regression:

- Headless SDK: 432 unit tests in 34.22 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete real-service suite: 43/43 in 19.96 s, all passed.
- CLI binaries: 182 and 16 unit tests; selected integrations: surface 1,
  execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass. A final 33-test HTTP compatibility rerun also
  passes after compacting the test-only wire fixture helper.

Full selected targets have zero ignored/filtered tests. Durations are test
execution observations, not comparable throughput or latency benchmarks.
The live suite exercises the backend; no new full Elixir/Linux suite is claimed.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. The
tensor includes native `verified` evidence without qualification promotion:
zero structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
Daji readiness remains blocked. Source/document limits remain 800/2000 lines
with zero tracked organization debt. All 224 exact version contracts match 3.5.0.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

The inline writer bounds its own retained encoded buffer, not total process
memory. Caller payloads, field selection clones, JSON parser results, independent
public raw/result mirrors, concurrent branches and reports still allocate.
Serialization still traverses oversized data to provide its exact size; this is
not a CPU preemption or wall-time guarantee. The network deadline starts after
preparation unless supplied explicitly, and cannot interrupt serialization.
Allocator overhead, socket buffers and kernel/platform vector-write behavior
are outside this bound. No total RSS cap, OOM guarantee, streaming JSON source,
throughput/peak-RSS improvement number or 1M qualification is claimed.

Independent clients in `sdks/`, installed GUI/Agent, authenticated/durable
deployments, cross-language parity and general scientific qualification remain
separate. Evidence is native `verified` only, with no grade promotion. Installed
local baseline remains 3.4.0. No App rebuild/install, version bump, commit, push
or remote run was requested or performed in this turn.
