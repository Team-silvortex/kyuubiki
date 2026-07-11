from __future__ import annotations

import json
import pathlib
import unittest
from copy import deepcopy

from kyuubiki_sdk import (
    MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION,
    validate_material_research_bundle,
)


REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
FIXTURE_PATH = REPO_ROOT / "schemas" / "examples.material-research-bundle.json"


class MaterialResearchBundleTest(unittest.TestCase):
    def test_validates_shared_bundle_fixture(self) -> None:
        bundle = json.loads(FIXTURE_PATH.read_text())

        validated = validate_material_research_bundle(bundle)

        self.assertIs(validated, bundle)
        self.assertEqual(validated["schema_version"], MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION)
        self.assertEqual(validated["study"], "heat-spreader")
        self.assertEqual(
            validated["summary"]["winner_candidate_id"],
            "pyrolytic_graphite_in_plane",
        )

    def test_rejects_bad_retained_artifact_schema(self) -> None:
        bundle = json.loads(FIXTURE_PATH.read_text())
        bundle = deepcopy(bundle)
        bundle["chain"]["schema_version"] = "wrong"

        with self.assertRaisesRegex(ValueError, "chain.schema_version"):
            validate_material_research_bundle(bundle)

    def test_rejects_bad_checksum_shape(self) -> None:
        bundle = json.loads(FIXTURE_PATH.read_text())
        bundle["artifact_checksums"]["chain_sha256"] = "not-a-digest"

        with self.assertRaisesRegex(ValueError, "chain_sha256"):
            validate_material_research_bundle(bundle)

    def test_rejects_summary_plan_decision_mismatch(self) -> None:
        bundle = json.loads(FIXTURE_PATH.read_text())
        bundle["next_round_execution_plan"]["decision"] = "repair_validation"

        with self.assertRaisesRegex(ValueError, "next_round_execution_plan.decision"):
            validate_material_research_bundle(bundle)


if __name__ == "__main__":
    unittest.main()
