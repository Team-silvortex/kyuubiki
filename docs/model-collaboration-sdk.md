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

## Four Separate Surfaces

| Surface | Primary role | Runtime owner |
| --- | --- | --- |
| Model collaboration | Translate model tools into an untrusted proposal | Integrator process |
| Headless SDK | Control, validate, plan, submit, observe, and recover | Rust, Python, or Elixir client |
| Worker SDK | Author and package executable operators | Rust agent/runtime |
| PWDT | Automate product-owned GUI actions | WebView WASM Python runtime |

A model can use the collaboration layer to prepare Headless work, but it does
not gain Worker SDK authority or bypass PWDT and GUI ownership rules.

## Safe Flow

1. Create a `ModelCollaborationSession` with a narrow objective and policy.
2. Call `build_model_collaboration_request()` with sanitized project context.
3. Send its instructions and provider-shaped tools through the caller's model
   client.
4. Call `normalize_model_response()` on the complete provider response.
5. Call `compile_model_proposal()` before considering execution.
6. Reject compilations whose `ok` field is false.
7. Present every confirmation in the compiled Headless plan to the controlling
   policy or human operator.
8. Dispatch only through an existing Headless executor after those gates pass.

Model output is never executable authority. Tool calls are untrusted input.

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

## Rust Entry Points

The reference implementation lives in
`workers/rust/crates/headless-sdk` and exports:

- `model_collaboration_tools()`
- `build_model_collaboration_request()`
- `project_model_tools()`
- `normalize_model_response()`
- `compile_model_proposal()`
- `sanitize_model_context()`

The provider adapters are pure transformations and are suitable for fixture,
fuzz, and cross-provider conformance testing without network access.

## Current Maturity

The Rust reference protocol and provider adapters are implemented and covered
by offline conformance tests. Python and Elixir provider adapters are not yet
implemented; they should consume the shared session and proposal schemas rather
than fork provider-specific workflow semantics. Until that parity lands, model
collaboration is a Rust-first SDK capability, while ordinary Headless control
remains available in all three official SDK families.

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
