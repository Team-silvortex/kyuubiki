"use client";

import { useMemo } from "react";
import type {
  WorkflowCatalogEntryArtifact,
  WorkflowGraphNode,
} from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowArtifactCardProps = {
  labels: WorkflowSidebarLabels;
  title: string;
  addLabel: string;
  mode: "entry" | "output";
  artifacts: WorkflowCatalogEntryArtifact[];
  selectedNodes: WorkflowGraphNode[];
  onAddArtifact: () => void;
  onRemoveArtifact: (index: number) => void;
  onUpdateArtifact: (
    index: number,
    updater: (artifact: WorkflowCatalogEntryArtifact) => WorkflowCatalogEntryArtifact,
  ) => void;
};

export function WorkbenchWorkflowArtifactCard({
  labels,
  title,
  addLabel,
  mode,
  artifacts,
  selectedNodes,
  onAddArtifact,
  onRemoveArtifact,
  onUpdateArtifact,
}: WorkbenchWorkflowArtifactCardProps) {
  const nodeIds = useMemo(() => selectedNodes.map((node) => node.id), [selectedNodes]);

  const nodeArtifactTypes = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const node of selectedNodes) {
      const ports = mode === "entry" ? node.inputs ?? [] : node.outputs ?? [];
      map.set(
        node.id,
        Array.from(new Set(ports.map((port) => port.artifact_type))).filter(Boolean),
      );
    }
    return map;
  }, [mode, selectedNodes]);

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span className="status-pill status-pill--watch">{labels.artifactDraftLocalLabel}</span>
      </div>
      <div className="button-row">
        <button onClick={onAddArtifact} type="button">
          {addLabel}
        </button>
      </div>
      {artifacts.length > 0 ? (
        <div className="sidebar-stack">
          {artifacts.map((artifact, index) => (
            <section className="sidebar-card sidebar-card--compact" key={`${artifact.node_id}:${artifact.artifact_type}:${index}`}>
              {(() => {
                const suggestedTypes = nodeArtifactTypes.get(artifact.node_id) ?? [];
                const suggestedArtifactType = suggestedTypes[0];
                const needsSuggestion =
                  suggestedTypes.length > 0 && !suggestedTypes.includes(artifact.artifact_type);
                return (
                  <>
              <div className="button-row">
                <button onClick={() => onRemoveArtifact(index)} type="button">
                  {labels.artifactRemoveLabel}
                </button>
                {needsSuggestion && suggestedArtifactType ? (
                  <button
                    onClick={() =>
                      onUpdateArtifact(index, (current) => ({
                        ...current,
                        artifact_type: suggestedArtifactType,
                      }))
                    }
                    type="button"
                  >
                    {labels.artifactAdoptSuggestedLabel}
                  </button>
                ) : null}
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{labels.artifactNodeLabel}</span>
                  <input
                    list={`${title}-node-options`}
                    onChange={(event) =>
                      onUpdateArtifact(index, (current) => ({
                        ...current,
                        node_id: event.target.value,
                      }))
                    }
                    value={artifact.node_id}
                  />
                </label>
                <label>
                  <span>{labels.artifactTypeLabel}</span>
                  <input
                    list={`${title}-artifact-options-${index}`}
                    onChange={(event) =>
                      onUpdateArtifact(index, (current) => ({
                        ...current,
                        artifact_type: event.target.value,
                      }))
                    }
                    value={artifact.artifact_type}
                  />
                </label>
                <label>
                  <span>{labels.artifactDescriptionLabel}</span>
                  <input
                    onChange={(event) =>
                      onUpdateArtifact(index, (current) => ({
                        ...current,
                        description: event.target.value,
                      }))
                    }
                    value={artifact.description}
                  />
                </label>
              </div>
              <datalist id={`${title}-artifact-options-${index}`}>
                {suggestedTypes.map((artifactType) => (
                  <option key={artifactType} value={artifactType} />
                ))}
              </datalist>
                  </>
                );
              })()}
            </section>
          ))}
        </div>
      ) : (
        <p className="card-copy">--</p>
      )}
      <datalist id={`${title}-node-options`}>
        {nodeIds.map((nodeId) => (
          <option key={nodeId} value={nodeId} />
        ))}
      </datalist>
    </section>
  );
}
