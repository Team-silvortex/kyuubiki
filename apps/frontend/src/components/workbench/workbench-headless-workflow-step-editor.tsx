"use client";

import { useState } from "react";

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
  bridgeActionListLabel: string;
  bridgeMacroIdLabel: string;
  bridgePreviewHideLabel: string;
  bridgePreviewPayloadLabel: string;
  bridgePreviewShowLabel: string;
  bridgeReplayModeHint: string;
  bridgeReplayModeLabel: string;
  bridgeRestoreLabel: string;
  bridgeStepCountLabel: string;
  contract: HeadlessActionContract | undefined;
  endpointsHint: string;
  endpointsLabel: string;
  noReferencesLabel: string;
  onRestoreBridgeMacro?: () => void;
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

function readStepActions(payload: PayloadObject | null) {
  const value = payload?.steps;
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry) => {
    if (!entry || typeof entry !== "object") return [];
    const action = (entry as { action?: unknown }).action;
    return typeof action === "string" ? [action] : [];
  });
}

function readBridgeSteps(payload: PayloadObject | null) {
  const value = payload?.steps;
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry) => {
    if (!entry || typeof entry !== "object") return [];
    const candidate = entry as { action?: unknown; payload?: unknown };
    if (typeof candidate.action !== "string") return [];
    return [
      {
        action: candidate.action,
        payload:
          candidate.payload && typeof candidate.payload === "object" && !Array.isArray(candidate.payload)
            ? (candidate.payload as PayloadObject)
            : {},
      },
    ];
  });
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
  bridgeActionListLabel,
  bridgeMacroIdLabel,
  bridgePreviewHideLabel,
  bridgePreviewPayloadLabel,
  bridgePreviewShowLabel,
  bridgeReplayModeHint,
  bridgeReplayModeLabel,
  bridgeRestoreLabel,
  bridgeStepCountLabel,
  contract,
  endpointsHint,
  endpointsLabel,
  noReferencesLabel,
  onRestoreBridgeMacro,
  parsePayloadText,
  patchStepPayload,
  referenceApplyLabel,
  referenceClearLabel,
  referenceCurrentLabel,
  references,
  referenceTitle,
  step,
}: WorkbenchHeadlessWorkflowStepEditorProps) {
  const [bridgePreviewExpanded, setBridgePreviewExpanded] = useState(false);
  const payload = parsePayloadText(step.payloadText);
  const stepActions = readStepActions(payload);
  const bridgeSteps = readBridgeSteps(payload);

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

  if (contract.id === "frontend_macro_bridge") {
    return (
      <>
        <label className="field-label">
          <span>{bridgeMacroIdLabel}</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "macro_id", event.target.value))}
            type="text"
            value={readString(payload, "macro_id")}
          />
        </label>
        <label className="field-label">
          <span>{bridgeReplayModeLabel}</span>
          <input
            className="text-input"
            onChange={(event) => patchStepPayload(step.id, (current) => updatePayloadField(current, "replay_mode", event.target.value))}
            type="text"
            value={readString(payload, "replay_mode")}
          />
        </label>
        <p className="card-copy">{bridgeReplayModeHint}</p>
        <div className="script-panel__payload">
          <span>{bridgeStepCountLabel}</span>
          <code>{String(stepActions.length)}</code>
        </div>
        <div className="card-subhead">
          <strong>{bridgeActionListLabel}</strong>
          <span>{stepActions.length}</span>
        </div>
        <div className="button-row">
          {onRestoreBridgeMacro ? (
            <button className="ghost-button ghost-button--compact" onClick={onRestoreBridgeMacro} type="button">
              {bridgeRestoreLabel}
            </button>
          ) : null}
          <button className="ghost-button ghost-button--compact" onClick={() => setBridgePreviewExpanded((current) => !current)} type="button">
            {bridgePreviewExpanded ? bridgePreviewHideLabel : bridgePreviewShowLabel}
          </button>
        </div>
        <div className="script-panel__catalog">
          {bridgeSteps.map((bridgeStep, index) => (
            <article className="script-panel__action" key={`${step.id}-bridge-${index}`}>
              <div className="script-panel__action-head">
                <strong>{`${index + 1}. ${bridgeStep.action}`}</strong>
                <span>{bridgeReplayModeLabel}</span>
              </div>
              {bridgePreviewExpanded ? (
                <div className="script-panel__payload">
                  <span>{bridgePreviewPayloadLabel}</span>
                  <code>{JSON.stringify(bridgeStep.payload, null, 2)}</code>
                </div>
              ) : null}
            </article>
          ))}
        </div>
      </>
    );
  }

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
