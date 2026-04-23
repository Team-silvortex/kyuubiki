"use client";

import { memo } from "react";

import { VirtualList } from "@/components/ui/virtual-list";
import type { ProjectRecord } from "@/lib/api";

type LibraryPanelTab = "samples" | "projects" | "models" | "jobs";

type SampleRow = {
  id: string;
  name: string;
  kindLabel: string;
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
  sections: { library: string };
  tabs: { samples: string; projects: string; models: string; jobs: string };
  kinds: Record<string, string>;
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
            items={sampleRows}
            itemHeight={102}
            maxHeight={328}
            itemKey={(sample) => sample.id}
            renderItem={(sample) => (
              <button className="history-item" onClick={() => onOpenSample(sample.href)} type="button">
                <strong>{sample.name}</strong>
                <span>{sample.kindLabel}</span>
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
            <span>{selectedProjectModelCount}</span>
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
        </section>
      ) : null}

      {libraryTab === "models" ? (
        <section className="sidebar-card">
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
        </section>
      ) : null}

      {libraryTab === "jobs" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{labels.sections.library}</h2>
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
      ) : null}
    </div>
  );
});
