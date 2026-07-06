#!/usr/bin/env node
import {
  isBelowMinimumCoverageLevel,
  levelRank,
  qualificationEvidenceErrors,
  qualificationEvidenceKitErrors,
  qualificationRoadmapErrors,
} from "./operator-reliability-rules.mjs";

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

  const manifest = { version_line: "tamamono self-test" };
  const operators = new Set(["solve.ok", "solve.low"]);
  const levels = new Map([
    ["solve.ok", "review"],
    ["solve.low", "baseline"],
  ]);
  const roadmap = {
    schema_version: "kyuubiki.operator-qualification-roadmap/v1",
    version_line: "tamamono self-test",
    minimum_candidate_level: "review",
    candidates: [
      {
        candidate_id: "self-test",
        priority: "p0",
        domain: "self_test",
        operator_ids: ["solve.ok"],
        rationale: "self-test candidate",
        evidence_gaps: ["gap"],
        required_artifacts: ["artifact"],
        graduation_gate: "gate",
      },
    ],
  };
  assert(
    qualificationRoadmapErrors(roadmap, manifest, operators, levels).length === 0,
    "valid qualification roadmap must pass"
  );
  assert(
    qualificationRoadmapErrors(
      { ...roadmap, version_line: "tamamono wrong" },
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

  const kits = {
    schema_version: "kyuubiki.operator-qualification-evidence-kits/v1",
    version_line: "tamamono self-test",
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
    qualificationEvidenceKitErrors({ ...kits, kits: [] }, roadmap, manifest).includes(
      "kits must be non-empty"
    ),
    "empty qualification evidence kits must fail"
  );
}

run();

if (failures.length > 0) {
  console.error(`operator reliability rules self-test failed: ${failures.join("; ")}`);
  process.exit(1);
}

console.log("operator reliability rules self-test passed");
