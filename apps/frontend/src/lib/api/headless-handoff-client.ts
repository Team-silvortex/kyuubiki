"use client";

import { requestJson } from "./core.ts";
import type { HeadlessOrchestraHandoffEnvelope } from "@/lib/scripting/workbench-headless-orchestra-handoff";

type HeadlessHandoffRequestJson = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;

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
  return defaultHeadlessHandoffApiClient.submitHeadlessOrchestraHandoff(input);
}

export function fetchHeadlessOrchestraHandoffStatus(handoffId: string) {
  return defaultHeadlessHandoffApiClient.fetchHeadlessOrchestraHandoffStatus(handoffId);
}

export function fetchHeadlessOrchestraHandoffHistory() {
  return defaultHeadlessHandoffApiClient.fetchHeadlessOrchestraHandoffHistory();
}

export function fetchHeadlessOrchestraHandoffSnapshot(handoffId: string) {
  return defaultHeadlessHandoffApiClient.fetchHeadlessOrchestraHandoffSnapshot(handoffId);
}

export function createHeadlessHandoffApiClient(request: HeadlessHandoffRequestJson) {
  return {
    submitHeadlessOrchestraHandoff(input: HeadlessOrchestraHandoffEnvelope) {
      return request<HeadlessHandoffReceipt>("/api/v1/headless/handoff", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    fetchHeadlessOrchestraHandoffStatus(handoffId: string) {
      return request<HeadlessHandoffReceipt>(`/api/v1/headless/handoff/${encodeURIComponent(handoffId)}`);
    },
    fetchHeadlessOrchestraHandoffHistory() {
      return request<HeadlessHandoffHistoryPayload>("/api/v1/headless/handoff");
    },
    fetchHeadlessOrchestraHandoffSnapshot(handoffId: string) {
      return request<HeadlessHandoffSnapshot>(
        `/api/v1/headless/handoff/${encodeURIComponent(handoffId)}/snapshot`,
      );
    },
  };
}

export type HeadlessHandoffApiClient = ReturnType<typeof createHeadlessHandoffApiClient>;

export const defaultHeadlessHandoffApiClient = createHeadlessHandoffApiClient(requestJson);
