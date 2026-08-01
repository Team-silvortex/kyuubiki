# Model Collaboration SDK

The model collaboration layer lets different large-model providers plan
Kyuubiki work through one stable, provider-neutral contract. It is a planning
and normalization layer over the Headless SDK, not a fourth execution runtime.

## Boundary

The collaboration layer owns:

- policy-filtered projection of the authoritative Headless action catalog
- OpenAI, Anthropic, Gemini, and canonical tool-definition shapes
- normalization of provider tool calls into one workflow proposal
- recursive redaction of credential-shaped context before provider handoff
- proposal compilation into the existing Headless execution batch and plan
- stable errors for malformed responses, invalid arguments, and policy blocks

It deliberately does not own:

- provider HTTP clients, model selection, retries, billing, or API keys
- credential persistence
- direct executor dispatch
- operator implementation or package authoring
- Workbench DOM automation

Applications can therefore use an official provider client, a self-hosted
OpenAI-compatible service, or a private model gateway without changing the
Kyuubiki execution contract.

For a model starting from documentation rather than an existing integration,
begin at [`../llms.txt`](../llms.txt) and
[`model-research-bootstrap.json`](model-research-bootstrap.json), then follow
[Model Research Onboarding](model-research-onboarding.html).

## Four Separate Surfaces

| Surface | Primary role | Runtime owner |
| --- | --- | --- |
| Model collaboration | Translate model tools into an untrusted proposal | Integrator process |
| Headless SDK | Control, validate, plan, submit, observe, and recover | Rust, Python, or Elixir client |
| Rust Operator SDK | Author and package executable operators | Rust agent/runtime |
| PWDT | Automate product-owned GUI actions | WebView WASM Python runtime |

A model can use the collaboration layer to prepare Headless work, but it does
not gain Operator SDK authority or bypass PWDT and GUI ownership rules.

## Safe Flow

1. Create a `ModelCollaborationSession` with a narrow objective and policy.
2. Run the selected SDK's model research bootstrap preflight and stop unless
   its report says `ready_for_planning: true`.
3. Call `build_model_collaboration_request()` with sanitized project context.
4. Send its instructions and provider-shaped tools through the caller's model
   client.
5. Call `normalize_model_response()` on the complete provider response.
6. Call `compile_model_proposal()` before considering execution.
7. Reject compilations whose `ok` field is false.
8. Present every confirmation in the compiled Headless plan to the controlling
   policy or human operator.
9. Dispatch only through an existing Headless executor after those gates pass.

Model output is never executable authority. Tool calls are untrusted input.

All three official SDKs provide bounded execution bridges over their existing
Headless Session clients. Every gated step must match an exact caller-issued
`kyuubiki.model-plan-approval/v1` before any network access, and a caller-owned
verifier must authenticate that approval independently of model output.
Execution returns a
`kyuubiki.model-research-execution-receipt/v1`; a failed receipt preserves
completed steps and the failing step without claiming workflow completion.

Cross-turn research uses `kyuubiki.model-research-frontier/v1`. A caller-owned
receipt verifier must authenticate each execution receipt before the frontier
can advance. The frontier-generated proposal carries the real submission
`job_id` into `job_wait`, then carries the same binding into `result_fetch`.
Completed result retrieval stops at `ready_to_validate`; numerical validity is
still established by the existing validation and research-bundle contracts.
Persisted frontier state is also untrusted, so a separate caller-owned frontier
verifier gates proposal generation and every subsequent transition.

The official SDKs close that handoff with
`kyuubiki.model-research-validation-report/v1`. The validator authenticates the
frontier and `result_fetch` receipt, enforces workflow/job identity, validates
the result against the resolved graph, and optionally validates a retained
material research bundle. Its strongest automated stage is
`screening_bundle_validated`; the report always states
`screening_only_not_qualification` and requires external validation.

## Default Policy

`ModelCollaborationPolicy::default()` exposes only service-backed actions with
normal risk. It also limits proposals to 12 calls and sanitized context to 64
KiB. Browser, sensitive, and destructive actions remain hidden unless the
integrator explicitly enables them.

Allow lists can narrow the catalog by action and category. Empty allow lists
mean "all actions that pass the remaining policy gates", not unrestricted
execution.

## Provider Shapes

- OpenAI mode uses Responses API function tools and `function_call` output
  items. OpenAI Chat mode emits the nested `function` tools used by Chat
  Completions and many self-hosted OpenAI-compatible gateways; matching
  `tool_calls` responses normalize through the same adapter.
- Anthropic uses `input_schema` tool definitions and `tool_use` content blocks.
- Gemini uses `functionDeclarations` and accepts both Generate Content
  `functionCall` parts and Interactions `function_call` steps.
- Canonical mode reads `kyuubiki.model-workflow-proposal/v1` directly.

Call IDs are preserved so an integrator can correlate provider turns and tool
results. Arguments must decode to JSON objects; free-form text is not silently
treated as an action.

## Canonical Proposal

The portable proposal schema is
[`schemas/model-workflow-proposal.schema.json`](../schemas/model-workflow-proposal.schema.json).
A compact fixture is available at
[`schemas/examples.model-workflow-proposal.json`](../schemas/examples.model-workflow-proposal.json).
The matching cross-language session policy lives in
[`schemas/model-collaboration-session.schema.json`](../schemas/model-collaboration-session.schema.json),
with a service-only fixture at
[`schemas/examples.model-collaboration-session.json`](../schemas/examples.model-collaboration-session.json).

```json
{
  "schema_version": "kyuubiki.model-workflow-proposal/v1",
  "session_id": "session.research.001",
  "summary": "Check the service before planning a study.",
  "calls": [
    {
      "id": "call-001",
      "action": "service_health",
      "payload": {},
      "reason": "Fail early when the control plane is unavailable."
    }
  ]
}
```

The schema intentionally describes a proposal rather than a provider response.
Provider envelopes change over time; Kyuubiki retains one stable internal file
format and updates adapters around it.

## Rust Headless Entry Points

The official integration surface lives in `sdks/rust` and exports:

- `rust_headless_model_tools()`
- `inspect_model_research_bootstrap()`
- `build_model_collaboration_request()`
- `project_model_tools()`
- `normalize_model_response()`
- `build_model_headless_plan()`
- `sanitize_model_context()`

It builds an inspectable `ModelHeadlessPlan`; the embedding application remains
responsible for confirmations and dispatch through ordinary Headless clients.

The internal protocol reference in `workers/rust/crates/headless-sdk` exports:

- `model_collaboration_tools()`
- `build_model_collaboration_request()`
- `project_model_tools()`
- `normalize_model_response()`
- `compile_model_proposal()`
- `sanitize_model_context()`

The provider adapters are pure transformations and are suitable for fixture,
fuzz, and cross-provider conformance testing without network access.

## Rust Operator Authoring

`workers/rust/crates/operator-sdk` exposes a separate model-readable authoring
manifest and `kyuubiki.operator-model-draft/v1`. A draft may describe an
operator descriptor, JSON schemas, Rust handler shape, and algorithm outline,
but it cannot load a library, activate a package, or claim qualification.
`validate_operator_model_draft()` must pass before a human or controlled
code-generation pipeline implements and packages the Rust operator. See
[Operator SDK](operator-sdk.md).

## Python and Elixir Entry Points

The Python SDK exports dictionary-first equivalents:

- `inspect_model_research_bootstrap()`
- `headless_model_tools()`
- `build_model_collaboration_request()`
- `normalize_model_response()`
- `build_model_headless_plan()`
- `sanitize_model_context()`

The Elixir SDK exposes the same flow through
`KyuubikiSdk.ModelCollaboration.tools/1`, `build_request/3`,
`normalize_response/3`, `build_plan/2`, and `sanitize_context/1`. Its public
root module also provides concise delegates for the request, normalization, and
plan operations. `KyuubikiSdk.ModelResearchBootstrap.inspect/3` provides the
same fail-closed readiness report used by Rust and Python.

Both adapters consume the same repository session/proposal fixtures and retain
the Rust policy defaults. Dynamic-language type errors are converted into
structured SDK validation failures rather than escaping from the integration
boundary.

## Current Maturity

The Rust reference protocol, all three official Headless adapters, and the Rust
Operator SDK draft validator are implemented and covered by offline conformance
tests. Rust, Python, and Elixir consume the same session and proposal schemas,
provider shapes, default policy, runtime boundary, and confirmation semantics.
Provider networking remains intentionally outside every SDK adapter.

## Security Notes

- Context keys resembling tokens, secrets, passwords, authorization headers,
  credentials, and private keys are redacted recursively.
- Bearer-token string values are also redacted.
- Redacted JSON-pointer paths remain visible for audit without retaining the
  secret value.
- The caller must still minimize data before constructing context. Redaction is
  a final guard, not permission to send an entire project to a provider.
- Provider retention and regional-processing settings remain the integrator's
  responsibility.
- Sensitive and destructive Headless actions retain their existing explicit
  confirmation requirements even when policy allows the model to propose them.

## Compatibility References

- [OpenAI Responses API reference](https://platform.openai.com/docs/api-reference/responses)
- [Anthropic tool use](https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/implement-tool-use)
- [Gemini function calling](https://ai.google.dev/gemini-api/docs/function-calling)
