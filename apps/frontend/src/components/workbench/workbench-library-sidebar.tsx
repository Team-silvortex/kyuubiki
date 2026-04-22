"use client";

import { VirtualList } from "@/components/ui/virtual-list";
import type { JobState, ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api";

type LibraryPanelTab = "samples" | "projects" | "models" | "jobs";

type SampleEntry = {
  id: string;
  name: string;
  kind: string;
  href: string;
  summary: string;
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
  sections: { library: string };
  tabs: { samples: string; projects: string; models: string; jobs: string };
  kinds: Record<string, string>;
  none: string;
};

type WorkbenchLibrarySidebarProps = {
  libraryTab: LibraryPanelTab;
  onLibraryTabChange: (tab: LibraryPanelTab) => void;
  labels: LibraryLabels;
  samples: SampleEntry[];
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
  selectedProjectModels: ModelRecord[];
  deferredProjectModels: ModelRecord[];
  selectedModelId: string | null;
  loadedModelName: string;
  onLoadedModelNameChange: (value: string) => void;
  onSaveModel: (saveAs: boolean) => void;
  onDeleteSavedModel: () => void;
  onOpenSavedModel: (model: ModelRecord) => void;
  deferredModelVersions: ModelVersionRecord[];
  modelVersions: ModelVersionRecord[];
  selectedVersionId: string | null;
  onRenameSelectedVersion: () => void;
  onDeleteSelectedVersion: () => void;
  onOpenSavedVersion: (version: ModelVersionRecord) => void;
  deferredJobHistory: JobState[];
  jobHistory: JobState[];
  activeJobId: string | null;
  onOpenHistoryJob: (jobId: string) => void;
  onOpenSample: (href: string) => void;
  onRefresh: () => void;
  onImportModel: (file: File | undefined) => void;
  formatTime: (value: string | undefined) => string;
};

export function WorkbenchLibrarySidebar({
  libraryTab,
  onLibraryTabChange,
  labels,
  samples,
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
  selectedProjectModels,
  deferredProjectModels,
  selectedModelId,
  loadedModelName,
  onLoadedModelNameChange,
  onSaveModel,
  onDeleteSavedModel,
  onOpenSavedModel,
  deferredModelVersions,
  modelVersions,
  selectedVersionId,
  onRenameSelectedVersion,
  onDeleteSelectedVersion,
  onOpenSavedVersion,
  deferredJobHistory,
  jobHistory,
  activeJobId,
  onOpenHistoryJob,
  onOpenSample,
  onRefresh,
  onImportModel,
  formatTime,
}: WorkbenchLibrarySidebarProps) {
  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab panel-tab--icon${libraryTab === "samples" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("samples")} type="button"><span className="panel-tab__glyph">S</span><span>{labels.tabs.samples}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "projects" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("projects")} type="button"><span className="panel-tab__glyph">P</span><span>{labels.tabs.projects}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "models" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("models")} type="button"><span className="panel-tab__glyph">M</span><span>{labels.tabs.models}</span></button>
        <button className={`panel-tab panel-tab--icon${libraryTab === "jobs" ? " panel-tab--active" : ""}`} onClick={() => onLibraryTabChange("jobs")} type="button"><span className="panel-tab__glyph">J</span><span>{labels.tabs.jobs}</span></button>
      </div>

      {libraryTab === "samples" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.sampleLibrary}</h2>
            <button className="link-button" onClick={onRefresh} type="button">
              {labels.refresh}
            </button>
          </div>
          <p className="card-copy">{labels.historyHint}</p>
          <label className="import-box">
            <span>{labels.importModel}</span>
            <small>{labels.importHint}</small>
            <input
              type="file"
              accept=".json,application/json"
              onChange={(event) => onImportModel(event.target.files?.[0])}
            />
          </label>
          <VirtualList
            className="history-list"
            items={samples}
            itemHeight={102}
            maxHeight={328}
            itemKey={(sample) => sample.id}
            renderItem={(sample) => (
              <button className="history-item" onClick={() => onOpenSample(sample.href)} type="button">
                <strong>{sample.name}</strong>
                <span>{labels.kinds[sample.kind] ?? sample.kind}</span>
                <small>{sample.summary}</small>
              </button>
            )}
          />
        </section>
      ) : null}

      {libraryTab === "projects" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.projectLibrary}</h2>
            <span>{projects.length}</span>
          </div>
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
          {projects.length === 0 ? <p className="card-copy">{labels.projectEmpty}</p> : null}
        </section>
      ) : null}

      {libraryTab === "models" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.savedModels}</h2>
            <span>{selectedProjectModels.length}</span>
          </div>
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
            items={deferredProjectModels}
            itemHeight={112}
            maxHeight={344}
            emptyState={<p className="card-copy">{labels.noSavedModels}</p>}
            itemKey={(model) => model.model_id}
            renderItem={(model) => (
              <button
                className={`history-item${selectedModelId === model.model_id ? " history-item--active" : ""}`}
                onClick={() => onOpenSavedModel(model)}
                type="button"
              >
                <strong>{model.name}</strong>
                <span>{labels.kinds[model.kind] ?? model.kind}</span>
                <small>
                  {labels.updatedAt}: {formatTime(model.updated_at)}
                </small>
                <small>v{model.latest_version_number ?? 1}</small>
              </button>
            )}
          />
        </section>
      ) : null}

      {libraryTab === "models" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.versions}</h2>
            <span>{modelVersions.length}</span>
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
            items={deferredModelVersions}
            itemHeight={100}
            maxHeight={320}
            emptyState={<p className="card-copy">{labels.noVersions}</p>}
            itemKey={(version) => version.version_id}
            renderItem={(version) => (
              <button
                className={`history-item${selectedVersionId === version.version_id ? " history-item--active" : ""}`}
                onClick={() => onOpenSavedVersion(version)}
                type="button"
              >
                <strong>{version.name}</strong>
                <span>v{version.version_number}</span>
                <small>
                  {labels.updatedAt}: {formatTime(version.updated_at)}
                </small>
              </button>
            )}
          />
        </section>
      ) : null}

      {libraryTab === "jobs" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.sections.library}</h2>
            <span>{jobHistory.length}</span>
          </div>
          <VirtualList
            className="history-list"
            items={deferredJobHistory}
            itemHeight={112}
            maxHeight={360}
            emptyState={<p className="card-copy">{labels.historyEmpty}</p>}
            itemKey={(historyJob) => historyJob.job_id}
            renderItem={(historyJob) => (
              <button
                className={`history-item${activeJobId === historyJob.job_id ? " history-item--active" : ""}`}
                onClick={() => onOpenHistoryJob(historyJob.job_id)}
                type="button"
              >
                <strong>{historyJob.job_id.slice(0, 8)}</strong>
                <span>{historyJob.status}</span>
                <small>
                  {labels.updatedAt}: {formatTime(historyJob.updated_at)}
                </small>
                <small>
                  {labels.hasResult}: {historyJob.has_result ? labels.yes : labels.no}
                </small>
              </button>
            )}
          />
        </section>
      ) : null}
    </div>
  );
}
