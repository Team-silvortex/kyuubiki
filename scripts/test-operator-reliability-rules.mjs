#!/usr/bin/env node
import {
  isBelowMinimumCoverageLevel,
  levelRank,
  qualificationEvidenceErrors,
  qualificationEvidenceKitErrors,
  qualificationPromotionErrors,
  qualificationRoadmapErrors,
} from "./operator-reliability-rules.mjs";
import { operatorReliabilitySchemaVersions } from "./operator-reliability-contracts.mjs";

const failures = [];

function assert(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}

function run() {
  assert(isBelowMinimumCoverageLevel("baseline", "review"), "baseline must fail review gate");
  assert(isBelowMinimumCoverageLevel("smoke", "review"), "smoke must fail review gate");
  assert(!isBelowMinimumCoverageLevel("review", "review"), "review must satisfy review gate");
  assert(
    !isBelowMinimumCoverageLevel("qualification", "review"),
    "qualification must satisfy review gate"
  );
  assert(levelRank("smoke") < levelRank("baseline"), "level order must keep smoke before baseline");
  assert(
    levelRank("review") < levelRank("qualification"),
    "level order must keep review before qualification"
  );

  assert(
    qualificationEvidenceErrors({ evidence: {} }).includes(
      "qualification-level operators must declare evidence.qualification"
    ),
    "missing qualification evidence must fail"
  );
  assert(
    qualificationEvidenceErrors({ evidence: { qualification: { tests: ["virtual.rs"] } } }).includes(
      "evidence.qualification.validation_sources must be non-empty"
    ),
    "partial qualification evidence must fail"
  );
  const fullQualificationEntry = {
    evidence: {
      qualification: {
        validation_sources: ["analytic"],
        convergence_checks: ["mesh_refinement"],
        provenance: ["baseline_id"],
        release_gates: ["release_blocking_regression"],
        tests: ["virtual.rs"],
      },
    },
  };
  assert(
    qualificationEvidenceErrors(fullQualificationEntry).length === 0,
    "complete qualification evidence must validate"
  );

  const manifest = { version_line: "moxi self-test" };
  const operators = new Set(["solve.ok", "solve.low"]);
  const levels = new Map([
    ["solve.ok", "review"],
    ["solve.low", "baseline"],
  ]);
  const roadmap = {
    schema_version: operatorReliabilitySchemaVersions.roadmap,
    version_line: "moxi self-test",
    minimum_candidate_level: "review",
    candidates: [
      {
        candidate_id: "self-test",
        priority: "p0",
        domain: "self_test",
        target_level: "qualification",
        evidence_phase: "planned",
        operator_ids: ["solve.ok"],
        rationale: "self-test candidate",
        primary_blocker: "self-test blocker",
        evidence_gaps: ["gap"],
        required_artifacts: ["artifact"],
        graduation_gate: "gate",
        preferred_validation_lane: "make check-operator-reliability",
        release_gate_impact: "release_blocker",
      },
    ],
  };
  assert(
    qualificationRoadmapErrors(roadmap, manifest, operators, levels).length === 0,
    "valid qualification roadmap must pass"
  );
  assert(
    qualificationRoadmapErrors(
      { ...roadmap, version_line: "moxi wrong" },
      manifest,
      operators,
      levels
    ).includes("version_line must match reliability manifest"),
    "roadmap version mismatch must fail"
  );
  assert(
    qualificationRoadmapErrors(
      { ...roadmap, candidates: [{ ...roadmap.candidates[0], operator_ids: ["solve.missing"] }] },
      manifest,
      operators,
      levels
    ).some((error) => error.includes("is not in reliability manifest")),
    "roadmap unknown operator must fail"
  );
  assert(
    qualificationRoadmapErrors(
      { ...roadmap, candidates: [{ ...roadmap.candidates[0], operator_ids: ["solve.low"] }] },
      manifest,
      operators,
      levels
    ).some((error) => error.includes("is below roadmap minimum review")),
    "roadmap candidate below minimum must fail"
  );
  assert(
    qualificationRoadmapErrors(
      roadmap,
      { ...manifest, minimum_coverage_level: "qualification" },
      operators,
      new Map([
        ["solve.ok", "qualification"],
        ["solve.low", "baseline"],
      ])
    ).includes("minimum_candidate_level review is below manifest minimum qualification"),
    "roadmap minimum below manifest minimum must fail"
  );

  const kits = {
    schema_version: operatorReliabilitySchemaVersions.evidenceKits,
    version_line: "moxi self-test",
    kits: [
      {
        candidate_id: "self-test",
        status: "planned",
        artifact_profile: "analytic",
        operator_ids: ["solve.ok"],
        artifact_requirements: [
          {
            artifact_id: "reference",
            kind: "note",
            gate: "gate",
            artifact_path: "docs/reference.md",
            path_policy: "repo-relative-required-before-qualification",
          },
        ],
      },
    ],
  };
  assert(
    qualificationEvidenceKitErrors(kits, roadmap, manifest).length === 0,
    "valid qualification evidence kit must pass"
  );
  assert(
    qualificationEvidenceKitErrors(
      { ...kits, kits: [{ ...kits.kits[0], candidate_id: "missing" }] },
      roadmap,
      manifest
    ).some((error) => error.includes("is not in qualification roadmap")),
    "qualification evidence kit without roadmap candidate must fail"
  );
  assert(
    qualificationEvidenceKitErrors(
      {
        ...kits,
        kits: [
          {
            ...kits.kits[0],
            status: "collecting",
            artifact_requirements: [
              {
                artifact_id: "reference",
                kind: "note",
                gate: "gate",
                path_policy: "repo-relative-required-before-qualification",
              },
            ],
          },
        ],
      },
      roadmap,
      manifest
    ).some((error) => error.includes("collecting kits must declare artifact_path or artifact_command")),
    "collecting qualification evidence kit without artifact entry must fail"
  );
  assert(
    qualificationEvidenceKitErrors(
      { ...kits, kits: [{ ...kits.kits[0], operator_ids: ["solve.low"] }] },
      roadmap,
      manifest
    ).some((error) => error.includes("is not in the roadmap candidate")),
    "qualification evidence kit with out-of-candidate operator must fail"
  );
  assert(
    qualificationEvidenceKitErrors(
      { ...kits, kits: [{ ...kits.kits[0], operator_ids: [] }] },
      roadmap,
      manifest
    ).some((error) => error.includes("missing roadmap operator_id solve.ok")),
    "qualification evidence kit missing roadmap operator must fail"
  );
  assert(
    qualificationEvidenceKitErrors(
      { ...kits, kits: [{ ...kits.kits[0], operator_ids: ["solve.ok", "solve.ok"] }] },
      roadmap,
      manifest
    ).some((error) => error.includes("duplicate operator_id solve.ok")),
    "qualification evidence kit duplicate operator must fail"
  );
  assert(
    qualificationEvidenceKitErrors(
      {
        ...kits,
        kits: [
          {
            ...kits.kits[0],
            artifact_requirements: [
              kits.kits[0].artifact_requirements[0],
              kits.kits[0].artifact_requirements[0],
            ],
          },
        ],
      },
      roadmap,
      manifest
    ).some((error) => error.includes("duplicate artifact_id reference")),
    "qualification evidence kit duplicate artifact must fail"
  );
  assert(
    qualificationEvidenceKitErrors(
      {
        ...kits,
        kits: [
          {
            ...kits.kits[0],
            artifact_requirements: [
              {
                ...kits.kits[0].artifact_requirements[0],
                artifact_path: "/tmp/reference.md",
              },
            ],
          },
        ],
      },
      roadmap,
      manifest
    ).some((error) => error.includes("artifact_path must be repo-relative")),
    "qualification evidence kit absolute artifact path must fail"
  );
  assert(
    qualificationEvidenceKitErrors({ ...kits, kits: [] }, roadmap, manifest).includes(
      "kits must be non-empty"
    ),
    "empty qualification evidence kits must fail"
  );

  const promotedOperator = {
    operator_id: "solve.ok",
    coverage_level: "qualification",
    evidence: {
      qualification: {
        provenance: [
          "releases/qualification-evidence/2.0.0/self-test.json",
          "releases/qualification-review-decisions/2.0.0/self-test.json",
        ],
        release_gates: ["releases/qualification-records/1.20.0.json"],
      },
    },
  };
  const releaseRecords = {
    records: [{
      candidate_id: "self-test",
      evidence_path: "releases/qualification-evidence/2.0.0/self-test.json",
      review_status: "approved",
      review_decision_path: "releases/qualification-review-decisions/2.0.0/self-test.json",
    }],
  };
  const releaseKit = {
    ...kits,
    kits: [{
      ...kits.kits[0],
      artifact_requirements: [{
        artifact_id: "release-output",
        kind: "release_retained_regression_output",
        gate: "gate",
        artifact_path: "releases/qualification-evidence/2.0.0/self-test.json",
        path_policy: "repo-relative-required-before-qualification",
      }],
    }],
  };
  assert(
    qualificationPromotionErrors([promotedOperator], roadmap, releaseKit, releaseRecords).length === 0,
    "approved qualification promotion must pass"
  );
  assert(
    qualificationPromotionErrors(
      [promotedOperator],
      roadmap,
      releaseKit,
      { records: [{ ...releaseRecords.records[0], review_status: "pending_signoff" }] },
    ).some((error) => error.includes("must be approved")),
    "qualification promotion without approved release record must fail"
  );
  assert(
    qualificationPromotionErrors(
      [{ ...promotedOperator, evidence: { qualification: { provenance: [], release_gates: [] } } }],
      roadmap,
      releaseKit,
      releaseRecords,
    ).some((error) => error.includes("release evidence")),
    "qualification promotion without retained provenance must fail"
  );
}

run();

if (failures.length > 0) {
  console.error(`operator reliability rules self-test failed: ${failures.join("; ")}`);
  process.exit(1);
}

console.log("operator reliability rules self-test passed");
