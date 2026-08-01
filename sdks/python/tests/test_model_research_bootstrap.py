from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from kyuubiki_sdk import inspect_model_research_bootstrap


ROOT_DIR = Path(__file__).resolve().parents[3]


class ModelResearchBootstrapTests(unittest.TestCase):
    def setUp(self) -> None:
        self.bootstrap = json.loads(
            (ROOT_DIR / "docs/model-research-bootstrap.json").read_text(encoding="utf-8")
        )

    def test_repository_bootstrap_is_ready_for_all_official_sdks(self) -> None:
        for sdk in ("rust", "python", "elixir"):
            report = inspect_model_research_bootstrap(
                self.bootstrap, sdk, lambda path: (ROOT_DIR / path).is_file()
            )
            self.assertTrue(report["ready_for_planning"], report["blockers"])
            self.assertEqual(report["execution_authority"], "none_preflight_only")
            self.assertEqual(report["missing_resources"], [])
            self.assertIsNotNone(report["selected_surface"])

    def test_python_report_exposes_native_preflight_entrypoint(self) -> None:
        report = inspect_model_research_bootstrap(
            self.bootstrap, "python", lambda path: (ROOT_DIR / path).is_file()
        )
        self.assertEqual(
            report["selected_surface"]["preflight_path"],
            "sdks/python/kyuubiki_sdk/model_research_bootstrap.py",
        )
        self.assertEqual(
            report["selected_surface"]["inspect"],
            "inspect_model_research_bootstrap",
        )

    def test_missing_and_unsafe_resources_fail_closed(self) -> None:
        missing = inspect_model_research_bootstrap(
            self.bootstrap,
            "python",
            lambda path: path != "llms.txt" and (ROOT_DIR / path).is_file(),
        )
        self.assertFalse(missing["ready_for_planning"])
        self.assertEqual(missing["missing_resources"], ["llms.txt"])

        unsafe_bootstrap = copy.deepcopy(self.bootstrap)
        unsafe_bootstrap["required_documents"][0]["path"] = "../secret"
        unsafe = inspect_model_research_bootstrap(
            unsafe_bootstrap, "python", lambda path: (ROOT_DIR / path).is_file()
        )
        self.assertFalse(unsafe["ready_for_planning"])
        self.assertTrue(
            any("safe project-relative path" in item for item in unsafe["blockers"])
        )

        authority_bootstrap = copy.deepcopy(self.bootstrap)
        authority_bootstrap["preflight"]["execution_authority"] = "model_owned"
        authority = inspect_model_research_bootstrap(
            authority_bootstrap, "python", lambda path: (ROOT_DIR / path).is_file()
        )
        self.assertFalse(authority["ready_for_planning"])
        self.assertTrue(
            any("none_preflight_only" in item for item in authority["blockers"])
        )


if __name__ == "__main__":
    unittest.main()
