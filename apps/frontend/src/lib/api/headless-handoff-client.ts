"use client";

import { requestJson } from "./core";
import type { HeadlessOrchestraHandoffEnvelope } from "@/lib/scripting/workbench-headless-orchestra-handoff";

export type HeadlessHandoffReceipt = {
  accepted: boolean;
  handoff_id: string;
  workflow_id: string;
  received_at: string;
  authority_mode: string;
  step_count: number;
  chosen_agent_count: number;
  warning_count: number;
  target_clusters: string[];
  has_dispatch_override: boolean;
  dispatch_override_count: number;
  override_acknowledged: boolean;
  override_note: string | null;
  override_step_keys: string[];
  override_summary: string | null;
  stage: string;
  status_message: string;
};

export type HeadlessHandoffHistoryPayload = {
  handoffs: HeadlessHandoffReceipt[];
};

export type HeadlessHandoffSnapshot = HeadlessHandoffReceipt & {
  envelope: HeadlessOrchestraHandoffEnvelope;
};

export function submitHeadlessOrchestraHandoff(input: HeadlessOrchestraHandoffEnvelope) {
  return requestJson<HeadlessHandoffReceipt>("/api/v1/headless/handoff", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function fetchHeadlessOrchestraHandoffStatus(handoffId: string) {
  return requestJson<HeadlessHandoffReceipt>(`/api/v1/headless/handoff/${encodeURIComponent(handoffId)}`);
}

export function fetchHeadlessOrchestraHandoffHistory() {
  return requestJson<HeadlessHandoffHistoryPayload>("/api/v1/headless/handoff");
}

export function fetchHeadlessOrchestraHandoffSnapshot(handoffId: string) {
  return requestJson<HeadlessHandoffSnapshot>(`/api/v1/headless/handoff/${encodeURIComponent(handoffId)}/snapshot`);
}
