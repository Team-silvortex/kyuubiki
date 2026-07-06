export const orderedLevels = ["smoke", "baseline", "review", "qualification"];
export const allowedLevels = new Set(orderedLevels);
export const allowedKitStatuses = new Set(["planned", "collecting", "ready_for_review", "blocked"]);

export function levelRank(level) {
  return orderedLevels.indexOf(level);
}

export function isBelowMinimumCoverageLevel(level, minimumLevel) {
  return levelRank(level) < levelRank(minimumLevel);
}

export function qualificationEvidenceErrors(entry) {
  const errors = [];
  const qualification = entry.evidence.qualification;
  if (!qualification) {
    return ["qualification-level operators must declare evidence.qualification"];
  }
  for (const field of [
    "validation_sources",
    "convergence_checks",
    "provenance",
    "release_gates",
    "tests",
  ]) {
    if (!Array.isArray(qualification[field]) || qualification[field].length === 0) {
      errors.push(`evidence.qualification.${field} must be non-empty`);
    }
  }
  return errors;
}

export function qualificationRoadmapErrors(roadmap, manifest, seenOperators, operatorLevels) {
  const errors = [];
  if (roadmap.schema_version !== "kyuubiki.operator-qualification-roadmap/v1") {
    errors.push("unexpected schema_version");
  }
  if (roadmap.version_line !== manifest.version_line) {
    errors.push("version_line must match reliability manifest");
  }
  if (!allowedLevels.has(roadmap.minimum_candidate_level)) {
    errors.push(`unknown minimum_candidate_level ${roadmap.minimum_candidate_level}`);
  }
  if (!Array.isArray(roadmap.candidates) || roadmap.candidates.length === 0) {
    errors.push("candidates must be non-empty");
    return errors;
  }

  const seenCandidates = new Set();
  for (const candidate of roadmap.candidates) {
    const context = `qualification roadmap ${candidate.candidate_id ?? "unknown"}`;
    if (!candidate.candidate_id || seenCandidates.has(candidate.candidate_id)) {
      errors.push(`${context}: candidate_id must be unique`);
      continue;
    }
    seenCandidates.add(candidate.candidate_id);
    for (const field of ["priority", "domain", "rationale", "graduation_gate"]) {
      if (typeof candidate[field] !== "string" || candidate[field].length === 0) {
        errors.push(`${context}: ${field} must be non-empty`);
      }
    }
    for (const field of ["operator_ids", "evidence_gaps", "required_artifacts"]) {
      if (!Array.isArray(candidate[field]) || candidate[field].length === 0) {
        errors.push(`${context}: ${field} must be non-empty`);
      }
    }
    for (const operatorId of candidate.operator_ids ?? []) {
      if (!seenOperators.has(operatorId)) {
        errors.push(`${context}: operator_id ${operatorId} is not in reliability manifest`);
        continue;
      }
      if (isBelowMinimumCoverageLevel(operatorLevels.get(operatorId), roadmap.minimum_candidate_level)) {
        errors.push(
          `${context}: operator_id ${operatorId} is below roadmap minimum ` +
            `${roadmap.minimum_candidate_level}`
        );
      }
    }
  }
  return errors;
}

export function qualificationEvidenceKitErrors(kits, roadmap, manifest) {
  const errors = [];
  if (kits.schema_version !== "kyuubiki.operator-qualification-evidence-kits/v1") {
    errors.push("unexpected schema_version");
  }
  if (kits.version_line !== manifest.version_line) {
    errors.push("version_line must match reliability manifest");
  }
  if (!Array.isArray(kits.kits) || kits.kits.length === 0) {
    errors.push("kits must be non-empty");
    return errors;
  }

  const roadmapCandidates = new Map(
    (roadmap.candidates ?? []).map((candidate) => [candidate.candidate_id, candidate])
  );
  const seenKits = new Set();
  for (const kit of kits.kits) {
    const context = `qualification evidence kit ${kit.candidate_id ?? "unknown"}`;
    if (!kit.candidate_id || seenKits.has(kit.candidate_id)) {
      errors.push(`${context}: candidate_id must be unique`);
      continue;
    }
    seenKits.add(kit.candidate_id);
    const roadmapCandidate = roadmapCandidates.get(kit.candidate_id);
    if (!roadmapCandidate) {
      errors.push(`${context}: candidate_id is not in qualification roadmap`);
    }
    if (!allowedKitStatuses.has(kit.status)) {
      errors.push(`${context}: unknown status ${kit.status}`);
    }
    if (typeof kit.artifact_profile !== "string" || kit.artifact_profile.length === 0) {
      errors.push(`${context}: artifact_profile must be non-empty`);
    }
    if (!Array.isArray(kit.operator_ids) || kit.operator_ids.length === 0) {
      errors.push(`${context}: operator_ids must be non-empty`);
    }
    const seenOperatorIds = new Set();
    for (const operatorId of kit.operator_ids ?? []) {
      if (seenOperatorIds.has(operatorId)) {
        errors.push(`${context}: duplicate operator_id ${operatorId}`);
        continue;
      }
      seenOperatorIds.add(operatorId);
      if (roadmapCandidate && !roadmapCandidate.operator_ids.includes(operatorId)) {
        errors.push(`${context}: operator_id ${operatorId} is not in the roadmap candidate`);
      }
    }
    for (const operatorId of roadmapCandidate?.operator_ids ?? []) {
      if (!kit.operator_ids?.includes(operatorId)) {
        errors.push(`${context}: missing roadmap operator_id ${operatorId}`);
      }
    }
    if (!Array.isArray(kit.artifact_requirements) || kit.artifact_requirements.length === 0) {
      errors.push(`${context}: artifact_requirements must be non-empty`);
    }
    const seenArtifacts = new Set();
    for (const requirement of kit.artifact_requirements ?? []) {
      if (seenArtifacts.has(requirement.artifact_id)) {
        errors.push(`${context}: duplicate artifact_id ${requirement.artifact_id}`);
        continue;
      }
      seenArtifacts.add(requirement.artifact_id);
      for (const field of ["artifact_id", "kind", "gate", "path_policy"]) {
        if (typeof requirement[field] !== "string" || requirement[field].length === 0) {
          errors.push(`${context}: artifact_requirements.${field} must be non-empty`);
        }
      }
    }
  }

  for (const candidateId of roadmapCandidates.keys()) {
    if (!seenKits.has(candidateId)) {
      errors.push(`qualification evidence kits: missing kit for roadmap candidate ${candidateId}`);
    }
  }
  return errors;
}
