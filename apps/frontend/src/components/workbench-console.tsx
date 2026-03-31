"use client";

import { memo } from "react";

type SidebarSection = "study" | "model" | "library" | "system";

type ConsoleElement = {
  index: number;
  x1?: number;
  x2?: number;
  node_i?: number;
  node_j?: number;
  node_k?: number;
  stress?: number;
  axial_force?: number;
  von_mises?: number;
};

type WorkbenchConsoleProps = {
  sidebarSection: SidebarSection;
  title: string;
  subtitle: string;
  modelMessageTitle: string;
  reportMessageTitle: string;
  message: string;
  dragNodeLabel: string;
  noNodeSelectedLabel: string;
  loadCaseLabel: string;
  diagnosticsLabel: string;
  selectedNodeId: string | null;
  selectedNodeX: number | null | undefined;
  selectedNodeY: number | null | undefined;
  selectedNodeLoadY: number | null | undefined;
  selectedNodeIssueCount: number | null;
  elementTitle: string;
  spanLabel: string;
  stressLabel: string;
  axialForceLabel: string;
  elements: ConsoleElement[];
};

function fixed(value: number | null | undefined, digits = 2): string {
  return typeof value === "number" ? value.toFixed(digits) : "--";
}

function scientific(value: number | null | undefined, digits = 3): string {
  return typeof value === "number" ? value.toExponential(digits) : "--";
}

function WorkbenchConsoleInner({
  sidebarSection,
  title,
  subtitle,
  modelMessageTitle,
  reportMessageTitle,
  message,
  dragNodeLabel,
  noNodeSelectedLabel,
  loadCaseLabel,
  diagnosticsLabel,
  selectedNodeId,
  selectedNodeX,
  selectedNodeY,
  selectedNodeLoadY,
  selectedNodeIssueCount,
  elementTitle,
  spanLabel,
  stressLabel,
  axialForceLabel,
  elements,
}: WorkbenchConsoleProps) {
  return (
    <section className="panel console-panel">
      <div className="panel-head">
        <h2>{title}</h2>
        <span>{subtitle}</span>
      </div>
      <div className="console-grid">
        <div className="console-card">
          <h3>{sidebarSection === "model" ? modelMessageTitle : reportMessageTitle}</h3>
          {sidebarSection === "model" ? (
            <div className="metric-grid">
              <div>
                <span>ID</span>
                <strong>{selectedNodeId ?? noNodeSelectedLabel}</strong>
              </div>
              <div>
                <span>X</span>
                <strong>{fixed(selectedNodeX)}</strong>
              </div>
              <div>
                <span>Y</span>
                <strong>{fixed(selectedNodeY)}</strong>
              </div>
              <div>
                <span>{loadCaseLabel}</span>
                <strong>{fixed(selectedNodeLoadY, 0)} N</strong>
              </div>
              <div>
                <span>{diagnosticsLabel}</span>
                <strong>{selectedNodeIssueCount ?? "--"}</strong>
              </div>
            </div>
          ) : (
            <p>{message}</p>
          )}
        </div>
        <div className="console-card">
          <h3>{elementTitle}</h3>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>#</th>
                  <th>{spanLabel}</th>
                  <th>{stressLabel}</th>
                  <th>{axialForceLabel}</th>
                </tr>
              </thead>
              <tbody>
                {elements.map((element) => (
                  <tr key={element.index}>
                    <td>{element.index}</td>
                    <td>
                      {"x1" in element && typeof element.x1 === "number"
                        ? `${fixed(element.x1, 2)} - ${fixed(element.x2, 2)}`
                        : typeof element.node_k === "number"
                          ? `${element.node_i} - ${element.node_j} - ${element.node_k}`
                          : `${element.node_i} - ${element.node_j}`}
                    </td>
                    <td>{scientific(typeof element.von_mises === "number" ? element.von_mises : element.stress)}</td>
                    <td>{scientific(element.axial_force)}</td>
                  </tr>
                ))}
                {elements.length === 0 ? (
                  <tr>
                    <td colSpan={4}>--</td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </section>
  );
}

export const WorkbenchConsole = memo(WorkbenchConsoleInner);
