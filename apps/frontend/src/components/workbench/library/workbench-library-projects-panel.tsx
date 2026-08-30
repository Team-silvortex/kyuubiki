"use client";

import type {
  ProjectPage,
  WorkbenchLibrarySidebarProps,
} from "@/components/workbench/library/workbench-library-sidebar-types";

type WorkbenchLibraryProjectsPanelProps = Pick<
  WorkbenchLibrarySidebarProps,
  | "labels"
  | "onCreateProject"
  | "onDeleteProject"
  | "onExportProjectJson"
  | "onExportProjectZip"
  | "onImportProjectBundle"
  | "onProjectDescriptionDraftChange"
  | "onProjectNameDraftChange"
  | "onSelectedProjectChange"
  | "onUpdateProject"
  | "projectDescriptionDraft"
  | "projectNameDraft"
  | "projects"
  | "selectedProjectId"
> & {
  projectPage: ProjectPage;
  setProjectPage: (page: ProjectPage) => void;
};

export function WorkbenchLibraryProjectsPanel({
  labels,
  onCreateProject,
  onDeleteProject,
  onExportProjectJson,
  onExportProjectZip,
  onImportProjectBundle,
  onProjectDescriptionDraftChange,
  onProjectNameDraftChange,
  onSelectedProjectChange,
  onUpdateProject,
  projectDescriptionDraft,
  projectNameDraft,
  projectPage,
  projects,
  selectedProjectId,
  setProjectPage,
}: WorkbenchLibraryProjectsPanelProps) {
  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{labels.projectLibrary}</h2>
        <span>{projects.length}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button
          className={`panel-tab${projectPage === "manage" ? " panel-tab--active" : ""}`}
          data-workbench-library-project-page="manage"
          onClick={() => setProjectPage("manage")}
          type="button"
        >
          {labels.projectManagePage}
        </button>
        <button
          className={`panel-tab${projectPage === "exchange" ? " panel-tab--active" : ""}`}
          data-workbench-library-project-page="exchange"
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
              <input
                value={projectDescriptionDraft}
                onChange={(event) => onProjectDescriptionDraftChange(event.target.value)}
              />
            </label>
            <label>
              <span>{labels.projectLibrary}</span>
              <select value={selectedProjectId ?? ""} onChange={(event) => onSelectedProjectChange(event.target.value || null)}>
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
      ) : (
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
      )}
    </section>
  );
}
