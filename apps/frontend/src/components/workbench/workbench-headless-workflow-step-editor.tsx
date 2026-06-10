"use client";

import type {
  DraftStep,
  HeadlessActionContract,
  HeadlessInputPort,
  HeadlessReferenceToken,
  PayloadObject,
} from "@/components/workbench/workbench-headless-workflow-contract";
import { updatePayloadField } from "@/components/workbench/workbench-headless-workflow-contract";
import { WorkbenchHeadlessReferenceMapper } from "@/components/workbench/workbench-headless-reference-mapper";

type WorkbenchHeadlessWorkflowStepEditorProps = {
  contract: HeadlessActionContract | undefined;
  endpointsHint: string;
  endpointsLabel: string;
  noReferencesLabel: string;
  parsePayloadText: (payloadText: string) => PayloadObject | null;
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

function isJsonField(port: HeadlessInputPort) {
  return port.key === "payload" || port.key === "input_artifacts";
}

function isNumericField(port: HeadlessInputPort) {
  return port.key === "timeout_ms" || port.key === "interval_ms";
}

function isListField(port: HeadlessInputPort) {
  return port.key === "endpoints";
}

export function WorkbenchHeadlessWorkflowStepEditor({
  contract,
  endpointsHint,
  endpointsLabel,
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

  const renderReferenceMapper = (port: HeadlessInputPort) =>
    port.bindable ? (
      <WorkbenchHeadlessReferenceMapper
        applyLabel={referenceApplyLabel}
        clearLabel={referenceClearLabel}
        emptyLabel={noReferencesLabel}
        field={port.key}
        helperText={referenceCurrentLabel}
        onApply={(template) => patchStepPayload(step.id, (current) => updatePayloadField(current, port.key, template))}
        onClear={() => patchStepPayload(step.id, (current) => updatePayloadField(current, port.key, ""))}
        references={references}
        title={referenceTitle}
        value={readString(payload, port.key)}
      />
    ) : null;

  const renderField = (port: HeadlessInputPort) => {
    const label = isListField(port) ? endpointsLabel : port.label;

    if (isJsonField(port)) {
      return (
        <label className="field-label" key={port.key}>
          <span>{label}</span>
          <textarea
            className="script-panel__editor"
            onChange={(event) => {
              const next = parsePayloadText(event.target.value);
              if (!next) return;
              patchStepPayload(step.id, (current) => updatePayloadField(current, port.key, next));
            }}
            rows={port.key === "payload" ? 6 : 5}
            spellCheck={false}
            value={readJsonBlock(payload, port.key)}
          />
        </label>
      );
    }

    if (isListField(port)) {
      return (
        <div key={port.key}>
          <label className="field-label">
            <span>{label}</span>
            <textarea
              className="script-panel__editor"
              onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, port.key, toStringList(event.target.value)))}
              rows={4}
              spellCheck={false}
              value={readStringList(payload, port.key)}
            />
          </label>
          <p className="card-copy">{endpointsHint}</p>
        </div>
      );
    }

    if (isNumericField(port)) {
      return (
        <label className="field-label" key={port.key}>
          <span>{label}</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, port.key, toOptionalNumber(event.target.value)))}
            type="text"
            value={readNumber(payload, port.key)}
          />
        </label>
      );
    }

    return (
      <label className="field-label" key={port.key}>
        <span>{label}</span>
        <input
          className="text-input"
          onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, port.key, event.target.value))}
          type="text"
          value={readString(payload, port.key)}
        />
      </label>
    );
  };

  if (!contract || contract.inputSchema.length === 0) return null;

  return (
    <>
      {contract.inputSchema.map((port) => (
        <div key={`${step.id}-${port.key}`}>
          {renderField(port)}
          {renderReferenceMapper(port)}
        </div>
      ))}
    </>
  );
}
