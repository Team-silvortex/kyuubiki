"use client";

import type { JobState, ProjectRecord, ResultRecord } from "@/lib/api";

type AdminContextLabels = {
  noJobVersion: string;
  noResultVersion: string;
  noRecordContext: string;
  linkedProjectMissing: string;
  linkedProjectOpened: string;
  noJobProject: string;
  noResultProject: string;
  selectJobFirst: string;
  missingResultJob: string;
  recordContextApplied: string;
};

type BuildAdminContextEffectsArgs = {
  selectedAdminJob: JobState | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  jobHistory: JobState[];
  projects: ProjectRecord[];
  refreshVersions: (modelId: string) => Promise<void>;
  openModelVersionById: (versionId: string) => void;
  setAdminFilterProjectId: (value: string) => void;
  setAdminFilterModelVersionId: (value: string) => void;
  setAdminJobCaseId: (value: string) => void;
  setLibraryTab: (value: any) => void;
  setSelectedProjectId: (value: string | null) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  setSidebarSection: (value: any) => void;
  setMessage: (value: string) => void;
  labels: AdminContextLabels;
};

type BuildAdminSelectionArgs = {
  jobHistory: JobState[];
  resultRecords: ResultRecord[];
  projects: ProjectRecord[];
  selectedProjectId: string | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
};

export function buildWorkbenchAdminSelections({
  jobHistory,
  resultRecords,
  projects,
  selectedProjectId,
  selectedAdminJobId,
  selectedAdminResultJobId,
}: BuildAdminSelectionArgs) {
  const selectedProject = projects.find((project) => project.project_id === selectedProjectId) ?? null;
  const selectedProjectModels = selectedProject?.models ?? [];
  const selectedAdminJob = jobHistory.find((entry) => entry.job_id === selectedAdminJobId) ?? null;
  const selectedAdminResult = resultRecords.find((entry) => entry.job_id === selectedAdminResultJobId) ?? null;

  return {
    selectedProject,
    selectedProjectModels,
    selectedAdminJob,
    selectedAdminResult,
  };
}

export function buildWorkbenchAdminDataEffects(args: BuildAdminContextEffectsArgs) {
  return {
    selectedAdminJob: args.selectedAdminJob,
    selectedAdminJobId: args.selectedAdminJobId,
    selectedAdminResultJobId: args.selectedAdminResultJobId,
    jobHistory: args.jobHistory,
    projects: args.projects,
    refreshVersions: args.refreshVersions,
    openModelVersionById: args.openModelVersionById,
    setAdminFilterProjectId: args.setAdminFilterProjectId,
    setAdminFilterModelVersionId: args.setAdminFilterModelVersionId,
    setAdminJobCaseId: args.setAdminJobCaseId,
    setLibraryTab: args.setLibraryTab,
    setSelectedProjectId: args.setSelectedProjectId,
    setSelectedModelId: args.setSelectedModelId,
    setSelectedVersionId: args.setSelectedVersionId,
    setModelVersions: args.setModelVersions,
    setSidebarSection: args.setSidebarSection,
    setMessage: args.setMessage,
    labels: args.labels,
  };
}

export function buildWorkbenchStudyFlags(studyKind: string) {
  const isAxial = studyKind === "axial_bar_1d";
  const isHeatBar = studyKind === "heat_bar_1d";
  const isElectrostaticPlaneTriangle = studyKind === "electrostatic_plane_triangle_2d";
  const isElectrostaticPlaneQuad = studyKind === "electrostatic_plane_quad_2d";
  const isHeatPlaneTriangle = studyKind === "heat_plane_triangle_2d";
  const isHeatPlaneQuad = studyKind === "heat_plane_quad_2d";
  const isHeatPlane = isHeatPlaneTriangle || isHeatPlaneQuad;
  const isElectrostaticPlane = isElectrostaticPlaneTriangle || isElectrostaticPlaneQuad;
  const isThermalBar = studyKind === "thermal_bar_1d";
  const isThermalBeam = studyKind === "thermal_beam_1d";
  const isThermalFrame = studyKind === "thermal_frame_2d";
  const isThermalTruss2d = studyKind === "thermal_truss_2d";
  const isThermalTruss3d = studyKind === "thermal_truss_3d";
  const isThermalPlaneTriangle = studyKind === "thermal_plane_triangle_2d";
  const isThermalPlaneQuad = studyKind === "thermal_plane_quad_2d";
  const isThermal = isThermalBar || isThermalTruss2d || isThermalTruss3d;
  const isSpring1d = studyKind === "spring_1d";
  const isSpring2d = studyKind === "spring_2d";
  const isSpring3d = studyKind === "spring_3d";
  const isSpring = isSpring1d || isSpring2d || isSpring3d;
  const isBeam = studyKind === "beam_1d" || isThermalBeam;
  const isTorsion = studyKind === "torsion_1d";
  const isTruss = studyKind === "truss_2d" || isThermalTruss2d;
  const isTruss3d = studyKind === "truss_3d" || isThermalTruss3d || isSpring3d;
  const isFrame = studyKind === "frame_2d";
  const isFrameLike = isFrame || isThermalFrame;
  const isPlane =
    isHeatPlane ||
    isElectrostaticPlane ||
    studyKind === "plane_triangle_2d" ||
    studyKind === "plane_quad_2d" ||
    isThermalPlaneTriangle ||
    isThermalPlaneQuad;

  return {
    isAxial,
    isHeatBar,
    isElectrostaticPlaneTriangle,
    isElectrostaticPlaneQuad,
    isElectrostaticPlane,
    isHeatPlaneTriangle,
    isHeatPlaneQuad,
    isHeatPlane,
    isThermalBar,
    isThermalBeam,
    isThermalFrame,
    isThermalTruss2d,
    isThermalTruss3d,
    isThermalPlaneTriangle,
    isThermalPlaneQuad,
    isThermal,
    isSpring1d,
    isSpring2d,
    isSpring3d,
    isSpring,
    isBeam,
    isTorsion,
    isTruss,
    isTruss3d,
    isFrame,
    isFrameLike,
    isPlane,
  };
}
