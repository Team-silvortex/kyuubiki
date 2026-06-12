"use client";

import { useRef } from "react";
import type { WorkflowCatalogEntryArtifact } from "@/lib/api";
import {
  applyWorkflowFemFieldDefault,
  applyWorkflowFemSectionDefaults,
  resolveWorkflowFemInputProfile,
  summarizeWorkflowFemInputCoverage,
} from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";
import {
  applyWorkflowFemMaterialPreset,
  resolveWorkflowFemMaterialPresets,
} from "@/components/workbench/workflow/workbench-workflow-fem-material-presets";
import {
  applyWorkflowFemSectionPreset,
  resolveWorkflowFemSectionPresets,
} from "@/components/workbench/workflow/workbench-workflow-fem-section-presets";
import {
  formatWorkflowFemFieldLabel,
  resolveWorkflowFemFieldMetadata,
} from "@/components/workbench/workflow/workbench-workflow-fem-field-metadata";
import { validateWorkflowFemInputPayload } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowInputArtifactsCardProps = {
  labels: WorkflowSidebarLabels;
  entryInputs: WorkflowCatalogEntryArtifact[];
  inputTexts: Record<string, string>;
  invalidKeys: string[];
  onChangeInputText: (nodeId: string, value: string) => void;
};

export function WorkbenchWorkflowInputArtifactsCard({
  labels,
  entryInputs,
  inputTexts,
  invalidKeys,
  onChangeInputText,
}: WorkbenchWorkflowInputArtifactsCardProps) {
  const textareaRefs = useRef<Record<string, HTMLTextAreaElement | null>>({});

  function focusInsertedField(nodeId: string, nextText: string, field: string) {
    window.setTimeout(() => {
      const textarea = textareaRefs.current[nodeId];
      if (!textarea) return;
      const marker = `"${field}"`;
      const start = nextText.indexOf(marker);
      textarea.focus();
      if (start >= 0) {
        textarea.setSelectionRange(start, start + marker.length);
        textarea.scrollTop = textarea.scrollHeight * (start / Math.max(nextText.length, 1));
      }
    }, 0);
  }

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.inputArtifactsTitle}</h2>
        <span className="status-pill status-pill--watch">{labels.artifactDraftLocalLabel}</span>
      </div>
      <p className="card-copy">{labels.inputArtifactsHint}</p>
      {entryInputs.length === 0 ? <p className="card-copy">{labels.emptyCatalogLabel}</p> : null}
      <div className="runtime-overview-grid">
        {entryInputs.map((artifact) => {
          const isInvalid = invalidKeys.includes(artifact.node_id);
          const profile = resolveWorkflowFemInputProfile(artifact.artifact_type);
          const parsedInput = (() => {
            try {
              return JSON.parse(inputTexts[artifact.node_id] ?? "null");
            } catch {
              return null;
            }
          })();
          const coverage = summarizeWorkflowFemInputCoverage(artifact.artifact_type, parsedInput);
          const materialPresets = resolveWorkflowFemMaterialPresets(artifact.artifact_type);
          const validationIssues = validateWorkflowFemInputPayload(artifact.artifact_type, parsedInput);
          const physicsIssues = validationIssues.filter((entry) => entry.category === "physics");
          const contractIssues = validationIssues.filter((entry) => entry.category === "contract");
          return (
            <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={artifact.node_id}>
              <div className="card-head">
                <h2>{artifact.node_id}</h2>
                <span className={`status-pill status-pill--${isInvalid ? "risk" : "good"}`}>
                  {artifact.artifact_type}
                </span>
              </div>
              {artifact.description ? <p className="card-copy">{artifact.description}</p> : null}
              {validationIssues.length > 0 ? (
                <div style={{ display: "grid", gap: "0.15rem", marginBottom: "0.75rem" }}>
                  <p className="card-copy" style={{ margin: 0 }}>
                    Input checks: {validationIssues.length} warning(s), {physicsIssues.length} physics, {contractIssues.length} contract
                  </p>
                  {validationIssues.slice(0, 4).map((entry) => (
                    <p className="card-copy" key={`${artifact.node_id}:warning:${entry.category}:${entry.sectionKey}:${entry.field}:${entry.message}`} style={{ margin: 0 }}>
                      {entry.category} / {entry.sectionKey}.{entry.field}: {entry.message}
                    </p>
                  ))}
                </div>
              ) : null}
              {profile ? (
                <details open style={{ marginBottom: "0.75rem" }}>
                  <summary className="card-copy" style={{ cursor: "pointer" }}>
                    {profile.studyFamily}: material / boundary / load / control
                  </summary>
                  <div style={{ display: "grid", gap: "0.35rem", marginTop: "0.5rem" }}>
                    {profile.sections.map((section) => {
                      const matched = coverage.find((entry) => entry.key === section.key)?.matchedFields ?? [];
                      const missing = section.fields.filter((field) => !matched.includes(field));
                      const sectionPresets = resolveWorkflowFemSectionPresets(artifact.artifact_type, section.key);
                      const expectedLabels = section.fields.map((field) => formatWorkflowFemFieldLabel(field));
                      const detectedLabels = matched.map((field) => formatWorkflowFemFieldLabel(field));
                      const sectionIssues = validationIssues.filter((entry) => entry.sectionKey === section.key);
                      return (
                        <details key={`${artifact.node_id}:${section.key}`} style={{ paddingLeft: "0.25rem" }}>
                          <summary className="card-copy" style={{ cursor: "pointer" }}>
                            <strong>{section.label}</strong>: {matched.length > 0 ? `${matched.length} field(s) detected` : "no detected fields"}
                          </summary>
                          <div style={{ display: "grid", gap: "0.25rem", marginTop: "0.35rem", paddingLeft: "0.5rem" }}>
                            <p className="card-copy" style={{ margin: 0 }}>{section.summary}</p>
                            {sectionIssues.length > 0 ? (
                              <div style={{ display: "grid", gap: "0.1rem" }}>
                                {sectionIssues.map((entry) => (
                                  <p className="card-copy" key={`${artifact.node_id}:${section.key}:issue:${entry.category}:${entry.field}:${entry.message}`} style={{ margin: 0 }}>
                                    Warning [{entry.category}]: {entry.field} {entry.message}
                                  </p>
                                ))}
                              </div>
                            ) : null}
                            <p className="card-copy" style={{ margin: 0 }}>Expected: {expectedLabels.join(", ")}</p>
                            {matched.length > 0 ? (
                              <p className="card-copy" style={{ margin: 0 }}>Detected: {detectedLabels.join(", ")}</p>
                            ) : null}
                            <div style={{ display: "grid", gap: "0.1rem" }}>
                              {section.fields.map((field) => {
                                const metadata = resolveWorkflowFemFieldMetadata(field);
                                return metadata ? (
                                  <p className="card-copy" key={`${artifact.node_id}:${section.key}:meta:${field}`} style={{ margin: 0 }}>
                                    {field}: {metadata.summary} Unit {metadata.unit}
                                  </p>
                                ) : null;
                              })}
                            </div>
                            {section.key === "material" && materialPresets.length > 0 ? (
                              <div style={{ display: "grid", gap: "0.25rem" }}>
                                <p className="card-copy" style={{ margin: 0 }}>
                                  Presets: {materialPresets.map((preset) => preset.label).join(", ")}
                                </p>
                                <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                                  {materialPresets.map((preset) => (
                                    <button
                                      disabled={parsedInput === null}
                                      key={`${artifact.node_id}:preset:${preset.key}`}
                                      onClick={() => {
                                        if (parsedInput === null) return;
                                        const nextPayload = applyWorkflowFemMaterialPreset(
                                          artifact.artifact_type,
                                          parsedInput,
                                          preset.key,
                                        );
                                        const nextText = JSON.stringify(nextPayload, null, 2);
                                        onChangeInputText(artifact.node_id, nextText);
                                        focusInsertedField(
                                          artifact.node_id,
                                          nextText,
                                          Object.keys(preset.values)[0] ?? section.fields[0] ?? "",
                                        );
                                      }}
                                      title={preset.summary}
                                      type="button"
                                    >
                                      {preset.label}
                                    </button>
                                  ))}
                                </div>
                              </div>
                            ) : null}
                            {section.key !== "material" && sectionPresets.length > 0 ? (
                              <div style={{ display: "grid", gap: "0.25rem" }}>
                                <p className="card-copy" style={{ margin: 0 }}>
                                  Presets: {sectionPresets.map((preset) => preset.label).join(", ")}
                                </p>
                                <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                                  {sectionPresets.map((preset) => (
                                    <button
                                      disabled={parsedInput === null}
                                      key={`${artifact.node_id}:${section.key}:preset:${preset.key}`}
                                      onClick={() => {
                                        if (parsedInput === null) return;
                                        const nextPayload = applyWorkflowFemSectionPreset(
                                          artifact.artifact_type,
                                          parsedInput,
                                          section.key,
                                          preset.key,
                                        );
                                        const nextText = JSON.stringify(nextPayload, null, 2);
                                        onChangeInputText(artifact.node_id, nextText);
                                        focusInsertedField(
                                          artifact.node_id,
                                          nextText,
                                          Object.keys(preset.values)[0] ?? section.fields[0] ?? "",
                                        );
                                      }}
                                      title={preset.summary}
                                      type="button"
                                    >
                                      {preset.label}
                                    </button>
                                  ))}
                                </div>
                              </div>
                            ) : null}
                            {missing.length > 0 ? (
                              <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                                {missing.map((field) => (
                                  <button
                                    disabled={parsedInput === null}
                                    key={`${artifact.node_id}:${section.key}:${field}`}
                                    onClick={() => {
                                      if (parsedInput === null) return;
                                      const nextPayload = applyWorkflowFemFieldDefault(
                                        artifact.artifact_type,
                                        parsedInput,
                                        section.key,
                                        field,
                                      );
                                      const nextText = JSON.stringify(nextPayload, null, 2);
                                      onChangeInputText(artifact.node_id, nextText);
                                      focusInsertedField(artifact.node_id, nextText, field);
                                    }}
                                    type="button"
                                  >
                                    + {formatWorkflowFemFieldLabel(field)}
                                  </button>
                                ))}
                              </div>
                            ) : null}
                            <button
                              disabled={parsedInput === null}
                              onClick={() => {
                                if (parsedInput === null) return;
                                const nextPayload = applyWorkflowFemSectionDefaults(
                                  artifact.artifact_type,
                                  parsedInput,
                                  section.key,
                                );
                                const nextText = JSON.stringify(nextPayload, null, 2);
                                onChangeInputText(artifact.node_id, nextText);
                                focusInsertedField(
                                  artifact.node_id,
                                  nextText,
                                  section.fields.find((field) => !matched.includes(field)) ?? section.fields[0] ?? "",
                                );
                              }}
                              type="button"
                            >
                              Insert missing {section.label.toLowerCase()} fields
                            </button>
                          </div>
                        </details>
                      );
                    })}
                  </div>
                </details>
              ) : null}
              <textarea
                className="shell-textarea"
                onChange={(event) => onChangeInputText(artifact.node_id, event.target.value)}
                ref={(element) => {
                  textareaRefs.current[artifact.node_id] = element;
                }}
                rows={8}
                value={inputTexts[artifact.node_id] ?? ""}
              />
              {isInvalid ? <p className="card-copy">{labels.runDraftInvalidInputsLabel}</p> : null}
            </section>
          );
        })}
      </div>
    </section>
  );
}
