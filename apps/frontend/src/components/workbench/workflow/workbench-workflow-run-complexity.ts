import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";

export function scoreWorkflowRunComplexity(run: WorkflowRunRecord) {
  if (!run.traceSummary) {
    return (run.branchDecisions?.length ?? 0) * 3 +
      (run.skippedNodes?.length ?? 0) * 2 +
      (run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0);
  }
  return run.traceSummary.branchDecisionCount * 3 +
    run.traceSummary.skippedNodeRunCount * 2 +
    run.traceSummary.derivedArtifactCount +
    Math.min(run.traceSummary.progressEventCount, 6);
}

export function describeWorkflowRunComplexity(run: WorkflowRunRecord) {
  const branches = run.traceSummary?.branchDecisionCount ?? run.branchDecisions?.length ?? 0;
  const derived =
    run.traceSummary?.derivedArtifactCount ??
    run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ??
    0;
  const skipped = run.traceSummary?.skippedNodeRunCount ?? run.skippedNodes?.length ?? 0;
  const progressEvents = run.traceSummary?.progressEventCount ?? 0;
  const score = scoreWorkflowRunComplexity(run);
  const tags: Array<{ label: string; tone: "watch" | "good" | "risk" }> = [];
  if (score >= 8) tags.push({ label: "complex", tone: "risk" });
  if (branches >= 2) tags.push({ label: "branch-heavy", tone: "watch" });
  if (derived >= 3) tags.push({ label: "lineage-heavy", tone: "good" });
  if (tags.length < 2 && progressEvents >= 4) tags.push({ label: "eventful", tone: "watch" });
  if (tags.length === 0 && skipped > 0) tags.push({ label: "skip-path", tone: "watch" });
  return tags.slice(0, 2);
}
