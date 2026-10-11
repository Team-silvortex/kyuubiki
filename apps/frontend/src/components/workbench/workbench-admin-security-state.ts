"use client";

import { useEffect, useRef, useState } from "react";
import type { JobState, ResultRecord } from "@/lib/api";
import type { SecurityEventRecord } from "@/lib/api/security-results-types";
import type {
  SecurityEventWindow,
} from "@/components/workbench/workbench-types";
import type {
  WorkbenchSecurityAuditRisk,
  WorkbenchSecurityAuditSource,
} from "@/lib/workbench/security-audit";

type UseWorkbenchAdminSecurityStateArgs = {
  jobHistory: JobState[];
  resultRecords: ResultRecord[];
  selectedAdminJobId: string | null;
};

type AdminDraftBaseline = { jobId: string | null; contents: string };

export function reconcileWorkbenchAdminDraft(
  draft: string,
  previous: AdminDraftBaseline,
  incoming: AdminDraftBaseline,
): string {
  return incoming.jobId !== null && previous.jobId === incoming.jobId && draft !== previous.contents
    ? draft : incoming.contents;
}

export function useWorkbenchAdminSecurityState({
  jobHistory,
  resultRecords,
  selectedAdminJobId,
}: UseWorkbenchAdminSecurityStateArgs) {
  const [selectedAdminResultJobId, setSelectedAdminResultJobId] = useState<string | null>(null);
  const [adminFilterProjectId, setAdminFilterProjectId] = useState("");
  const [adminFilterModelVersionId, setAdminFilterModelVersionId] = useState("");
  const [securityEventRecords, setSecurityEventRecords] = useState<SecurityEventRecord[]>([]);
  const [scriptRecordingMode, setScriptRecordingMode] = useState(false);
  const [securityEventWindowFilter, setSecurityEventWindowFilter] = useState<SecurityEventWindow>("24h");
  const [securityEventSourceFilter, setSecurityEventSourceFilter] = useState<
    WorkbenchSecurityAuditSource | "hub-assistant" | ""
  >("");
  const [securityEventRiskFilter, setSecurityEventRiskFilter] = useState<
    WorkbenchSecurityAuditRisk | ""
  >("");
  const [securityEventStatusFilter, setSecurityEventStatusFilter] = useState<
    "" | "allowed" | "blocked"
  >("");
  const [securityEventActionFilter, setSecurityEventActionFilter] = useState("");
  const [adminJobMessage, setAdminJobMessage] = useState("");
  const [adminJobProjectId, setAdminJobProjectId] = useState("");
  const [adminJobModelVersionId, setAdminJobModelVersionId] = useState("");
  const [adminJobCaseId, setAdminJobCaseId] = useState("");
  const [adminResultDraft, setAdminResultDraft] = useState("{}");
  const jobDraftBaselineRef = useRef({ jobId: null as string | null,
    message: "", project: "", version: "", case: "" });
  const resultDraftBaselineRef = useRef({ jobId: null as string | null, contents: "{}" });

  useEffect(() => {
    const current = jobHistory.find((entry) => entry.job_id === selectedAdminJobId) ?? null;
    const previous = jobDraftBaselineRef.current;
    const incoming = { jobId: current?.job_id ?? null, message: current?.message ?? "",
      project: current?.project_id ?? "", version: current?.model_version_id ?? "", case: current?.simulation_case_id ?? "" };
    // Each clean field follows reads independently; unsaved fields keep their working value.
    const reconcile = (draft: string, field: "message" | "project" | "version" | "case") =>
      reconcileWorkbenchAdminDraft(draft, { jobId: previous.jobId, contents: previous[field] },
        { jobId: incoming.jobId, contents: incoming[field] });
    setAdminJobMessage((draft) => reconcile(draft, "message"));
    setAdminJobProjectId((draft) => reconcile(draft, "project"));
    setAdminJobModelVersionId((draft) => reconcile(draft, "version"));
    setAdminJobCaseId((draft) => reconcile(draft, "case"));
    jobDraftBaselineRef.current = incoming;
  }, [jobHistory, selectedAdminJobId]);

  useEffect(() => {
    const current = resultRecords.find((entry) => entry.job_id === selectedAdminResultJobId) ?? null;
    const previous = resultDraftBaselineRef.current;
    const contents = JSON.stringify(current?.result ?? {}, null, 2);
    // Reads may update the baseline, but only clean drafts follow it automatically.
    const incoming = { jobId: current?.job_id ?? null, contents };
    setAdminResultDraft((draft) => reconcileWorkbenchAdminDraft(draft, previous, incoming));
    resultDraftBaselineRef.current = incoming;
  }, [resultRecords, selectedAdminResultJobId]);

  return {
    selectedAdminResultJobId,
    setSelectedAdminResultJobId,
    adminFilterProjectId,
    setAdminFilterProjectId,
    adminFilterModelVersionId,
    setAdminFilterModelVersionId,
    securityEventRecords,
    setSecurityEventRecords,
    scriptRecordingMode,
    setScriptRecordingMode,
    securityEventWindowFilter,
    setSecurityEventWindowFilter,
    securityEventSourceFilter,
    setSecurityEventSourceFilter,
    securityEventRiskFilter,
    setSecurityEventRiskFilter,
    securityEventStatusFilter,
    setSecurityEventStatusFilter,
    securityEventActionFilter,
    setSecurityEventActionFilter,
    adminJobMessage,
    setAdminJobMessage,
    adminJobProjectId,
    setAdminJobProjectId,
    adminJobModelVersionId,
    setAdminJobModelVersionId,
    adminJobCaseId,
    setAdminJobCaseId,
    adminResultDraft,
    setAdminResultDraft,
  };
}
