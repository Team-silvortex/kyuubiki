from __future__ import annotations

import json
import pathlib
import unittest

from kyuubiki_sdk import (
    MODEL_COLLABORATION_SCHEMA_VERSION,
    ModelCollaborationValidationError,
    build_model_collaboration_request,
    build_model_headless_plan,
    default_model_collaboration_policy,
    headless_model_tools,
    normalize_model_response,
)


SCHEMAS_DIR = pathlib.Path(__file__).resolve().parents[3] / "schemas"
ROOT_DIR = SCHEMAS_DIR.parent


def load_json(filename: str) -> dict:
    return json.loads((SCHEMAS_DIR / filename).read_text(encoding="utf-8"))


def session(allow_sensitive: bool = False, service_only: bool = True) -> dict:
    policy = default_model_collaboration_policy()
    policy.update({"allow_sensitive": allow_sensitive, "service_only": service_only})
    return {
        "schema_version": MODEL_COLLABORATION_SCHEMA_VERSION,
        "session_id": "session.python-model",
        "workflow_id": "workflow.python-model",
        "objective": "Discover the runtime and submit one bounded solve",
        "language": "en",
        "created_at": "2026-08-01T00:00:00Z",
        "policy": policy,
    }


class ModelCollaborationTest(unittest.TestCase):
    def test_default_catalog_is_read_only_and_service_owned(self) -> None:
        tools = headless_model_tools()
        self.assertTrue(any(tool["action"] == "service_health" for tool in tools))
        self.assertTrue(all(tool["risk"] == "normal" for tool in tools))
        self.assertTrue(all(tool["runtime"] == "service" for tool in tools))

    def test_service_only_excludes_direct_solver(self) -> None:
        service_tools = headless_model_tools(session(True)["policy"])
        self.assertTrue(any(tool["action"] == "fem_submit" for tool in service_tools))
        self.assertFalse(any(tool["action"] == "direct_solver_rpc" for tool in service_tools))
        all_tools = headless_model_tools(session(True, False)["policy"])
        self.assertTrue(any(tool["action"] == "direct_solver_rpc" for tool in all_tools))

    def test_provider_requests_share_catalog_and_redaction(self) -> None:
        for provider in ("openai", "openai_chat", "anthropic", "gemini", "canonical"):
            request = build_model_collaboration_request(
                provider,
                session(),
                {"authorization": "Bearer secret-value", "nested": {"api_key": "secret"}},
            )
            self.assertEqual(request["context"]["authorization"], "[REDACTED]")
            self.assertEqual(request["context"]["nested"]["api_key"], "[REDACTED]")
            self.assertTrue(request["tools"])

    def test_normalizes_provider_calls(self) -> None:
        responses = [
            ("openai", {"output": [{"type": "function_call", "call_id": "o1", "name": "service_health", "arguments": "{}"}]}),
            ("openai_chat", {"choices": [{"message": {"tool_calls": [{"id": "c1", "function": {"name": "service_health", "arguments": "{}"}}]}}]}),
            ("anthropic", {"content": [{"type": "tool_use", "id": "a1", "name": "service_health", "input": {}}]}),
            ("gemini", {"candidates": [{"content": {"parts": [{"functionCall": {"id": "g1", "name": "service_health", "args": {}}}]}}]}),
            ("gemini", {"steps": [{"type": "function_call", "id": "g2", "name": "service_health", "arguments": {}}]}),
            ("canonical", {"schema_version": "kyuubiki.model-workflow-proposal/v1", "session_id": "session.python-model", "calls": [{"action": "service_health", "payload": {}}]}),
        ]
        for provider, response in responses:
            proposal = normalize_model_response(provider, "session.python-model", response)
            self.assertEqual(proposal["calls"][0]["action"], "service_health")

    def test_oversized_context_fails_closed(self) -> None:
        bounded = session()
        bounded["policy"]["max_context_bytes"] = 16
        with self.assertRaises(ModelCollaborationValidationError) as context:
            build_model_collaboration_request(
                "openai",
                bounded,
                {"token": "secret", "payload": "this remains too large"},
            )
        self.assertIn("policy allows 16", str(context.exception))

    def test_sensitive_plan_requires_confirmation(self) -> None:
        proposal = {
            "schema_version": "kyuubiki.model-workflow-proposal/v1",
            "session_id": "session.python-model",
            "summary": "Submit one solve",
            "calls": [{"action": "fem_submit", "payload": {"solve_kind": "thermal_frame_3d", "payload": {"model": {}}}}],
        }
        plan = build_model_headless_plan(session(True), proposal)
        self.assertTrue(plan["ok"], plan["issues"])
        self.assertFalse(plan["ready_without_confirmation"])
        self.assertTrue(plan["steps"][0]["requires_confirmation"])

    def test_hidden_and_malformed_calls_fail_closed(self) -> None:
        proposal = {
            "schema_version": "kyuubiki.model-workflow-proposal/v1",
            "session_id": "session.python-model",
            "calls": [{"action": "fem_submit", "payload": {"solve_kind": "thermal_frame_3d"}}],
        }
        self.assertFalse(build_model_headless_plan(session(), proposal)["ok"])
        self.assertFalse(build_model_headless_plan(session(True), proposal)["ok"])
        with self.assertRaises(ModelCollaborationValidationError):
            normalize_model_response(
                "openai",
                "session.python-model",
                {"output": [{"type": "function_call", "name": "service_health", "arguments": "[]"}]},
            )
        with self.assertRaises(ModelCollaborationValidationError):
            headless_model_tools({"allow_sensitive": "yes"})
        with self.assertRaises(ModelCollaborationValidationError):
            normalize_model_response("openai", "session.python-model", {"output": {}})

    def test_model_research_bootstrap_reaches_valid_first_plan(self) -> None:
        bootstrap = json.loads(
            (ROOT_DIR / "docs/model-research-bootstrap.json").read_text(encoding="utf-8")
        )
        for document in bootstrap["required_documents"]:
            self.assertTrue((ROOT_DIR / document["path"]).is_file())
        first = bootstrap["first_research"]
        fixture_session = json.loads(
            (ROOT_DIR / first["session_fixture"]).read_text(encoding="utf-8")
        )
        proposal = json.loads(
            (ROOT_DIR / first["proposal_fixture"]).read_text(encoding="utf-8")
        )
        request = build_model_collaboration_request("canonical", fixture_session, {})
        plan = build_model_headless_plan(fixture_session, proposal)
        self.assertEqual(request["output_contract"], proposal["schema_version"])
        self.assertTrue(plan["ok"], plan["issues"])
        self.assertFalse(plan["ready_without_confirmation"])


if __name__ == "__main__":
    unittest.main()
