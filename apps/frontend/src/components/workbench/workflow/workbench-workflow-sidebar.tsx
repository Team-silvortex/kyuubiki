"use client";

import type { JobState, WorkflowCatalogEntry } from "@/lib/api";

export type WorkflowSurfaceTab = "overview" | "catalog" | "builder" | "runs";

export type WorkflowRunRecord = {
  jobId: string;
  workflowId: string;
  status: string;
  progress: number;
  currentNode?: string | null;
  summary?: string | null;
  updatedAt?: string | null;
};

type WorkflowSidebarLabels = {
  sectionTitle: string;
  overviewPageLabel: string;
  catalogPageLabel: string;
  builderPageLabel: string;
  runsPageLabel: string;
  overviewHint: string;
  catalogHint: string;
  builderHint: string;
  runsHint: string;
  catalogTitle: string;
  refreshLabel: string;
  runLabel: string;
  emptyCatalogLabel: string;
  noSelectionLabel: string;
  nodesTitle: string;
  edgesTitle: string;
  entryInputsTitle: string;
  outputArtifactsTitle: string;
  datasetContractTitle: string;
  datasetValuesTitle: string;
  datasetValueLabel: string;
  datasetSemanticTypeLabel: string;
  datasetEncodingLabel: string;
  datasetShapeLabel: string;
  datasetAxesLabel: string;
  datasetSchemaLabel: string;
  datasetClassLabel: string;
  datasetNoneLabel: string;
  operatorLabel: string;
  kindLabel: string;
  progressLabel: string;
  currentNodeLabel: string;
  latestSummaryLabel: string;
  openRunLabel: string;
  emptyRunsLabel: string;
  selectForBuilderLabel: string;
  statusReadyLabel: string;
  statusBusyLabel: string;
};

type WorkbenchWorkflowSidebarProps = {
  surfaceTab: WorkflowSurfaceTab;
  onSurfaceTabChange: (tab: WorkflowSurfaceTab) => void;
  labels: WorkflowSidebarLabels;
  workflowCatalogEntries: WorkflowCatalogEntry[];
  workflowCatalogBusy: boolean;
  selectedWorkflowId: string | null;
  selectedWorkflow: WorkflowCatalogEntry | null;
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
  onRefreshWorkflowCatalog: () => void;
  onSelectWorkflow: (workflowId: string) => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onOpenWorkflowRun: (jobId: string) => void;
};

function workflowStatusTone(status: string): string {
  if (status === "completed") return "good";
  if (status === "failed" || status === "cancelled") return "risk";
  return "watch";
}

export function WorkbenchWorkflowSidebar({
  surfaceTab,
  onSurfaceTabChange,
  labels,
  workflowCatalogEntries,
  workflowCatalogBusy,
  selectedWorkflowId,
  selectedWorkflow,
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  onRefreshWorkflowCatalog,
  onSelectWorkflow,
  onRunWorkflowCatalog,
  onOpenWorkflowRun,
}: WorkbenchWorkflowSidebarProps) {
  const selectedGraph = selectedWorkflow?.graph ?? null;
  const selectedNodes = selectedGraph?.nodes ?? [];
  const selectedEdges = selectedGraph?.edges ?? [];
  const selectedDatasetContract = selectedGraph?.dataset_contract ?? null;
  const selectedDatasetValues = selectedDatasetContract?.values ?? [];
  const latestRun = workflowRuns[0] ?? null;

  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${surfaceTab === "overview" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("overview")}
          type="button"
        >
          {labels.overviewPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "catalog" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("catalog")}
          type="button"
        >
          {labels.catalogPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "builder" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("builder")}
          type="button"
        >
          {labels.builderPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "runs" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("runs")}
          type="button"
        >
          {labels.runsPageLabel}
        </button>
      </div>

      {surfaceTab === "overview" ? (
        <div className="runtime-overview-grid">
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.catalogPageLabel}</h2>
            </div>
            <p className="card-copy">{labels.catalogHint}</p>
            <div className="button-row">
              <button onClick={() => onSurfaceTabChange("catalog")} type="button">
                {labels.catalogPageLabel}
              </button>
            </div>
          </section>
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.builderPageLabel}</h2>
            </div>
            <p className="card-copy">{labels.builderHint}</p>
            <div className="button-row">
              <button onClick={() => onSurfaceTabChange("builder")} type="button">
                {labels.builderPageLabel}
              </button>
            </div>
          </section>
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.runsPageLabel}</h2>
            </div>
            <p className="card-copy">{labels.runsHint}</p>
            <div className="button-row">
              <button onClick={() => onSurfaceTabChange("runs")} type="button">
                {labels.runsPageLabel}
              </button>
            </div>
          </section>
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.sectionTitle}</h2>
              <span className={`status-pill status-pill--${workflowCatalogBusy ? "watch" : "good"}`}>
                {workflowCatalogBusy ? labels.statusBusyLabel : labels.statusReadyLabel}
              </span>
            </div>
            <p className="card-copy">{labels.overviewHint}</p>
            {selectedWorkflow ? (
              <div className="sidebar-list">
                <div className="sidebar-list__row">
                  <span>{labels.builderPageLabel}</span>
                  <strong>{selectedWorkflow.name}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>{labels.runsPageLabel}</span>
                  <strong>{latestRun?.status ?? "--"}</strong>
                </div>
              </div>
            ) : null}
          </section>
        </div>
      ) : null}

      {surfaceTab === "catalog" ? (
        <section className="sidebar-card sidebar-card--compact">
          <div className="card-head">
            <h2>{labels.catalogTitle}</h2>
          </div>
          <p className="card-copy">{labels.catalogHint}</p>
          <div className="button-row">
            <button onClick={onRefreshWorkflowCatalog} type="button">
              {labels.refreshLabel}
            </button>
          </div>
          {workflowCatalogEntries.length === 0 ? <p className="card-copy">{labels.emptyCatalogLabel}</p> : null}
          <div className="runtime-overview-grid">
            {workflowCatalogEntries.map((workflow) => (
              <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={workflow.id}>
                <div className="card-head">
                  <h2>{workflow.name}</h2>
                  <span className={`status-pill status-pill--${selectedWorkflowId === workflow.id ? "good" : "watch"}`}>
                    {workflow.version}
                  </span>
                </div>
                <p className="card-copy">{workflow.summary}</p>
                <div className="button-row">
                  <button
                    onClick={() => {
                      onSelectWorkflow(workflow.id);
                      onSurfaceTabChange("builder");
                    }}
                    type="button"
                  >
                    {labels.selectForBuilderLabel}
                  </button>
                  <button onClick={() => onRunWorkflowCatalog(workflow.id)} type="button">
                    {labels.runLabel}
                  </button>
                </div>
              </section>
            ))}
          </div>
        </section>
      ) : null}

      {surfaceTab === "builder" ? (
        <section className="sidebar-card sidebar-card--compact">
          {selectedWorkflow ? (
            <>
              <div className="card-head">
                <h2>{selectedWorkflow.name}</h2>
                <span className="status-pill status-pill--good">{selectedWorkflow.version}</span>
              </div>
              <p className="card-copy">{selectedWorkflow.summary}</p>
              <div className="button-row">
                <button onClick={() => onRunWorkflowCatalog(selectedWorkflow.id)} type="button">
                  {labels.runLabel}
                </button>
              </div>
              <div className="sidebar-list">
                <div className="sidebar-list__row">
                  <span>{labels.nodesTitle}</span>
                  <strong>{selectedNodes.length}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>{labels.edgesTitle}</span>
                  <strong>{selectedEdges.length}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>{labels.entryInputsTitle}</span>
                  <strong>{selectedWorkflow.entry_inputs.length}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>{labels.outputArtifactsTitle}</span>
                  <strong>{selectedWorkflow.output_artifacts.length}</strong>
                </div>
              </div>

              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{labels.nodesTitle}</h2>
                </div>
                <div className="sidebar-list">
                  {selectedNodes.map((node) => (
                    <div className="sidebar-list__row" key={node.id}>
                      <span>
                        {node.id}
                      </span>
                      <strong>
                        {labels.kindLabel}: {node.kind}
                        {node.operator_id ? ` · ${labels.operatorLabel}: ${node.operator_id}` : ""}
                        {node.outputs?.some((port) => port.dataset_value)
                          ? ` · ${labels.datasetValueLabel}: ${node.outputs
                              ?.map((port) => port.dataset_value)
                              .filter(Boolean)
                              .join(", ")}`
                          : ""}
                      </strong>
                    </div>
                  ))}
                </div>
              </section>

              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{labels.edgesTitle}</h2>
                </div>
                <div className="sidebar-list">
                  {selectedEdges.map((edge) => (
                    <div className="sidebar-list__row" key={edge.id}>
                      <span>
                        {edge.from.node}.{edge.from.port} → {edge.to.node}.{edge.to.port}
                      </span>
                      <strong>
                        {edge.artifact_type}
                        {edge.dataset_value ? ` · ${labels.datasetValueLabel}: ${edge.dataset_value}` : ""}
                      </strong>
                    </div>
                  ))}
                </div>
              </section>

              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{labels.datasetContractTitle}</h2>
                  <span className={`status-pill status-pill--${selectedDatasetContract ? "good" : "watch"}`}>
                    {selectedDatasetContract?.version ?? "--"}
                  </span>
                </div>
                {selectedDatasetContract ? (
                  <>
                    <p className="card-copy">
                      {selectedDatasetContract.name ?? selectedDatasetContract.id}
                    </p>
                    <div className="sidebar-list">
                      <div className="sidebar-list__row">
                        <span>{labels.datasetValuesTitle}</span>
                        <strong>{selectedDatasetValues.length}</strong>
                      </div>
                    </div>
                    <div className="sidebar-stack">
                      {selectedDatasetValues.map((value) => {
                        const axes = value.shape?.axes ?? [];
                        const schemaLabel = value.schema_ref
                          ? `${value.schema_ref.schema}@${value.schema_ref.version}`
                          : "--";
                        const shapeLabel =
                          axes.length > 0
                            ? axes
                                .map((axis) =>
                                  axis.size != null ? `${axis.id}[${axis.size}]` : axis.id,
                                )
                                .join(" × ")
                            : "--";
                        return (
                          <section className="sidebar-card sidebar-card--compact" key={value.id}>
                            <div className="card-head">
                              <h2>{value.id}</h2>
                              <span className="status-pill status-pill--watch">
                                {value.data_class}
                              </span>
                            </div>
                            <div className="sidebar-list">
                              <div className="sidebar-list__row">
                                <span>{labels.datasetSemanticTypeLabel}</span>
                                <strong>{value.semantic_type ?? "--"}</strong>
                              </div>
                              <div className="sidebar-list__row">
                                <span>{labels.datasetEncodingLabel}</span>
                                <strong>{value.encoding ?? "--"}</strong>
                              </div>
                              <div className="sidebar-list__row">
                                <span>{labels.datasetClassLabel}</span>
                                <strong>{value.element_type}</strong>
                              </div>
                              <div className="sidebar-list__row">
                                <span>{labels.datasetShapeLabel}</span>
                                <strong>{shapeLabel}</strong>
                              </div>
                              <div className="sidebar-list__row">
                                <span>{labels.datasetSchemaLabel}</span>
                                <strong>{schemaLabel}</strong>
                              </div>
                            </div>
                            {axes.length > 0 ? (
                              <div className="sidebar-list">
                                {axes.map((axis) => (
                                  <div className="sidebar-list__row" key={`${value.id}:${axis.id}`}>
                                    <span>{labels.datasetAxesLabel}</span>
                                    <strong>
                                      {axis.id}
                                      {axis.semantic ? ` · ${axis.semantic}` : ""}
                                      {axis.size != null ? ` · ${axis.size}` : ""}
                                    </strong>
                                  </div>
                                ))}
                              </div>
                            ) : null}
                          </section>
                        );
                      })}
                    </div>
                  </>
                ) : (
                  <p className="card-copy">{labels.datasetNoneLabel}</p>
                )}
              </section>

              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{labels.entryInputsTitle}</h2>
                </div>
                <div className="sidebar-list">
                  {selectedWorkflow.entry_inputs.map((artifact) => (
                    <div className="sidebar-list__row" key={`${artifact.node_id}:${artifact.artifact_type}`}>
                      <span>{artifact.node_id}</span>
                      <strong>{artifact.artifact_type}</strong>
                    </div>
                  ))}
                </div>
              </section>

              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{labels.outputArtifactsTitle}</h2>
                </div>
                <div className="sidebar-list">
                  {selectedWorkflow.output_artifacts.map((artifact) => (
                    <div className="sidebar-list__row" key={`${artifact.node_id}:${artifact.artifact_type}`}>
                      <span>{artifact.node_id}</span>
                      <strong>{artifact.artifact_type}</strong>
                    </div>
                  ))}
                </div>
              </section>
            </>
          ) : (
            <p className="card-copy">{labels.noSelectionLabel}</p>
          )}
        </section>
      ) : null}

      {surfaceTab === "runs" ? (
        <section className="sidebar-card sidebar-card--compact">
          <div className="card-head">
            <h2>{labels.runsPageLabel}</h2>
          </div>
          <p className="card-copy">{labels.runsHint}</p>
          {latestJob ? (
            <div className="sidebar-list">
              <div className="sidebar-list__row">
                <span>{labels.progressLabel}</span>
                <strong>{Math.round((latestJob.progress ?? 0) * 100)}%</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.currentNodeLabel}</span>
                <strong>{latestRun?.currentNode ?? "--"}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.latestSummaryLabel}</span>
                <strong>{latestWorkflowSummary ?? "--"}</strong>
              </div>
            </div>
          ) : null}
          {workflowRuns.length === 0 ? <p className="card-copy">{labels.emptyRunsLabel}</p> : null}
          <div className="runtime-overview-grid">
            {workflowRuns.map((run) => (
              <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={run.jobId}>
                <div className="card-head">
                  <h2>{run.workflowId}</h2>
                  <span className={`status-pill status-pill--${workflowStatusTone(run.status)}`}>{run.status}</span>
                </div>
                <div className="sidebar-list">
                  <div className="sidebar-list__row">
                    <span>{labels.progressLabel}</span>
                    <strong>{Math.round(run.progress * 100)}%</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{labels.currentNodeLabel}</span>
                    <strong>{run.currentNode ?? "--"}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{labels.latestSummaryLabel}</span>
                    <strong>{run.summary ?? "--"}</strong>
                  </div>
                </div>
                <div className="button-row">
                  <button onClick={() => onOpenWorkflowRun(run.jobId)} type="button">
                    {labels.openRunLabel}
                  </button>
                </div>
              </section>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}
