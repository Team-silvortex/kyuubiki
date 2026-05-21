"use client";

import { memo, useMemo, useState } from "react";

import { VirtualList } from "@/components/ui/virtual-list";
import type { ProjectRecord } from "@/lib/api";
import type { StudyDomainKey } from "@/lib/workbench/view-models";

type LibraryPanelTab = "jobs" | "results" | "models" | "projects" | "samples";
type SamplePage = "catalog" | "import";
type ProjectPage = "manage" | "exchange";
type ModelPage = "saved" | "versions";

type SampleRow = {
  id: string;
  name: string;
  kindLabel: string;
  domainKey: StudyDomainKey;
  domainLabel: string;
  familyKey: string;
  familyLabel: string;
  href: string;
  summary: string;
};

type ModelRow = {
  id: string;
  name: string;
  kindLabel: string;
  updatedAt: string;
  versionLabel: string;
};

type VersionRow = {
  id: string;
  name: string;
  versionLabel: string;
  updatedAt: string;
};

type JobRow = {
  id: string;
  shortId: string;
  status: string;
  updatedAt: string;
  hasResult: string;
};

type LibraryLabels = {
  refresh: string;
  historyHint: string;
  importModel: string;
  importHint: string;
  sampleLibrary: string;
  projectLibrary: string;
  projectNameField: string;
  projectDescriptionField: string;
  createProject: string;
  updateProject: string;
  deleteProject: string;
  exportProjectJson: string;
  exportProjectZip: string;
  importProject: string;
  importProjectHint: string;
  projectEmpty: string;
  modelName: string;
  savedModels: string;
  save: string;
  saveAs: string;
  deleteSavedModel: string;
  noSavedModels: string;
  versions: string;
  renameVersion: string;
  deleteVersion: string;
  noVersions: string;
  historyEmpty: string;
  updatedAt: string;
  hasResult: string;
  yes: string;
  no: string;
  jobWorkspaceTitle: string;
  jobWorkspaceHint: string;
  resultWorkspaceTitle: string;
  resultWorkspaceHint: string;
  modelWorkspaceTitle: string;
  modelWorkspaceHint: string;
  openLatestJob: string;
  openLatestResult: string;
  openLatestModel: string;
  waitingJobs: string;
  readyResults: string;
  savedCount: string;
  versionCount: string;
  sections: { library: string };
  tabs: { jobs: string; results: string; models: string; projects: string; samples: string };
  sampleCatalogPage: string;
  sampleImportPage: string;
  projectManagePage: string;
  projectExchangePage: string;
  modelSavedPage: string;
  modelVersionsPage: string;
  kinds: Record<string, string>;
  studyDomain: string;
  noDomainStudies: string;
  none: string;
};

type WorkbenchLibrarySidebarProps = {
  libraryTab: LibraryPanelTab;
  onLibraryTabChange: (tab: LibraryPanelTab) => void;
  labels: LibraryLabels;
  sampleRows: SampleRow[];
  projects: ProjectRecord[];
  selectedProjectId: string | null;
  onSelectedProjectChange: (projectId: string | null) => void;
  projectNameDraft: string;
  onProjectNameDraftChange: (value: string) => void;
  projectDescriptionDraft: string;
  onProjectDescriptionDraftChange: (value: string) => void;
  onCreateProject: () => void;
  onUpdateProject: () => void;
  onDeleteProject: () => void;
  onExportProjectJson: () => void;
  onExportProjectZip: () => void;
  onImportProjectBundle: (file: File | undefined) => void | Promise<void>;
  selectedProjectModelCount: number;
  modelRows: ModelRow[];
  selectedModelId: string | null;
  loadedModelName: string;
  onLoadedModelNameChange: (value: string) => void;
  onSaveModel: (saveAs: boolean) => void;
  onDeleteSavedModel: () => void;
  onOpenSavedModel: (modelId: string) => void;
  versionRows: VersionRow[];
  modelVersionCount: number;
  selectedVersionId: string | null;
  onRenameSelectedVersion: () => void;
  onDeleteSelectedVersion: () => void;
  onOpenSavedVersion: (versionId: string) => void;
  jobRows: JobRow[];
  jobCount: number;
  activeJobId: string | null;
  onOpenHistoryJob: (jobId: string) => void;
  onOpenSample: (href: string) => void;
  onRefresh: () => void;
  onImportModel: (file: File | undefined) => void;
};

type HistoryWorkspaceCardProps = {
  title: string;
  hint: string;
  actionLabel: string;
  actionDisabled: boolean;
  onAction: () => void;
  metrics: Array<{ label: string; value: string | number }>;
};

function HistoryWorkspaceCard({ title, hint, actionLabel, actionDisabled, onAction, metrics }: HistoryWorkspaceCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
      </div>
      <p className="card-copy">{hint}</p>
      <div className="sidebar-list sidebar-list--metrics">
        {metrics.map((metric) => (
          <div key={metric.label} className="sidebar-list__row">
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
          </div>
        ))}
      </div>
      <div className="button-row">
        <button className="ghost-button" disabled={actionDisabled} onClick={onAction} type="button">
          {actionLabel}
        </button>
      </div>
    </section>
  );
}

export const WorkbenchLibrarySidebar = memo(function WorkbenchLibrarySidebar({
  libraryTab,
  onLibraryTabChange,
  labels,
  sampleRows,
  projects,
  selectedProjectId,
  onSelectedProjectChange,
  projectNameDraft,
  onProjectNameDraftChange,
  projectDescriptionDraft,
  onProjectDescriptionDraftChange,
  onCreateProject,
  onUpdateProject,
  onDeleteProject,
  onExportProjectJson,
  onExportProjectZip,
  onImportProjectBundle,
  selectedProjectModelCount,
  modelRows,
  selectedModelId,
  loadedModelName,
  onLoadedModelNameChange,
  onSaveModel,
  onDeleteSavedModel,
  onOpenSavedModel,
  versionRows,
  modelVersionCount,
  selectedVersionId,
  onRenameSelectedVersion,
  onDeleteSelectedVersion,
  onOpenSavedVersion,
  jobRows,
  jobCount,
  activeJobId,
  onOpenHistoryJob,
  onOpenSample,
  onRefresh,
  onImportModel,
}: WorkbenchLibrarySidebarProps) {
  const [selectedSampleDomain, setSelectedSampleDomain] = useState<StudyDomainKey>("mechanical");
  const [samplePage, setSamplePage] = useState<SamplePage>("catalog");
  const [projectPage, setProjectPage] = useState<ProjectPage>("manage");
  const [modelPage, setModelPage] = useState<ModelPage>("saved");
  const groupedSampleRows = useMemo(() => {
    const groups = new Map<string, { label: string; rows: SampleRow[] }>();
    for (const sample of sampleRows.filter((entry) => entry.domainKey === selectedSampleDomain)) {
      const existing = groups.get(sample.familyKey);
      if (existing) {
        existing.rows.push(sample);
        continue;
      }
      groups.set(sample.familyKey, { label: sample.familyLabel, rows: [sample] });
    }
    return Array.from(groups.values());
  }, [sampleRows, selectedSampleDomain]);

  const sampleDomainOptions = useMemo(
    () =>
      [
        { key: "mechanical" as const, label: sampleRows.find((entry) => entry.domainKey === "mechanical")?.domainLabel ?? "Mechanical" },
        { key: "thermal" as const, label: sampleRows.find((entry) => entry.domainKey === "thermal")?.domainLabel ?? "Thermal" },
        {
          key: "thermoMechanical" as const,
          label: sampleRows.find((entry) => entry.domainKey === "thermoMechanical")?.domainLabel ?? "Thermo-mechanical",
        },
      ] satisfies Array<{ key: StudyDomainKey; label: string }>,
    [sampleRows],
  );
  const resultRows = useMemo(() => jobRows.filter((row) => row.hasResult === labels.yes), [jobRows, labels.yes]);
  const latestJobRow = jobRows[0] ?? null;
  const latestResultRow = resultRows[0] ?? null;
  const latestModelRow = modelRows[0] ?? null;
  const waitingJobsCount = useMemo(() => jobRows.filter((row) => row.hasResult === labels.no).length, [jobRows, labels.no]);

  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab panel-tab--icon${libraryTab === "jobs" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("jobs")} type="button"><span className="panel-tab__glyph">J</span><span>{labels.tabs.jobs}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "results" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("results")} type="button"><span className="panel-tab__glyph">R</span><span>{labels.tabs.results}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "models" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("models")} type="button"><span className="panel-tab__glyph">M</span><span>{labels.tabs.models}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "projects" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("projects")} type="button"><span className="panel-tab__glyph">P</span><span>{labels.tabs.projects}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "samples" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("samples")} type="button"><span className="panel-tab__glyph">S</span><span>{labels.tabs.samples}</span></button>
      </div>

      {libraryTab === "jobs" ? (
        <>
          <HistoryWorkspaceCard
            title={labels.jobWorkspaceTitle}
            hint={labels.jobWorkspaceHint}
            actionLabel={labels.openLatestJob}
            actionDisabled={!latestJobRow}
            onAction={() => latestJobRow && onOpenHistoryJob(latestJobRow.id)}
            metrics={[
              { label: labels.tabs.jobs, value: jobCount },
              { label: labels.waitingJobs, value: waitingJobsCount },
            ]}
          />
          <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.tabs.jobs}</h2>
            <span>{jobCount}</span>
          </div>
          <VirtualList
            className="history-list"
            items={jobRows}
            itemHeight={112}
            maxHeight={360}
            emptyState={<p className="card-copy">{labels.historyEmpty}</p>}
            itemKey={(historyJob) => historyJob.id}
            renderItem={(historyJob) => (
              <button
                className={`history-item${activeJobId === historyJob.id ? " history-item--active" : ""}`}
                onClick={() => onOpenHistoryJob(historyJob.id)}
                type="button"
              >
                <strong>{historyJob.shortId}</strong>
                <span>{historyJob.status}</span>
                <small>
                  {labels.updatedAt}: {historyJob.updatedAt}
                </small>
                <small>
                  {labels.hasResult}: {historyJob.hasResult}
                </small>
              </button>
            )}
          />
          </section>
        </>
      ) : null}

      {libraryTab === "results" ? (
        <>
          <HistoryWorkspaceCard
            title={labels.resultWorkspaceTitle}
            hint={labels.resultWorkspaceHint}
            actionLabel={labels.openLatestResult}
            actionDisabled={!latestResultRow}
            onAction={() => latestResultRow && onOpenHistoryJob(latestResultRow.id)}
            metrics={[
              { label: labels.readyResults, value: resultRows.length },
              { label: labels.tabs.jobs, value: jobCount },
            ]}
          />
          <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.tabs.results}</h2>
            <span>{resultRows.length}</span>
          </div>
          <VirtualList
            className="history-list"
            items={resultRows}
            itemHeight={112}
            maxHeight={360}
            emptyState={<p className="card-copy">{labels.historyEmpty}</p>}
            itemKey={(historyJob) => historyJob.id}
            renderItem={(historyJob) => (
              <button
                className={`history-item${activeJobId === historyJob.id ? " history-item--active" : ""}`}
                onClick={() => onOpenHistoryJob(historyJob.id)}
                type="button"
              >
                <strong>{historyJob.shortId}</strong>
                <span>{historyJob.status}</span>
                <small>
                  {labels.updatedAt}: {historyJob.updatedAt}
                </small>
                <small>
                  {labels.hasResult}: {historyJob.hasResult}
                </small>
              </button>
            )}
          />
          </section>
        </>
      ) : null}

      {libraryTab === "samples" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.sampleLibrary}</h2>
            <button className="link-button" onClick={onRefresh} type="button">
              {labels.refresh}
            </button>
          </div>
          <div className="panel-tabs panel-tabs--wide">
            <button
              className={`panel-tab${samplePage === "catalog" ? " panel-tab--active" : ""}`}
              onClick={() => setSamplePage("catalog")}
              type="button"
            >
              {labels.sampleCatalogPage}
            </button>
            <button
              className={`panel-tab${samplePage === "import" ? " panel-tab--active" : ""}`}
              onClick={() => setSamplePage("import")}
              type="button"
            >
              {labels.sampleImportPage}
            </button>
          </div>
          {samplePage === "catalog" ? (
            <>
              <p className="card-copy">{labels.historyHint}</p>
              <div className="form-grid compact">
                <label>
                  <span>{labels.studyDomain}</span>
                  <div className="button-row">
                    {sampleDomainOptions.map((option) => (
                      <button
                        key={option.key}
                        className={`ghost-button ghost-button--compact${selectedSampleDomain === option.key ? " ghost-button--active" : ""}`}
                        onClick={() => setSelectedSampleDomain(option.key)}
                        type="button"
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                </label>
              </div>
              <div className="history-list sample-group-list">
                {groupedSampleRows.length === 0 ? <p className="card-copy">{labels.noDomainStudies}</p> : null}
                {groupedSampleRows.map((group) => (
                  <div key={group.label} className="sample-group">
                    <div className="sample-group__head">
                      <strong>{group.label}</strong>
                      <span>{group.rows.length}</span>
                    </div>
                    <div className="sample-group__items">
                      {group.rows.map((sample) => (
                        <button key={sample.id} className="history-item" onClick={() => onOpenSample(sample.href)} type="button">
                          <strong>{sample.name}</strong>
                          <span>{sample.kindLabel}</span>
                          <small>{sample.summary}</small>
                        </button>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </>
          ) : null}
          {samplePage === "import" ? (
            <label className="import-box">
              <span>{labels.importModel}</span>
              <small>{labels.importHint}</small>
              <input
                type="file"
                accept=".json,application/json"
                onChange={(event) => onImportModel(event.target.files?.[0])}
              />
            </label>
          ) : null}
        </section>
      ) : null}

      {libraryTab === "projects" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.projectLibrary}</h2>
            <span>{projects.length}</span>
          </div>
          <div className="panel-tabs panel-tabs--wide">
            <button
              className={`panel-tab${projectPage === "manage" ? " panel-tab--active" : ""}`}
              onClick={() => setProjectPage("manage")}
              type="button"
            >
              {labels.projectManagePage}
            </button>
            <button
              className={`panel-tab${projectPage === "exchange" ? " panel-tab--active" : ""}`}
              onClick={() => setProjectPage("exchange")}
              type="button"
            >
              {labels.projectExchangePage}
            </button>
          </div>
          {projectPage === "manage" ? (
            <>
              <div className="form-grid compact">
                <label>
                  <span>{labels.projectNameField}</span>
                  <input value={projectNameDraft} onChange={(event) => onProjectNameDraftChange(event.target.value)} />
                </label>
                <label>
                  <span>{labels.projectDescriptionField}</span>
                  <input value={projectDescriptionDraft} onChange={(event) => onProjectDescriptionDraftChange(event.target.value)} />
                </label>
                <label>
                  <span>{labels.projectLibrary}</span>
                  <select
                    value={selectedProjectId ?? ""}
                    onChange={(event) => onSelectedProjectChange(event.target.value || null)}
                  >
                    <option value="">{labels.none}</option>
                    {projects.map((project) => (
                      <option key={project.project_id} value={project.project_id}>
                        {project.name}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={onCreateProject} type="button">
                  {labels.createProject}
                </button>
                <button className="ghost-button" disabled={!selectedProjectId} onClick={onUpdateProject} type="button">
                  {labels.updateProject}
                </button>
                <button className="ghost-button" disabled={!selectedProjectId} onClick={onDeleteProject} type="button">
                  {labels.deleteProject}
                </button>
              </div>
              {projects.length === 0 ? <p className="card-copy">{labels.projectEmpty}</p> : null}
            </>
          ) : null}
          {projectPage === "exchange" ? (
            <>
              <div className="button-row">
                <button className="ghost-button" disabled={!selectedProjectId} onClick={onExportProjectJson} type="button">
                  {labels.exportProjectJson}
                </button>
                <button className="ghost-button" disabled={!selectedProjectId} onClick={onExportProjectZip} type="button">
                  {labels.exportProjectZip}
                </button>
              </div>
              <label className="import-box">
                <span>{labels.importProject}</span>
                <small>{labels.importProjectHint}</small>
                <input
                  type="file"
                  accept=".kyuubiki,.kyuubiki.json,application/json,application/zip"
                  onChange={(event) => void onImportProjectBundle(event.target.files?.[0])}
                />
              </label>
            </>
          ) : null}
        </section>
      ) : null}

      {libraryTab === "models" ? (
        <>
          <HistoryWorkspaceCard
            title={labels.modelWorkspaceTitle}
            hint={labels.modelWorkspaceHint}
            actionLabel={labels.openLatestModel}
            actionDisabled={!latestModelRow}
            onAction={() => latestModelRow && onOpenSavedModel(latestModelRow.id)}
            metrics={[
              { label: labels.savedCount, value: modelRows.length },
              { label: labels.versionCount, value: modelVersionCount },
            ]}
          />
          <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.savedModels}</h2>
            <span>{selectedProjectModelCount}</span>
          </div>
          <div className="panel-tabs panel-tabs--wide">
            <button
              className={`panel-tab${modelPage === "saved" ? " panel-tab--active" : ""}`}
              onClick={() => setModelPage("saved")}
              type="button"
            >
              {labels.modelSavedPage}
            </button>
            <button
              className={`panel-tab${modelPage === "versions" ? " panel-tab--active" : ""}`}
              onClick={() => setModelPage("versions")}
              type="button"
            >
              {labels.modelVersionsPage}
            </button>
          </div>
          {modelPage === "saved" ? (
            <>
              <div className="form-grid compact">
                <label>
                  <span>{labels.modelName}</span>
                  <input value={loadedModelName} onChange={(event) => onLoadedModelNameChange(event.target.value)} />
                </label>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={() => onSaveModel(false)} type="button">
                  {labels.save}
                </button>
                <button className="ghost-button" onClick={() => onSaveModel(true)} type="button">
                  {labels.saveAs}
                </button>
                <button className="ghost-button" disabled={!selectedModelId} onClick={onDeleteSavedModel} type="button">
                  {labels.deleteSavedModel}
                </button>
              </div>
              <VirtualList
                className="history-list"
                items={modelRows}
                itemHeight={112}
                maxHeight={344}
                emptyState={<p className="card-copy">{labels.noSavedModels}</p>}
                itemKey={(model) => model.id}
                renderItem={(model) => (
                  <button
                    className={`history-item${selectedModelId === model.id ? " history-item--active" : ""}`}
                    onClick={() => onOpenSavedModel(model.id)}
                    type="button"
                  >
                    <strong>{model.name}</strong>
                    <span>{model.kindLabel}</span>
                    <small>
                      {labels.updatedAt}: {model.updatedAt}
                    </small>
                    <small>{model.versionLabel}</small>
                  </button>
                )}
              />
            </>
          ) : null}
          {modelPage === "versions" ? (
            <>
              <div className="card-head">
                <h2>{labels.versions}</h2>
                <span>{modelVersionCount}</span>
              </div>
              <div className="button-row">
                <button className="ghost-button" disabled={!selectedVersionId} onClick={onRenameSelectedVersion} type="button">
                  {labels.renameVersion}
                </button>
                <button className="ghost-button" disabled={!selectedVersionId} onClick={onDeleteSelectedVersion} type="button">
                  {labels.deleteVersion}
                </button>
              </div>
              <VirtualList
                className="history-list"
                items={versionRows}
                itemHeight={100}
                maxHeight={320}
                emptyState={<p className="card-copy">{labels.noVersions}</p>}
                itemKey={(version) => version.id}
                renderItem={(version) => (
                  <button
                    className={`history-item${selectedVersionId === version.id ? " history-item--active" : ""}`}
                    onClick={() => onOpenSavedVersion(version.id)}
                    type="button"
                  >
                    <strong>{version.name}</strong>
                    <span>{version.versionLabel}</span>
                    <small>
                      {labels.updatedAt}: {version.updatedAt}
                    </small>
                  </button>
                )}
              />
            </>
          ) : null}
          </section>
        </>
      ) : null}

    </div>
  );
});
