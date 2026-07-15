#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { operatorReliabilityPaths } from "./operator-reliability-contracts.mjs";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultOut = "tmp/operator-qualification-readiness.json";
const validationProfilesPath = "config/operator-validation-profiles.json";
const releaseReviewStatuses = ["missing", "pending_signoff", "approved", "blocked_scope", "rejected"];
const operatorTrustLevels = ["smoke", "baseline", "review", "qualification"];

function fail(message) {
  console.error(`operator qualification readiness build failed: ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function parseArgs(argv) {
  const args = { out: defaultOut };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--out") {
      args.out = argv[index + 1];
      index += 1;
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  if (!args.out) {
    fail("--out requires a repo-local path");
  }
  return args;
}

function repoLocalOutput(outPath) {
  const absoluteOut = path.resolve(repoRoot, outPath);
  const relativeOut = path.relative(repoRoot, absoluteOut);
  if (relativeOut.startsWith("..") || path.isAbsolute(relativeOut)) {
    fail("--out must stay inside the repository");
  }
  return { absoluteOut, relativeOut };
}

function releaseRecordsByCandidate() {
  const recordsPath = operatorReliabilityPaths.releaseRecords;
  const absolute = path.join(repoRoot, recordsPath);
  if (!fs.existsSync(absolute)) return new Map();
  const records = readJson(recordsPath);
  return new Map((records.records ?? []).map((record) => [record.candidate_id, record]));
}

function validationProfilesByCandidate() {
  const source = readJson(validationProfilesPath);
  const grouped = new Map();
  for (const profile of source.profiles ?? []) {
    const candidateId = profile.qualification_candidate_id ?? profile.profile_id;
    const entry = {
      profile_id: profile.profile_id,
      profile_role: profile.profile_role ?? "component_profile",
      trust_goal: profile.trust_goal,
      operator_count: profile.operators?.length ?? 0,
      command_count: profile.commands?.length ?? 0,
    };
    grouped.set(candidateId, [...(grouped.get(candidateId) ?? []), entry]);
  }
  for (const profiles of grouped.values()) {
    profiles.sort((left, right) =>
      left.profile_role.localeCompare(right.profile_role)
      || left.profile_id.localeCompare(right.profile_id)
    );
  }
  return grouped;
}

function operatorTrustLevelCounts() {
  const manifest = readJson(operatorReliabilityPaths.manifest);
  const operators = (manifest.shards ?? []).flatMap((shardPath) => readJson(shardPath).operators ?? []);
  return Object.fromEntries(operatorTrustLevels.map((level) => [
    level,
    operators.filter((operator) => operator.coverage_level === level).length,
  ]));
}

function artifactState(requirement, releaseRecord) {
  if (requirement.artifact_command) {
    const releaseState = releaseRecord?.capture_command === requirement.artifact_command ? releaseRecord.status : null;
    return {
      artifact_id: requirement.artifact_id,
      kind: requirement.kind,
      state: releaseState ? "present" : "command_available",
      path: requirement.artifact_path ?? null,
      command: requirement.artifact_command,
      check_command: requirement.artifact_check_command ?? null,
      release_record_state: releaseState ?? "missing",
      release_record_path: releaseRecord?.evidence_path ?? "",
      release_review_status: releaseRecord?.review_status ?? "missing",
      release_review_gate: releaseRecord?.review_gate ?? "",
      release_review_decision_path: releaseRecord?.review_decision_path ?? "",
      gate: requirement.gate,
    };
  }
  if (requirement.artifact_path) {
    return {
      artifact_id: requirement.artifact_id,
      kind: requirement.kind,
      state: fs.existsSync(path.join(repoRoot, requirement.artifact_path)) ? "present" : "missing",
      path: requirement.artifact_path,
      gate: requirement.gate,
    };
  }
  return {
    artifact_id: requirement.artifact_id,
    kind: requirement.kind,
    state: "not_started",
    gate: requirement.gate,
  };
}

function readinessFor(candidate, kit, releaseRecord, validationProfiles) {
  const artifacts = (kit?.artifact_requirements ?? []).map((requirement) => artifactState(requirement, releaseRecord));
  const present = artifacts.filter((artifact) => artifact.state === "present").length;
  const commands = artifacts.filter((artifact) => artifact.state === "command_available").length;
  const missing = artifacts.filter((artifact) => artifact.state === "missing").length;
  const notStarted = artifacts.filter((artifact) => artifact.state === "not_started").length;
  const actionable = artifacts.filter((artifact) => artifact.state !== "not_started").length;
  let readiness = "planned";
  if (kit?.status === "blocked") {
    readiness = "blocked";
  } else if (missing > 0) {
    readiness = "broken";
  } else if (notStarted === 0 && artifacts.length > 0) {
    readiness = "ready_for_review";
  } else if (actionable > 0) {
    readiness = "partially_collecting";
  }
  return {
    candidate_id: candidate.candidate_id,
    priority: candidate.priority,
    domain: candidate.domain,
    target_level: candidate.target_level,
    evidence_phase: candidate.evidence_phase,
    status: kit?.status ?? "missing_kit",
    readiness,
    operator_ids: candidate.operator_ids,
    artifact_counts: {
      total: artifacts.length,
      present,
      command_available: commands,
      missing,
      not_started: notStarted,
    },
    artifacts,
    validation_profiles: validationProfiles,
    primary_blocker: candidate.primary_blocker,
    evidence_gaps: candidate.evidence_gaps,
    graduation_gate: candidate.graduation_gate,
    preferred_validation_lane: candidate.preferred_validation_lane,
    release_gate_impact: candidate.release_gate_impact,
  };
}

function priorityRank(priority) {
  const ranks = new Map([
    ["p0", 0],
    ["p1", 1],
    ["p2", 2],
    ["p3", 3],
  ]);
  return ranks.get(priority) ?? 99;
}

function readinessRank(readiness) {
  const ranks = new Map([
    ["broken", 0],
    ["planned", 1],
    ["partially_collecting", 2],
    ["collecting_with_entries", 3],
    ["ready_for_review", 4],
    ["blocked", 5],
  ]);
  return ranks.get(readiness) ?? 99;
}

function firstActionableArtifact(candidate) {
  return candidate.artifacts.find((artifact) => artifact.state !== "present") ?? null;
}

function actionKindForArtifact(artifact) {
  if (!artifact) return "review";
  if (artifact.state === "command_available") return "run_command";
  if (artifact.state === "missing") return "restore_or_generate_artifact";
  return "collect_artifact";
}

function hasApprovedReleaseReview(candidate) {
  return (candidate.artifacts ?? []).some((artifact) =>
    artifact.kind === "release_retained_regression_output"
    && artifact.release_review_status === "approved"
  );
}

function buildNextActions(candidates) {
  return candidates
    .flatMap((candidate) => {
      const artifact = firstActionableArtifact(candidate);
      if (!artifact && hasApprovedReleaseReview(candidate)) {
        return [];
      }
      const validationProfiles = candidate.validation_profiles ?? [];
      const releaseProfiles = validationProfiles.filter((profile) => profile.profile_role === "release_candidate");
      return [{
        candidate_id: candidate.candidate_id,
        priority: candidate.priority,
        target_level: candidate.target_level,
        evidence_phase: candidate.evidence_phase,
        readiness: candidate.readiness,
        action_kind: actionKindForArtifact(artifact),
        artifact_id: artifact?.artifact_id ?? null,
        artifact_state: artifact?.state ?? null,
        artifact_kind: artifact?.kind ?? null,
        command: artifact?.command ?? null,
        check_command: artifact?.check_command ?? null,
        path: artifact?.path ?? null,
        gate: artifact?.gate ?? candidate.graduation_gate,
        review_reason: artifact ? null : candidate.primary_blocker,
        validation_profile_count: validationProfiles.length,
        release_candidate_profile_count: releaseProfiles.length,
        preferred_validation_lane: candidate.preferred_validation_lane,
        release_gate_impact: candidate.release_gate_impact,
      }];
    })
    .filter((action) => action.artifact_id !== null || action.readiness !== "collecting_with_entries")
    .sort((left, right) =>
      priorityRank(left.priority) - priorityRank(right.priority)
      || readinessRank(left.readiness) - readinessRank(right.readiness)
      || left.candidate_id.localeCompare(right.candidate_id)
    );
}

function countBy(candidates, field, values) {
  return Object.fromEntries(values.map((value) => [
    value,
    candidates.filter((candidate) => candidate[field] === value).length,
  ]));
}

function countReleaseReviewStatuses(candidates) {
  const releaseArtifacts = candidates.flatMap((candidate) =>
    candidate.artifacts.filter((artifact) => artifact.kind === "release_retained_regression_output")
  );
  return Object.fromEntries(releaseReviewStatuses.map((status) => [
    status,
    releaseArtifacts.filter((artifact) => (artifact.release_review_status ?? "missing") === status).length,
  ]));
}

function countReleaseReviewDecisions(candidates) {
  const releaseArtifacts = candidates.flatMap((candidate) =>
    candidate.artifacts.filter((artifact) => artifact.kind === "release_retained_regression_output")
  );
  const withDecisionPath = releaseArtifacts.filter((artifact) => artifact.release_review_decision_path);
  const retained = withDecisionPath.filter((artifact) =>
    fs.existsSync(path.join(repoRoot, artifact.release_review_decision_path))
  );
  return {
    required: releaseArtifacts.length,
    declared: withDecisionPath.length,
    retained: retained.length,
    missing: releaseArtifacts.length - retained.length,
  };
}

function sameStrings(left, right) {
  const sortedLeft = [...left].sort((a, b) => a.localeCompare(b));
  const sortedRight = [...right].sort((a, b) => a.localeCompare(b));
  return sortedLeft.length === sortedRight.length
    && sortedLeft.every((item, index) => item === sortedRight[index]);
}

function countReleasePromotionSummaries(candidates) {
  const releaseVersion = readJson(operatorReliabilityPaths.releaseRecords).release_version;
  const approvedArtifacts = candidates.flatMap((candidate) =>
    candidate.artifacts
      .filter((artifact) =>
        artifact.kind === "release_retained_regression_output"
        && artifact.release_review_status === "approved"
      )
      .map((artifact) => ({ candidate, artifact }))
  );
  const retained = approvedArtifacts.filter(({ artifact }) =>
    artifact.release_record_path && fs.existsSync(path.join(repoRoot, artifact.release_record_path))
  );
  const withSummary = retained
    .map(({ candidate, artifact }) => ({
      candidate,
      artifact,
      evidence: readJson(artifact.release_record_path),
    }))
    .filter(({ evidence }) => evidence.promotion_summary);
  const matched = withSummary.filter(({ candidate, artifact, evidence }) => {
    const summary = evidence.promotion_summary;
    return summary.candidate_id === candidate.candidate_id
      && summary.release_version === releaseVersion
      && summary.approved_coverage_level === "qualification"
      && summary.retained_evidence_path === artifact.release_record_path
      && summary.release_record_path === operatorReliabilityPaths.releaseRecords
      && summary.review_decision_path === artifact.release_review_decision_path
      && sameStrings(summary.promoted_operator_ids ?? [], candidate.operator_ids ?? []);
  });
  return {
    required: approvedArtifacts.length,
    retained: retained.length,
    declared: withSummary.length,
    matched: matched.length,
    missing: approvedArtifacts.length - matched.length,
  };
}

function buildReport() {
  const roadmap = readJson(operatorReliabilityPaths.roadmap);
  const kits = readJson(operatorReliabilityPaths.evidenceKits);
  if (roadmap.version_line !== kits.version_line) {
    fail("roadmap and evidence kits version_line must match");
  }
  const kitByCandidate = new Map(kits.kits.map((kit) => [kit.candidate_id, kit]));
  const releaseRecords = releaseRecordsByCandidate();
  const validationProfiles = validationProfilesByCandidate();
  const candidates = roadmap.candidates.map((candidate) =>
    readinessFor(
      candidate,
      kitByCandidate.get(candidate.candidate_id),
      releaseRecords.get(candidate.candidate_id),
      validationProfiles.get(candidate.candidate_id) ?? []
    )
  );
  const nextActions = buildNextActions(candidates);
  const validationProfileCount = candidates.reduce((count, candidate) => count + candidate.validation_profiles.length, 0);
  const releaseCandidateProfiles = candidates.reduce((count, candidate) =>
    count + candidate.validation_profiles.filter((profile) => profile.profile_role === "release_candidate").length, 0);
  return {
    schema_version: "kyuubiki.operator-qualification-readiness/v1",
    version_line: roadmap.version_line,
    generated_at_utc: new Date().toISOString(),
    summary: {
      candidates: candidates.length,
      collecting: candidates.filter((candidate) => candidate.status === "collecting").length,
      planned: candidates.filter((candidate) => candidate.status === "planned").length,
      with_entries: candidates.filter((candidate) => candidate.artifact_counts.present > 0 || candidate.artifact_counts.command_available > 0).length,
      not_started: candidates.filter((candidate) => candidate.artifact_counts.not_started === candidate.artifact_counts.total).length,
      broken: candidates.filter((candidate) => candidate.readiness === "broken").length,
      validation_profile_count: validationProfileCount,
      release_candidate_profiles: releaseCandidateProfiles,
      component_profiles: validationProfileCount - releaseCandidateProfiles,
      candidates_missing_release_profile: candidates.filter((candidate) =>
        !candidate.validation_profiles.some((profile) => profile.profile_role === "release_candidate")
      ).length,
      next_action_count: nextActions.length,
      target_levels: countBy(candidates, "target_level", ["baseline", "review", "qualification"]),
      operator_trust_levels: operatorTrustLevelCounts(),
      evidence_phases: countBy(candidates, "evidence_phase", ["planned", "collecting", "ready_for_review", "blocked"]),
      release_gate_impacts: countBy(candidates, "release_gate_impact", [
        "release_blocker",
        "release_watch",
        "experimental_only",
      ]),
      release_review_statuses: countReleaseReviewStatuses(candidates),
      release_review_decisions: countReleaseReviewDecisions(candidates),
      release_promotion_summaries: countReleasePromotionSummaries(candidates),
    },
    next_actions: nextActions,
    candidates,
  };
}

function writeReport(outPath, report) {
  const { absoluteOut, relativeOut } = repoLocalOutput(outPath);
  fs.mkdirSync(path.dirname(absoluteOut), { recursive: true });
  fs.writeFileSync(absoluteOut, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`operator qualification readiness wrote ${relativeOut}`);
}

const args = parseArgs(process.argv);
writeReport(args.out, buildReport());
