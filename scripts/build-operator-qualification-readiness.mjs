#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { operatorReliabilityPaths } from "./operator-reliability-contracts.mjs";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultOut = "tmp/operator-qualification-readiness.json";

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

function artifactState(requirement) {
  if (requirement.artifact_path) {
    return {
      artifact_id: requirement.artifact_id,
      kind: requirement.kind,
      state: fs.existsSync(path.join(repoRoot, requirement.artifact_path)) ? "present" : "missing",
      path: requirement.artifact_path,
      gate: requirement.gate,
    };
  }
  if (requirement.artifact_command) {
    return {
      artifact_id: requirement.artifact_id,
      kind: requirement.kind,
      state: "command_available",
      command: requirement.artifact_command,
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

function readinessFor(candidate, kit) {
  const artifacts = (kit?.artifact_requirements ?? []).map(artifactState);
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
    readiness = "collecting_with_entries";
  } else if (actionable > 0) {
    readiness = "partially_collecting";
  }
  return {
    candidate_id: candidate.candidate_id,
    priority: candidate.priority,
    domain: candidate.domain,
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
    evidence_gaps: candidate.evidence_gaps,
    graduation_gate: candidate.graduation_gate,
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
    ["blocked", 4],
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

function buildNextActions(candidates) {
  return candidates
    .map((candidate) => {
      const artifact = firstActionableArtifact(candidate);
      return {
        candidate_id: candidate.candidate_id,
        priority: candidate.priority,
        readiness: candidate.readiness,
        action_kind: actionKindForArtifact(artifact),
        artifact_id: artifact?.artifact_id ?? null,
        artifact_state: artifact?.state ?? null,
        artifact_kind: artifact?.kind ?? null,
        command: artifact?.command ?? null,
        path: artifact?.path ?? null,
        gate: artifact?.gate ?? candidate.graduation_gate,
      };
    })
    .filter((action) => action.artifact_id !== null || action.readiness !== "collecting_with_entries")
    .sort((left, right) =>
      priorityRank(left.priority) - priorityRank(right.priority)
      || readinessRank(left.readiness) - readinessRank(right.readiness)
      || left.candidate_id.localeCompare(right.candidate_id)
    );
}

function buildReport() {
  const roadmap = readJson(operatorReliabilityPaths.roadmap);
  const kits = readJson(operatorReliabilityPaths.evidenceKits);
  if (roadmap.version_line !== kits.version_line) {
    fail("roadmap and evidence kits version_line must match");
  }
  const kitByCandidate = new Map(kits.kits.map((kit) => [kit.candidate_id, kit]));
  const candidates = roadmap.candidates.map((candidate) =>
    readinessFor(candidate, kitByCandidate.get(candidate.candidate_id))
  );
  const nextActions = buildNextActions(candidates);
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
      next_action_count: nextActions.length,
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
