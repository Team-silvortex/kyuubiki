from __future__ import annotations

import json
from collections.abc import Mapping
from typing import Any

from .errors import ModelCollaborationValidationError


MODEL_COLLABORATION_SCHEMA_VERSION = "kyuubiki.model-collaboration/v1"
MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION = "kyuubiki.model-workflow-proposal/v1"
MODEL_HEADLESS_PLAN_SCHEMA_VERSION = "kyuubiki.model-headless-plan/v1"
MODEL_PROVIDERS = ("openai", "openai_chat", "anthropic", "gemini", "canonical")


def default_model_collaboration_policy() -> dict[str, Any]:
    return {
        "allowed_actions": [],
        "allowed_categories": [],
        "max_steps": 12,
        "max_context_bytes": 64 * 1024,
        "service_only": True,
        "allow_sensitive": False,
        "allow_destructive": False,
    }


def headless_model_tools(policy: Mapping[str, Any] | None = None) -> list[dict[str, Any]]:
    normalized = _validate_policy(policy)
    return [tool for tool in _base_model_tools() if _policy_allows_tool(normalized, tool)]


def build_model_collaboration_request(
    provider: str,
    session: Mapping[str, Any],
    context: Any,
) -> dict[str, Any]:
    normalized_session = _validate_session(session)
    if provider not in MODEL_PROVIDERS:
        _fail(f"unsupported model provider: {provider}")
    sanitized, redacted_paths = sanitize_model_context(context)
    context_bytes = len(
        json.dumps(sanitized, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    )
    policy = normalized_session["policy"]
    if context_bytes > policy["max_context_bytes"]:
        _fail(
            f"sanitized model context uses {context_bytes} bytes; policy allows "
            f"{policy['max_context_bytes']}"
        )
    tools = headless_model_tools(policy)
    if not tools:
        _fail("model collaboration policy exposes no Headless tools")
    return {
        "schema_version": MODEL_COLLABORATION_SCHEMA_VERSION,
        "provider": provider,
        "session": normalized_session,
        "instructions": [
            f"Plan only for this objective: {normalized_session['objective']}",
            "Use only supplied Headless tools and never invent action names.",
            f"Return no more than {policy['max_steps']} tool calls.",
            "Return tool calls as an untrusted proposal; never claim that execution occurred.",
        ],
        "context": sanitized,
        "redacted_paths": redacted_paths,
        "tools": project_model_tools(provider, tools),
        "output_contract": MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    }


def build_model_headless_plan(
    session: Mapping[str, Any], proposal: Mapping[str, Any]
) -> dict[str, Any]:
    normalized_session = _validate_session(session)
    policy = normalized_session["policy"]
    available = {tool["action"]: tool for tool in headless_model_tools(policy)}
    issues: list[str] = []
    if not isinstance(proposal, Mapping):
        _fail("model proposal must be a JSON object")
    if proposal.get("schema_version") != MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION:
        issues.append(f"unsupported proposal schema_version: {proposal.get('schema_version')}")
    if proposal.get("session_id") != normalized_session["session_id"]:
        issues.append("proposal session_id does not match collaboration session")
    calls = proposal.get("calls", [])
    if not isinstance(calls, list):
        calls = []
        issues.append("model proposal calls must be an array")
    if not calls:
        issues.append("model proposal contains no tool calls")
    if len(calls) > policy["max_steps"]:
        issues.append(
            f"model proposal contains {len(calls)} calls; policy allows {policy['max_steps']}"
        )
    steps = [
        _plan_step(index, call, available, issues)
        for index, call in enumerate(calls, start=1)
    ]
    issues = sorted(set(issues))
    return {
        "schema_version": MODEL_HEADLESS_PLAN_SCHEMA_VERSION,
        "session_id": normalized_session["session_id"],
        "workflow_id": normalized_session["workflow_id"],
        "ok": not issues,
        "ready_without_confirmation": not issues
        and all(not step["requires_confirmation"] for step in steps),
        "issues": issues,
        "steps": steps,
    }


def project_model_tools(provider: str, tools: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if provider not in MODEL_PROVIDERS:
        _fail(f"unsupported model provider: {provider}")
    definitions = []
    for tool in tools:
        parameters = _tool_parameters(tool["required_payload_keys"])
        if provider == "openai":
            definition = {
                "type": "function",
                "name": tool["action"],
                "description": tool["description"],
                "parameters": parameters,
                "strict": False,
            }
        elif provider == "openai_chat":
            definition = {
                "type": "function",
                "function": {
                    "name": tool["action"],
                    "description": tool["description"],
                    "parameters": parameters,
                    "strict": False,
                },
            }
        elif provider == "anthropic":
            definition = {
                "name": tool["action"],
                "description": tool["description"],
                "input_schema": parameters,
            }
        elif provider == "gemini":
            definition = {
                "name": tool["action"],
                "description": tool["description"],
                "parameters": parameters,
            }
        else:
            definition = dict(tool)
        definitions.append(definition)
    return [{"functionDeclarations": definitions}] if provider == "gemini" else definitions


def normalize_model_response(
    provider: str, session_id: str, response: Mapping[str, Any]
) -> dict[str, Any]:
    if provider == "canonical":
        return _normalize_canonical(session_id, response)
    calls: list[dict[str, Any]] = []
    summaries: list[str] = []
    if provider in ("openai", "openai_chat"):
        _collect_openai(response, calls, summaries)
    elif provider == "anthropic":
        _collect_anthropic(response, calls, summaries)
    elif provider == "gemini":
        _collect_gemini(response, calls, summaries)
    else:
        _fail(f"unsupported model provider: {provider}")
    if not calls:
        _fail("provider response contains no supported tool calls")
    return {
        "schema_version": MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
        "session_id": session_id,
        "summary": "\n".join(summaries),
        "calls": calls,
    }


def sanitize_model_context(context: Any) -> tuple[Any, list[str]]:
    redacted_paths: list[str] = []

    def sanitize(value: Any, path: str) -> Any:
        if isinstance(value, Mapping):
            result = {}
            for key, child in value.items():
                text_key = str(key)
                next_path = f"{path}/{_escape_pointer(text_key)}"
                if _sensitive_key(text_key):
                    redacted_paths.append(next_path)
                    result[text_key] = "[REDACTED]"
                else:
                    result[text_key] = sanitize(child, next_path)
            return result
        if isinstance(value, list):
            return [sanitize(child, f"{path}/{index}") for index, child in enumerate(value)]
        if isinstance(value, str) and value.lstrip().lower().startswith("bearer "):
            redacted_paths.append(path or "/")
            return "[REDACTED]"
        return value

    return sanitize(context, ""), redacted_paths


def _normalize_policy(policy: Mapping[str, Any] | None) -> dict[str, Any]:
    normalized = default_model_collaboration_policy()
    if policy is not None:
        if not isinstance(policy, Mapping):
            _fail("model collaboration policy must be a JSON object")
        normalized.update(policy)
    return normalized


def _validate_session(session: Mapping[str, Any]) -> dict[str, Any]:
    if not isinstance(session, Mapping):
        _fail("model collaboration session must be a JSON object")
    errors = []
    if session.get("schema_version") != MODEL_COLLABORATION_SCHEMA_VERSION:
        errors.append(f"unsupported session schema_version: {session.get('schema_version')}")
    for key in ("session_id", "workflow_id", "objective", "created_at"):
        value = session.get(key)
        if not isinstance(value, str) or not value.strip():
            errors.append(f"{key} is required")
    policy = _validate_policy(session.get("policy"))
    for key in ("max_steps", "max_context_bytes"):
        value = policy.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            errors.append(f"{key} must be a positive integer")
    for key in ("allowed_actions", "allowed_categories"):
        if not isinstance(policy.get(key), list):
            errors.append(f"{key} must be an array")
    if errors:
        raise ModelCollaborationValidationError(errors)
    normalized = dict(session)
    normalized["language"] = session.get("language", "en")
    normalized["policy"] = policy
    return normalized


def _validate_policy(policy: Mapping[str, Any] | None) -> dict[str, Any]:
    normalized = _normalize_policy(policy)
    errors = []
    for key in ("allowed_actions", "allowed_categories"):
        value = normalized.get(key)
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            errors.append(f"{key} must be an array of strings")
    for key in ("service_only", "allow_sensitive", "allow_destructive"):
        if not isinstance(normalized.get(key), bool):
            errors.append(f"{key} must be a boolean")
    for key in ("max_steps", "max_context_bytes"):
        value = normalized.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            errors.append(f"{key} must be a positive integer")
    if errors:
        raise ModelCollaborationValidationError(errors)
    return normalized


def _plan_step(
    index: int,
    call: Any,
    available: dict[str, dict[str, Any]],
    issues: list[str],
) -> dict[str, Any]:
    if not isinstance(call, Mapping):
        issues.append(f"step {index} must be a JSON object")
        call = {}
    action = call.get("action")
    if not isinstance(action, str) or not action:
        action = ""
        issues.append(f"step {index} action is required")
    tool = available.get(action)
    if tool is None:
        issues.append(f"step {index} action {action} is unknown or blocked by policy")
    payload = call.get("payload", {})
    if not isinstance(payload, Mapping):
        issues.append(f"step {index} ({action}) payload must be a JSON object")
        payload = {}
    if tool:
        for key in tool["required_payload_keys"]:
            if not _has_present_value(payload, key):
                issues.append(f"step {index} ({action}) is missing required payload key {key}")
        _validate_known_payload(index, action, payload, issues)
    risk = tool["risk"] if tool else "normal"
    return {
        "index": index,
        "action": action,
        "category": tool["category"] if tool else None,
        "risk": risk,
        "payload": dict(payload),
        "requires_confirmation": risk != "normal",
        "confirmation_reason": _confirmation_reason(risk),
        "output_keys": list(tool["output_keys"]) if tool else [],
    }


def _collect_openai(
    response: Mapping[str, Any], calls: list[dict[str, Any]], summaries: list[str]
) -> None:
    for item in _map_items(response.get("output")):
        if item.get("type") == "function_call":
            calls.append(_parse_call(item.get("call_id") or item.get("id"), item.get("name"), item.get("arguments")))
        elif item.get("type") == "message":
            for part in _map_items(item.get("content")):
                if part.get("type") in ("output_text", "text"):
                    _collect_text(part.get("text"), summaries)
    for choice in _map_items(response.get("choices")):
        message = choice.get("message", {}) if isinstance(choice.get("message"), Mapping) else {}
        _collect_text(message.get("content"), summaries)
        for item in _map_items(message.get("tool_calls")):
            function = item.get("function", {}) if isinstance(item.get("function"), Mapping) else {}
            calls.append(_parse_call(item.get("id"), function.get("name"), function.get("arguments")))


def _collect_anthropic(
    response: Mapping[str, Any], calls: list[dict[str, Any]], summaries: list[str]
) -> None:
    for block in _map_items(response.get("content")):
        if block.get("type") == "tool_use":
            calls.append(_parse_call(block.get("id"), block.get("name"), block.get("input")))
        elif block.get("type") == "text":
            _collect_text(block.get("text"), summaries)


def _collect_gemini(
    response: Mapping[str, Any], calls: list[dict[str, Any]], summaries: list[str]
) -> None:
    for candidate in _map_items(response.get("candidates")):
        content = candidate.get("content", {}) if isinstance(candidate.get("content"), Mapping) else {}
        for part in _map_items(content.get("parts")):
            _collect_text(part.get("text"), summaries)
            function = part.get("functionCall")
            if function:
                calls.append(_parse_call(function.get("id"), function.get("name"), function.get("args")))
    for step in _map_items(response.get("steps")):
        if step.get("type") == "function_call":
            calls.append(_parse_call(step.get("id"), step.get("name"), step.get("arguments")))


def _normalize_canonical(session_id: str, response: Mapping[str, Any]) -> dict[str, Any]:
    if response.get("session_id") != session_id:
        _fail("canonical proposal session_id does not match requested session")
    if response.get("schema_version") != MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION:
        _fail("canonical proposal has unsupported schema_version")
    calls = []
    for call in response.get("calls", []):
        if not isinstance(call, Mapping):
            _fail("canonical proposal calls must be JSON objects")
        normalized = dict(call)
        normalized["payload"] = _parse_arguments(call.get("payload", {}))
        calls.append(normalized)
    proposal = dict(response)
    proposal["calls"] = calls
    return proposal


def _parse_call(call_id: Any, name: Any, arguments: Any) -> dict[str, Any]:
    if not isinstance(name, str) or not name:
        _fail("provider tool call is missing a string name")
    if arguments is None:
        _fail(f"provider tool call {name} is missing arguments")
    return {"id": call_id if isinstance(call_id, str) else None, "action": name, "payload": _parse_arguments(arguments), "reason": None}


def _parse_arguments(arguments: Any) -> dict[str, Any]:
    if isinstance(arguments, str):
        try:
            arguments = json.loads(arguments)
        except json.JSONDecodeError as error:
            _fail(f"provider tool arguments are invalid JSON: {error.msg}")
    if not isinstance(arguments, Mapping):
        _fail("provider tool arguments must decode to a JSON object")
    return dict(arguments)


def _tool_parameters(required_keys: list[str]) -> dict[str, Any]:
    return {
        "type": "object",
        "properties": {key: {"description": f"Required `{key}` payload"} for key in required_keys},
        "required": required_keys,
        "additionalProperties": True,
    }


def _policy_allows_tool(policy: Mapping[str, Any], tool: Mapping[str, Any]) -> bool:
    return (
        (not policy["allowed_actions"] or tool["action"] in policy["allowed_actions"])
        and (not policy["allowed_categories"] or tool["category"] in policy["allowed_categories"])
        and (policy["allow_sensitive"] or tool["risk"] != "sensitive")
        and (policy["allow_destructive"] or tool["risk"] != "destructive")
        and (not policy["service_only"] or tool["runtime"] == "service")
    )


def _has_present_value(payload: Mapping[str, Any], key: str) -> bool:
    value = payload.get(key)
    return value is not None and (not isinstance(value, str) or bool(value.strip()))


def _validate_known_payload(
    index: int,
    action: str,
    payload: Mapping[str, Any],
    issues: list[str],
) -> None:
    string_keys: tuple[str, ...] = ()
    object_keys: tuple[str, ...] = ()
    if action in ("fem_submit", "direct_solver_rpc"):
        string_keys, object_keys = ("solve_kind",), ("payload",)
    elif action == "workflow_submit_catalog":
        string_keys, object_keys = ("workflow_id",), ("input_artifacts",)
    elif action == "workflow_submit_graph":
        object_keys = ("graph", "input_artifacts")
    elif action in ("operator_task_prepare", "operator_task_execute"):
        object_keys = ("task",)
    elif action in ("operator_task_batch_prepare", "operator_task_batch_execute"):
        object_keys = ("batch",)
    elif action in ("job_wait", "result_fetch", "job_cancel"):
        string_keys = ("job_id",)
    elif action == "result_chunk_fetch":
        string_keys = ("job_id", "kind")

    for key in string_keys:
        value = payload.get(key)
        if value is not None and (not isinstance(value, str) or not value.strip()):
            issues.append(
                f"step {index} ({action}) payload key {key} must be a non-empty string"
            )
    for key in object_keys:
        value = payload.get(key)
        if value is not None and not isinstance(value, Mapping):
            issues.append(f"step {index} ({action}) payload key {key} must be a JSON object")
    if action == "result_chunk_fetch":
        for key in ("offset", "limit"):
            value = payload.get(key)
            if value is not None and (
                not isinstance(value, int) or isinstance(value, bool) or value < 0
            ):
                issues.append(
                    f"step {index} ({action}) payload key {key} must be an unsigned integer"
                )


def _confirmation_reason(risk: str) -> str | None:
    if risk == "sensitive":
        return "sensitive Headless action requires explicit approval before dispatch"
    if risk == "destructive":
        return "destructive Headless action requires explicit approval before dispatch"
    return None


def _collect_text(value: Any, summaries: list[str]) -> None:
    if isinstance(value, str) and value.strip():
        summaries.append(value.strip())


def _map_items(value: Any) -> list[Mapping[str, Any]]:
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, Mapping)]


def _sensitive_key(key: str) -> bool:
    normalized = key.lower().replace("-", "_").replace(".", "_")
    return any(marker in normalized for marker in ("token", "secret", "password", "api_key", "apikey", "authorization", "credential", "private_key"))


def _escape_pointer(value: str) -> str:
    return value.replace("~", "~0").replace("/", "~1")


def _tool(
    action: str,
    category: str,
    description: str,
    risk: str,
    runtime: str,
    required: tuple[str, ...] = (),
    outputs: tuple[str, ...] = (),
) -> dict[str, Any]:
    return {
        "action": action,
        "category": category,
        "description": description,
        "risk": risk,
        "runtime": runtime,
        "required_payload_keys": list(required),
        "output_keys": list(outputs),
    }


def _base_model_tools() -> list[dict[str, Any]]:
    service = "service"
    return [
        _tool("service_health", "discovery", "Check control-plane health.", "normal", service, outputs=("health",)),
        _tool("protocol_describe", "discovery", "Read protocol compatibility and service endpoints.", "normal", service, outputs=("protocol",)),
        _tool("agents_describe", "discovery", "List reachable agents and capabilities.", "normal", service, outputs=("agents",)),
        _tool("workflow_catalog_list", "discovery", "List centrally owned workflow templates.", "normal", service, outputs=("workflows",)),
        _tool("operator_catalog_list", "discovery", "List workflow operator descriptors.", "normal", service, outputs=("operators",)),
        _tool("fem_submit", "solve", "Submit a FEM solve kind and model payload.", "sensitive", service, ("solve_kind", "payload"), ("job",)),
        _tool("direct_solver_rpc", "solve", "Call a configured solver agent without Orchestra.", "sensitive", "direct", ("solve_kind", "payload"), ("result",)),
        _tool("workflow_submit_catalog", "workflow", "Submit a catalog workflow job.", "sensitive", service, ("workflow_id", "input_artifacts"), ("job",)),
        _tool("workflow_submit_graph", "workflow", "Submit a validated inline workflow graph.", "sensitive", service, ("graph", "input_artifacts"), ("job",)),
        _tool("operator_task_prepare", "task_ir", "Preflight one language-neutral Operator TaskIR envelope.", "normal", service, ("task",), ("preparation",)),
        _tool("operator_task_execute", "task_ir", "Execute one prepared Operator TaskIR envelope.", "sensitive", service, ("task",), ("execution",)),
        _tool("operator_task_batch_prepare", "task_ir", "Preflight an Operator TaskIR batch.", "normal", service, ("batch",), ("preparation",)),
        _tool("operator_task_batch_execute", "task_ir", "Execute an Operator TaskIR batch.", "sensitive", service, ("batch",), ("execution",)),
        _tool("job_wait", "observation", "Poll a job until it reaches a terminal state.", "normal", service, ("job_id",), ("job",)),
        _tool("result_fetch", "observation", "Fetch the retained result bundle for a job.", "normal", service, ("job_id",), ("result",)),
        _tool("result_chunk_fetch", "observation", "Fetch one bounded result chunk.", "normal", service, ("job_id", "kind"), ("chunk",)),
        _tool("job_cancel", "lifecycle", "Cancel a running job after explicit approval.", "destructive", service, ("job_id",), ("job",)),
    ]


def _fail(message: str) -> None:
    raise ModelCollaborationValidationError([message])
