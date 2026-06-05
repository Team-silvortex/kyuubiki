"use client";

import { useEffect, useState } from "react";
import type { JobState, ResultRecord } from "@/lib/api";
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

export function useWorkbenchAdminSecurityState({
  jobHistory,
  resultRecords,
  selectedAdminJobId,
}: UseWorkbenchAdminSecurityStateArgs) {
  const [selectedAdminResultJobId, setSelectedAdminResultJobId] = useState<string | null>(null);
  const [adminFilterProjectId, setAdminFilterProjectId] = useState("");
  const [adminFilterModelVersionId, setAdminFilterModelVersionId] = useState("");
  const [securityEventRecords, setSecurityEventRecords] = useState<any[]>([]);
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

  useEffect(() => {
    const current = jobHistory.find((entry) => entry.job_id === selectedAdminJobId) ?? null;
    setAdminJobMessage(current?.message ?? "");
    setAdminJobProjectId(current?.project_id ?? "");
    setAdminJobModelVersionId(current?.model_version_id ?? "");
    setAdminJobCaseId(current?.simulation_case_id ?? "");
  }, [jobHistory, selectedAdminJobId]);

  useEffect(() => {
    const current = resultRecords.find((entry) => entry.job_id === selectedAdminResultJobId) ?? null;
    setAdminResultDraft(JSON.stringify(current?.result ?? {}, null, 2));
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
