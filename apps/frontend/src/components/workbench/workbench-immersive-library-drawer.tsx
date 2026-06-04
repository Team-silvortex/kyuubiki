"use client";

import type { ModelRecord } from "@/lib/api";
import type { SidebarSection } from "@/components/workbench/workbench-types";

type CopyShape = {
  immersiveDrawer: string;
  immersiveLibrary: string;
  close: string;
  immersiveSamples: string;
  immersiveModels: string;
  immersiveJobs: string;
  immersiveEmptyModels: string;
  immersiveEmptyJobs: string;
  updatedAt: string;
};

type SampleRow = { href: string; name: string; kindLabel: string };
type ModelRow = { id: string; name: string; updatedAt: string };
type JobRow = { id: string; shortId: string; status: string };
type WorkbenchImmersiveLibraryDrawerProps = {
  t: CopyShape;
  immersiveViewport: boolean;
  sidebarSection: SidebarSection;
  librarySampleRows: SampleRow[];
  libraryModelRows: ModelRow[];
  libraryJobRows: JobRow[];
  selectedProjectModels: ModelRecord[];
  handleSidebarSectionChange: (section: SidebarSection) => void;
  openSample: (href: string) => void;
  openSavedModel: (modelRecord: ModelRecord) => void;
  openHistoryJob: (jobId: string) => void;
};

export function WorkbenchImmersiveLibraryDrawer({
  t,
  immersiveViewport,
  sidebarSection,
  librarySampleRows,
  libraryModelRows,
  libraryJobRows,
  selectedProjectModels,
  handleSidebarSectionChange,
  openSample,
  openSavedModel,
  openHistoryJob,
}: WorkbenchImmersiveLibraryDrawerProps) {
  if (!(immersiveViewport && sidebarSection === "library")) return null;

  return (
    <div className="immersive-drawer">
      <section className="immersive-drawer__card">
        <div className="card-head">
          <h2>{t.immersiveDrawer}</h2>
          <div className="immersive-drawer__head-actions">
            <span>{t.immersiveLibrary}</span>
            <button
              className="ghost-button ghost-button--compact"
              onClick={() => handleSidebarSectionChange("model")}
              type="button"
            >
              {t.close}
            </button>
          </div>
        </div>
        <div className="immersive-drawer__grid">
          <div className="immersive-drawer__section">
            <h3>{t.immersiveSamples}</h3>
            <div className="immersive-drawer__list">
              {librarySampleRows.slice(0, 4).map((sample) => (
                <button
                  key={sample.href}
                  className="history-item immersive-drawer__button"
                  onClick={() => openSample(sample.href)}
                  type="button"
                >
                  <strong>{sample.name}</strong>
                  <span>{sample.kindLabel}</span>
                </button>
              ))}
            </div>
          </div>
          <div className="immersive-drawer__section">
            <h3>{t.immersiveModels}</h3>
            <div className="immersive-drawer__list">
              {libraryModelRows.slice(0, 4).length > 0 ? (
                libraryModelRows.slice(0, 4).map((model) => (
                  <button
                    key={model.id}
                    className="history-item immersive-drawer__button"
                    onClick={() => {
                      const modelRecord = selectedProjectModels.find((entry) => entry.model_id === model.id);
                      if (modelRecord) openSavedModel(modelRecord);
                    }}
                    type="button"
                  >
                    <strong>{model.name}</strong>
                    <small>{t.updatedAt}: {model.updatedAt}</small>
                  </button>
                ))
              ) : (
                <p className="card-copy">{t.immersiveEmptyModels}</p>
              )}
            </div>
          </div>
          <div className="immersive-drawer__section">
            <h3>{t.immersiveJobs}</h3>
            <div className="immersive-drawer__list">
              {libraryJobRows.slice(0, 4).length > 0 ? (
                libraryJobRows.slice(0, 4).map((historyJob) => (
                  <button
                    key={historyJob.id}
                    className="history-item immersive-drawer__button"
                    onClick={() => openHistoryJob(historyJob.id)}
                    type="button"
                  >
                    <strong>{historyJob.shortId}</strong>
                    <span>{historyJob.status}</span>
                  </button>
                ))
              ) : (
                <p className="card-copy">{t.immersiveEmptyJobs}</p>
              )}
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
