"use client";

import type { DraftStep, HeadlessReferenceToken, PayloadObject } from "@/components/workbench/workbench-headless-workflow-contract";
import { updatePayloadField } from "@/components/workbench/workbench-headless-workflow-contract";
import { WorkbenchHeadlessReferenceMapper } from "@/components/workbench/workbench-headless-reference-mapper";
import type { WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

type WorkbenchHeadlessWorkflowStepEditorProps = {
  endpointsHint: string;
  endpointsLabel: string;
  language: WorkbenchScriptLanguage;
  noReferencesLabel: string;
  parsePayloadText: (payloadText: string) => PayloadObject | null;
  payloadJsonLabel: string;
  patchStepPayload: (stepId: string, updater: (payload: PayloadObject | null) => PayloadObject | null) => void;
  referenceApplyLabel: string;
  referenceClearLabel: string;
  referenceCurrentLabel: string;
  references: HeadlessReferenceToken[];
  referenceTitle: string;
  step: DraftStep;
};

function readString(payload: PayloadObject | null, key: string) {
  const value = payload?.[key];
  return typeof value === "string" ? value : "";
}

function readNumber(payload: PayloadObject | null, key: string) {
  const value = payload?.[key];
  return typeof value === "number" ? String(value) : "";
}

function readStringList(payload: PayloadObject | null, key: string) {
  const value = payload?.[key];
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string").join("\n") : "";
}

function readJsonBlock(payload: PayloadObject | null, key: string) {
  const value = payload?.[key];
  return JSON.stringify(value && typeof value === "object" && !Array.isArray(value) ? value : {}, null, 2);
}

function toStringList(value: string) {
  return value
    .split("\n")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function toOptionalNumber(value: string) {
  if (!value.trim()) return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

export function WorkbenchHeadlessWorkflowStepEditor({
  endpointsHint,
  endpointsLabel,
  language,
  noReferencesLabel,
  parsePayloadText,
  patchStepPayload,
  referenceApplyLabel,
  referenceClearLabel,
  referenceCurrentLabel,
  references,
  referenceTitle,
  step,
}: WorkbenchHeadlessWorkflowStepEditorProps) {
  const payload = parsePayloadText(step.payloadText);

  const renderReferenceMapper = (field: string) => (
    <WorkbenchHeadlessReferenceMapper
      applyLabel={referenceApplyLabel}
      clearLabel={referenceClearLabel}
      emptyLabel={noReferencesLabel}
      field={field}
      helperText={referenceCurrentLabel}
      onApply={(template) => patchStepPayload(step.id, (current) => updatePayloadField(current, field, template))}
      onClear={() => patchStepPayload(step.id, (current) => updatePayloadField(current, field, ""))}
      references={references}
      title={referenceTitle}
      value={readString(payload, field)}
    />
  );

  if (step.action === "project_create") {
    return (
      <>
        <label className="field-label">
          <span>name</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "name", event.target.value))}
            type="text"
            value={readString(payload, "name")}
          />
        </label>
        <label className="field-label">
          <span>description</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "description", event.target.value))}
            type="text"
            value={readString(payload, "description")}
          />
        </label>
      </>
    );
  }

  if (step.action === "model_create") {
    return (
      <>
        <label className="field-label">
          <span>project_id</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "project_id", event.target.value))}
            type="text"
            value={readString(payload, "project_id")}
          />
        </label>
        {renderReferenceMapper("project_id")}
        <label className="field-label">
          <span>name</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "name", event.target.value))}
            type="text"
            value={readString(payload, "name")}
          />
        </label>
        <label className="field-label">
          <span>kind</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "kind", event.target.value))}
            type="text"
            value={readString(payload, "kind")}
          />
        </label>
        <label className="field-label">
          <span>payload</span>
          <textarea
            className="script-panel__editor"
            onChange={(event) => {
              const next = parsePayloadText(event.target.value);
              if (!next) return;
              patchStepPayload(step.id, (current) => updatePayloadField(current, "payload", next));
            }}
            rows={6}
            spellCheck={false}
            value={readJsonBlock(payload, "payload")}
          />
        </label>
      </>
    );
  }

  if (step.action === "model_version_create") {
    return (
      <>
        <label className="field-label">
          <span>model_id</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "model_id", event.target.value))}
            type="text"
            value={readString(payload, "model_id")}
          />
        </label>
        {renderReferenceMapper("model_id")}
        <label className="field-label">
          <span>name</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "name", event.target.value))}
            type="text"
            value={readString(payload, "name")}
          />
        </label>
        <label className="field-label">
          <span>payload</span>
          <textarea
            className="script-panel__editor"
            onChange={(event) => {
              const next = parsePayloadText(event.target.value);
              if (!next) return;
              patchStepPayload(step.id, (current) => updatePayloadField(current, "payload", next));
            }}
            rows={6}
            spellCheck={false}
            value={readJsonBlock(payload, "payload")}
          />
        </label>
      </>
    );
  }

  if (step.action === "workflow_submit_catalog") {
    return (
      <>
        <label className="field-label">
          <span>workflow_id</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "workflow_id", event.target.value))}
            type="text"
            value={readString(payload, "workflow_id")}
          />
        </label>
        <label className="field-label">
          <span>input_artifacts</span>
          <textarea
            className="script-panel__editor"
            onChange={(event) => {
              const next = parsePayloadText(event.target.value);
              if (!next) return;
              patchStepPayload(step.id, (current) => updatePayloadField(current, "input_artifacts", next));
            }}
            rows={5}
            spellCheck={false}
            value={readJsonBlock(payload, "input_artifacts")}
          />
        </label>
      </>
    );
  }

  if (step.action === "solve_from_model_version" || step.action === "solve_and_wait_from_model_version") {
    return (
      <>
        <label className="field-label">
          <span>model_version_id</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "model_version_id", event.target.value))}
            type="text"
            value={readString(payload, "model_version_id")}
          />
        </label>
        {renderReferenceMapper("model_version_id")}
        <label className="field-label">
          <span>{endpointsLabel}</span>
          <textarea
            className="script-panel__editor"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "endpoints", toStringList(event.target.value)))}
            rows={4}
            spellCheck={false}
            value={readStringList(payload, "endpoints")}
          />
        </label>
        <p className="card-copy">{endpointsHint}</p>
        {step.action === "solve_and_wait_from_model_version" ? (
          <label className="field-label">
            <span>timeout_ms</span>
            <input
              className="text-input"
              onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "timeout_ms", toOptionalNumber(event.target.value)))}
              type="text"
              value={readNumber(payload, "timeout_ms")}
            />
          </label>
        ) : null}
      </>
    );
  }

  if (step.action === "job_wait") {
    return (
      <>
        <label className="field-label">
          <span>job_id</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "job_id", event.target.value))}
            type="text"
            value={readString(payload, "job_id")}
          />
        </label>
        {renderReferenceMapper("job_id")}
        <label className="field-label">
          <span>interval_ms</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "interval_ms", toOptionalNumber(event.target.value)))}
            type="text"
            value={readNumber(payload, "interval_ms")}
          />
        </label>
        <label className="field-label">
          <span>timeout_ms</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "timeout_ms", toOptionalNumber(event.target.value)))}
            type="text"
            value={readNumber(payload, "timeout_ms")}
          />
        </label>
      </>
    );
  }

  if (step.action === "job_fetch" || step.action === "result_fetch") {
    return (
      <>
        <label className="field-label">
          <span>job_id</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "job_id", event.target.value))}
            type="text"
            value={readString(payload, "job_id")}
          />
        </label>
        {renderReferenceMapper("job_id")}
      </>
    );
  }

  return null;
}
