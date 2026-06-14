"use client";

import type { ProtocolAgentDescriptor } from "@/lib/api";
import type { HeadlessWorkflowExecutionBatch } from "@/components/workbench/workbench-headless-workflow-export";

type DispatchLane = "orchestrator_service" | "direct_mesh_solver" | "frontend_bridge";

type DispatchCapability = "service_api" | "solver_rpc" | "stable_builtin_ui";

type HeadlessAgentDispatchCandidate = {
  agent_id: string;
  endpoint: string;
  cluster_id: string | null;
  runtime_mode: string;
  score: number;
  score_breakdown: {
    health: number;
    headless_bonus: number;
    runtime_match_bonus: number;
    total: number;
  };
};

export type HeadlessAgentDispatchPlan = {
  schema_version: "kyuubiki.headless-agent-dispatch/v1";
  generated_at: string;
  workflow_id: string;
  warnings: string[];
  steps: Array<{
    index: number;
    action: string;
    lane: DispatchLane;
    required_capabilities: DispatchCapability[];
    chosen_agent_id: string | null;
    candidates: HeadlessAgentDispatchCandidate[];
    note: string;
    winner_reason_summary: string;
  }>;
};

function resolveDispatchLane(action: string): DispatchLane {
  if (action === "frontend_macro_bridge") return "frontend_bridge";
  if (action === "direct_mesh_solve" || action === "solve_from_model_version" || action === "solve_and_wait_from_model_version") {
    return "direct_mesh_solver";
  }
  return "orchestrator_service";
}

function resolveDispatchCapabilities(lane: DispatchLane): DispatchCapability[] {
  if (lane === "direct_mesh_solver") return ["solver_rpc"];
  if (lane === "frontend_bridge") return ["stable_builtin_ui"];
  return ["service_api"];
}

function agentSupportsCapability(agent: ProtocolAgentDescriptor, capability: DispatchCapability) {
  const descriptor = agent.descriptor;
  if (!descriptor) return false;
  const tags = descriptor.capabilities.flatMap((entry) => entry.tags);
  const methods = descriptor.protocol.methods;
  if (capability === "solver_rpc") {
    return tags.includes("solver_rpc") || methods.some((method) => method.includes("solve"));
  }
  if (capability === "stable_builtin_ui") {
    return !descriptor.runtime.headless;
  }
  return tags.includes("service_api") || methods.some((method) => /workflow|job|project|model|health/.test(method));
}

function candidateScore(agent: ProtocolAgentDescriptor, lane: DispatchLane) {
  const runtime = agent.descriptor?.runtime;
  const health = runtime?.health_score ?? 0;
  const headlessBonus = runtime?.headless ? 20 : 0;
  const directBonus = lane === "direct_mesh_solver" && runtime?.runtime_mode === "direct_mesh_gui" ? 20 : 0;
  const orchestrationBonus = lane === "orchestrator_service" && runtime?.runtime_mode === "orchestrated_gui" ? 20 : 0;
  return {
    health,
    headless_bonus: headlessBonus,
    runtime_match_bonus: directBonus + orchestrationBonus,
    total: health + headlessBonus + directBonus + orchestrationBonus,
  };
}

function buildStepCandidates(stepAction: string, protocolAgents: ProtocolAgentDescriptor[]) {
  const lane = resolveDispatchLane(stepAction);
  const requiredCapabilities = resolveDispatchCapabilities(lane);
  const candidates = protocolAgents
    .filter((agent) => requiredCapabilities.every((capability) => agentSupportsCapability(agent, capability)))
    .map((agent) => {
      const score = candidateScore(agent, lane);
      return {
        agent_id: agent.id,
        endpoint: `${agent.host}:${agent.port}`,
        cluster_id: agent.descriptor?.runtime.cluster_id ?? null,
        runtime_mode: agent.descriptor?.runtime.runtime_mode ?? "--",
        score: score.total,
        score_breakdown: score,
      };
    })
    .sort((left, right) => right.score_breakdown.total - left.score_breakdown.total);
  return { lane, requiredCapabilities, candidates };
}

function summarizeWinnerReason(
  lane: DispatchLane,
  winner: HeadlessAgentDispatchCandidate | undefined,
) {
  if (!winner) return "No compatible candidate is currently visible.";
  const reasons: string[] = [];
  if (winner.score_breakdown.runtime_match_bonus > 0) {
    reasons.push(
      lane === "direct_mesh_solver" ? "direct-mesh runtime match" : "orchestrated runtime match",
    );
  }
  if (winner.score_breakdown.headless_bonus > 0) {
    reasons.push("headless bonus");
  }
  if (winner.score_breakdown.health > 0) {
    reasons.push(`health ${winner.score_breakdown.health}`);
  }
  return reasons.length > 0 ? reasons.join(" + ") : "baseline compatibility only";
}

export function buildHeadlessAgentDispatchPlan({
  batch,
  protocolAgents,
}: {
  batch: HeadlessWorkflowExecutionBatch;
  protocolAgents: ProtocolAgentDescriptor[];
}): HeadlessAgentDispatchPlan {
  const warnings = [...batch.warnings];
  const steps = batch.steps.map((step) => {
    const dispatch = buildStepCandidates(step.action, protocolAgents);
    if (dispatch.candidates.length === 0) {
      warnings.push(`No protocol agent candidates found for step ${step.index} (${step.action}).`);
    }
    return {
      index: step.index,
      action: step.action,
      lane: dispatch.lane,
      required_capabilities: dispatch.requiredCapabilities,
      chosen_agent_id: dispatch.candidates[0]?.agent_id ?? null,
      candidates: dispatch.candidates,
      winner_reason_summary: summarizeWinnerReason(dispatch.lane, dispatch.candidates[0]),
      note:
        dispatch.lane === "frontend_bridge"
          ? "Frontend bridge steps should remain UI-owned until a stable non-UI bridge target exists."
          : dispatch.candidates[0]
            ? `Prefer ${dispatch.candidates[0].agent_id} based on runtime mode, headless support, and health score.`
            : "No compatible agent candidate is currently visible.",
    };
  });

  return {
    schema_version: "kyuubiki.headless-agent-dispatch/v1",
    generated_at: new Date().toISOString(),
    workflow_id: batch.workflow_id,
    warnings,
    steps,
  };
}
