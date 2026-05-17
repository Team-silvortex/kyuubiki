import type {
  JobEnvelope,
  JobState,
  ModelRecord,
  ModelVersionRecord,
  ProtocolAgentDescriptor,
  ResultRecord,
  Truss3dElementInput,
  Truss3dNodeInput,
} from "@/lib/api";

export type SampleLibraryEntry = {
  id: string;
  name: string;
  kind: string;
  href: string;
  summary: string;
};

export type StudyFamilyKey = "axialAndSprings" | "beamsAndFrames" | "trusses" | "planes";

export function classifyStudyKindFamily(kind: string): StudyFamilyKey {
  switch (kind) {
    case "axial_bar_1d":
    case "thermal_bar_1d":
    case "spring_1d":
    case "spring_2d":
    case "spring_3d":
      return "axialAndSprings";
    case "thermal_beam_1d":
    case "beam_1d":
    case "torsion_1d":
    case "frame_2d":
      return "beamsAndFrames";
    case "truss_2d":
    case "truss_3d":
    case "thermal_truss_2d":
    case "thermal_truss_3d":
      return "trusses";
    case "plane_triangle_2d":
    case "plane_quad_2d":
    default:
      return "planes";
  }
}

export function buildLibrarySampleRows({
  samples,
  kindLabel,
  familyLabel,
}: {
  samples: SampleLibraryEntry[];
  kindLabel: (kind: string) => string;
  familyLabel: (family: StudyFamilyKey) => string;
}) {
  return samples.map((sample) => ({
    id: sample.id,
    name: sample.name,
    kindLabel: kindLabel(sample.kind),
    familyKey: classifyStudyKindFamily(sample.kind),
    familyLabel: familyLabel(classifyStudyKindFamily(sample.kind)),
    href: sample.href,
    summary: sample.summary,
  }));
}

export function buildLibraryModelRows({
  models,
  kindLabel,
  updatedAtLabel,
}: {
  models: ModelRecord[];
  kindLabel: (kind: string) => string;
  updatedAtLabel: (value: string) => string;
}) {
  return models.map((model) => ({
    id: model.model_id,
    name: model.name,
    kindLabel: kindLabel(model.kind),
    updatedAt: updatedAtLabel(model.updated_at),
    versionLabel: `v${model.latest_version_number ?? 1}`,
  }));
}

export function buildLibraryVersionRows({
  versions,
  updatedAtLabel,
}: {
  versions: ModelVersionRecord[];
  updatedAtLabel: (value: string) => string;
}) {
  return versions.map((version) => ({
    id: version.version_id,
    name: version.name,
    versionLabel: `v${version.version_number}`,
    updatedAt: updatedAtLabel(version.updated_at),
  }));
}

export function buildLibraryJobRows({
  jobs,
  updatedAtLabel,
  hasResultLabel,
}: {
  jobs: JobState[];
  updatedAtLabel: (value: string | undefined) => string;
  hasResultLabel: (hasResult: boolean) => string;
}) {
  return jobs.map((job) => ({
    id: job.job_id,
    shortId: job.job_id.slice(0, 8),
    status: job.status,
    updatedAt: updatedAtLabel(job.updated_at),
    hasResult: hasResultLabel(Boolean(job.has_result)),
  }));
}

export function buildAdminJobRows({
  jobs,
  heartbeatTone,
  heartbeatLabel,
  detailLabel,
}: {
  jobs: JobState[];
  heartbeatTone: (job: JobEnvelope["job"]) => string;
  heartbeatLabel: (job: JobEnvelope["job"]) => string;
  detailLabel: (job: JobState) => string;
}) {
  return jobs.map((job) => ({
    id: job.job_id,
    status: job.status,
    projectId: job.project_id ?? null,
    heartbeatTone: heartbeatTone(job as JobEnvelope["job"]),
    heartbeatLabel: heartbeatLabel(job as JobEnvelope["job"]),
    detail: detailLabel(job),
  }));
}

export function buildAdminResultRows({
  records,
  jobs,
  updatedAtLabel,
  summaryLabel,
}: {
  records: ResultRecord[];
  jobs: JobState[];
  updatedAtLabel: (record: ResultRecord) => string;
  summaryLabel: (record: ResultRecord) => string;
}) {
  const jobLookup = new Map(jobs.map((job) => [job.job_id, job]));

  return records.map((record) => ({
    id: record.job_id,
    updatedAt: updatedAtLabel(record),
    projectId: jobLookup.get(record.job_id)?.project_id?.slice(0, 8) ?? null,
    modelVersionId: jobLookup.get(record.job_id)?.model_version_id?.slice(0, 8) ?? null,
    status: jobLookup.get(record.job_id)?.status ?? null,
    summary: summaryLabel(record),
  }));
}

export function buildProtocolAgentCards({
  agents,
  labels,
  clusterHealthTone,
  peerStatusLabel,
}: {
  agents: ProtocolAgentDescriptor[];
  labels: {
    runtimeMode: string;
    cluster: string;
    clusterSize: string;
    clusterHealth: string;
    peers: string;
    headless: string;
    yes: string;
    no: string;
    capabilities: string;
    methods: string;
    peerState: string;
  };
  clusterHealthTone: (score: number | null | undefined) => string;
  peerStatusLabel: (status: string | undefined) => string;
}) {
  return agents.slice(0, 4).map((agent) => ({
    id: agent.id,
    endpoint: `${agent.host}:${agent.port}`,
    metrics: [
      { label: labels.runtimeMode, value: agent.descriptor?.runtime?.runtime_mode ?? "--" },
      { label: labels.cluster, value: agent.descriptor?.runtime?.cluster_id ?? "--" },
      { label: labels.clusterSize, value: agent.descriptor?.runtime?.cluster_size ?? 1 },
      {
        label: labels.clusterHealth,
        value: agent.descriptor?.runtime?.health_score ?? "--",
        tone: clusterHealthTone(agent.descriptor?.runtime?.health_score),
      },
      { label: labels.peers, value: agent.descriptor?.runtime?.peers?.length ?? 0 },
      { label: labels.headless, value: agent.descriptor?.runtime?.headless ? labels.yes : labels.no },
      { label: labels.capabilities, value: agent.descriptor?.capabilities?.length ?? 0 },
      { label: labels.methods, value: agent.descriptor?.protocol?.methods?.length ?? 0 },
    ],
    chips: [
      ...(agent.descriptor?.capabilities?.flatMap((capability) =>
        capability.tags.slice(0, 3).map((tag) => ({
          key: `${agent.id}-${capability.id}-${tag}`,
          label: tag,
        })),
      ) ?? []),
      ...(agent.descriptor?.runtime?.peers?.slice(0, 2).map((peer) => ({
        key: `${agent.id}-${peer.address}`,
        label: peer.address,
        tone: clusterHealthTone(
          peer.status === "healthy" ? 100 : peer.status === "degraded" ? 65 : peer.status === "seed" ? 85 : 25,
        ),
        title: `${labels.peerState}: ${peerStatusLabel(peer.status)}`,
      })) ?? []),
    ],
    error: agent.descriptor_error,
  }));
}

type StudyKind =
  | "axial_bar_1d"
  | "thermal_bar_1d"
  | "thermal_beam_1d"
  | "thermal_truss_2d"
  | "thermal_truss_3d"
  | "spring_1d"
  | "spring_2d"
  | "spring_3d"
  | "beam_1d"
  | "torsion_1d"
  | "truss_2d"
  | "truss_3d"
  | "plane_triangle_2d"
  | "plane_quad_2d"
  | "frame_2d";

export type StudyKindOptionGroup = {
  label: string;
  options: Array<{ value: StudyKind; label: string }>;
};

export function buildStudyKindOptionGroups({
  kinds,
  families,
}: {
  kinds: {
    axial_bar_1d: string;
    thermal_bar_1d: string;
    thermal_beam_1d: string;
    thermal_truss_2d: string;
    thermal_truss_3d: string;
    spring_1d: string;
    spring_2d: string;
    spring_3d: string;
    beam_1d: string;
    torsion_1d: string;
    truss_2d: string;
    truss_3d: string;
    plane_triangle_2d: string;
    plane_quad_2d: string;
    frame_2d: string;
  };
  families: {
    axialAndSprings: string;
    beamsAndFrames: string;
    trusses: string;
    planes: string;
  };
}) {
  return [
    {
      label: families.axialAndSprings,
      options: [
        { value: "axial_bar_1d" as const, label: kinds.axial_bar_1d },
        { value: "thermal_bar_1d" as const, label: kinds.thermal_bar_1d },
        { value: "spring_1d" as const, label: kinds.spring_1d },
        { value: "spring_2d" as const, label: kinds.spring_2d },
        { value: "spring_3d" as const, label: kinds.spring_3d },
      ],
    },
    {
      label: families.beamsAndFrames,
      options: [
        { value: "beam_1d" as const, label: kinds.beam_1d },
        { value: "thermal_beam_1d" as const, label: kinds.thermal_beam_1d },
        { value: "torsion_1d" as const, label: kinds.torsion_1d },
        { value: "frame_2d" as const, label: kinds.frame_2d },
      ],
    },
    {
      label: families.trusses,
      options: [
        { value: "truss_2d" as const, label: kinds.truss_2d },
        { value: "truss_3d" as const, label: kinds.truss_3d },
        { value: "thermal_truss_2d" as const, label: kinds.thermal_truss_2d },
        { value: "thermal_truss_3d" as const, label: kinds.thermal_truss_3d },
      ],
    },
    {
      label: families.planes,
      options: [
        { value: "plane_triangle_2d" as const, label: kinds.plane_triangle_2d },
        { value: "plane_quad_2d" as const, label: kinds.plane_quad_2d },
      ],
    },
  ] satisfies StudyKindOptionGroup[];
}

export function buildStudyKindOptions(kinds: {
  axial_bar_1d: string;
  thermal_bar_1d: string;
  thermal_beam_1d: string;
  thermal_truss_2d: string;
  thermal_truss_3d: string;
  spring_1d: string;
  spring_2d: string;
  spring_3d: string;
  beam_1d: string;
  torsion_1d: string;
  truss_2d: string;
  truss_3d: string;
  plane_triangle_2d: string;
  plane_quad_2d: string;
  frame_2d: string;
}) {
  return [
    { value: "axial_bar_1d" as const, label: kinds.axial_bar_1d },
    { value: "thermal_bar_1d" as const, label: kinds.thermal_bar_1d },
    { value: "thermal_truss_2d" as const, label: kinds.thermal_truss_2d },
    { value: "spring_1d" as const, label: kinds.spring_1d },
    { value: "spring_2d" as const, label: kinds.spring_2d },
    { value: "spring_3d" as const, label: kinds.spring_3d },
    { value: "beam_1d" as const, label: kinds.beam_1d },
    { value: "thermal_beam_1d" as const, label: kinds.thermal_beam_1d },
    { value: "torsion_1d" as const, label: kinds.torsion_1d },
    { value: "truss_2d" as const, label: kinds.truss_2d },
    { value: "truss_3d" as const, label: kinds.truss_3d },
    { value: "thermal_truss_3d" as const, label: kinds.thermal_truss_3d },
    { value: "plane_triangle_2d" as const, label: kinds.plane_triangle_2d },
    { value: "plane_quad_2d" as const, label: kinds.plane_quad_2d },
    { value: "frame_2d" as const, label: kinds.frame_2d },
  ];
}

export function buildStudySummaryRows({
  labels,
  loadedModelName,
  materialLabel,
  meshValue,
  loadValue,
  supportValue,
}: {
  labels: { modelName: string; material: string; mesh: string; load: string; support: string };
  loadedModelName: string;
  materialLabel: string;
  meshValue: string | number;
  loadValue: string | number;
  supportValue: string;
}) {
  return [
    { label: labels.modelName, value: loadedModelName },
    { label: labels.material, value: materialLabel },
    { label: labels.mesh, value: meshValue },
    { label: labels.load, value: loadValue },
    { label: labels.support, value: supportValue },
  ];
}

export function buildStudyControlsRows({
  labels,
  studyKind,
  loadedModelName,
  materialLabel,
  trussNodeCount,
  trussElementCount,
  truss3dNodeCount,
  truss3dElementCount,
  truss3dLoadValue,
  planeNodeCount,
  planeElementCount,
  planeThicknessValue,
}: {
  labels: {
    nodes: string;
    trussElements: string;
    material: string;
    sourceModel: string;
    spatialTrussElements: string;
    load: string;
    planeElements: string;
    frameElements: string;
    thickness: string;
  };
  studyKind: "axial_bar_1d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_truss_2d" | "thermal_truss_3d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";
  loadedModelName: string;
  materialLabel: string;
  trussNodeCount: number;
  trussElementCount: number;
  truss3dNodeCount: number;
  truss3dElementCount: number;
  truss3dLoadValue: string;
  planeNodeCount: number;
  planeElementCount: number;
  planeThicknessValue: string;
}) {
  if (studyKind === "axial_bar_1d") {
    return [];
  }

  if (studyKind === "thermal_bar_1d") {
    return [
      { label: labels.nodes, value: planeNodeCount },
      { label: labels.frameElements, value: planeElementCount },
      { label: labels.material, value: materialLabel },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "thermal_truss_2d") {
    return [
      { label: labels.nodes, value: trussNodeCount },
      { label: labels.trussElements, value: trussElementCount },
      { label: labels.material, value: materialLabel },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "truss_2d") {
    return [
      { label: labels.nodes, value: trussNodeCount },
      { label: labels.trussElements, value: trussElementCount },
      { label: labels.material, value: materialLabel },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "truss_3d") {
    return [
      { label: labels.nodes, value: truss3dNodeCount },
      { label: labels.spatialTrussElements, value: truss3dElementCount },
      { label: labels.load, value: truss3dLoadValue },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "thermal_truss_3d") {
    return [
      { label: labels.nodes, value: truss3dNodeCount },
      { label: labels.spatialTrussElements, value: truss3dElementCount },
      { label: labels.load, value: truss3dLoadValue },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "frame_2d") {
    return [
      { label: labels.nodes, value: trussNodeCount },
      { label: labels.frameElements, value: trussElementCount },
      { label: labels.material, value: materialLabel },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "spring_1d") {
    return [
      { label: labels.nodes, value: planeNodeCount },
      { label: labels.frameElements, value: planeElementCount },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "spring_2d") {
    return [
      { label: labels.nodes, value: planeNodeCount },
      { label: labels.frameElements, value: planeElementCount },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "spring_3d") {
    return [
      { label: labels.nodes, value: truss3dNodeCount },
      { label: labels.spatialTrussElements, value: truss3dElementCount },
      { label: labels.load, value: truss3dLoadValue },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "beam_1d") {
    return [
      { label: labels.nodes, value: planeNodeCount },
      { label: labels.frameElements, value: planeElementCount },
      { label: labels.material, value: materialLabel },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "thermal_beam_1d") {
    return [
      { label: labels.nodes, value: planeNodeCount },
      { label: labels.frameElements, value: planeElementCount },
      { label: labels.material, value: materialLabel },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  if (studyKind === "torsion_1d") {
    return [
      { label: labels.nodes, value: planeNodeCount },
      { label: labels.frameElements, value: planeElementCount },
      { label: labels.sourceModel, value: loadedModelName },
    ];
  }

  return [
    { label: labels.nodes, value: planeNodeCount },
    { label: labels.planeElements, value: planeElementCount },
    { label: labels.thickness, value: planeThicknessValue },
    { label: labels.sourceModel, value: loadedModelName },
  ];
}

export function buildTruss3dTreeRows({
  nodes,
  elements,
  selectedNode,
  selectedTruss3dNodes,
  memberDraftNodes,
  fixed,
}: {
  nodes: Truss3dNodeInput[];
  elements: Array<{ id: string; node_i: number; node_j: number }>;
  selectedNode: number | null;
  selectedTruss3dNodes: number[];
  memberDraftNodes: number[];
  fixed: (value: number, decimals?: number) => string;
}) {
  return {
    nodes: nodes.map((node, index) => ({
      index,
      id: node.id,
      x: fixed(node.x, 2),
      y: fixed(node.y, 2),
      z: fixed(node.z, 2),
      active: selectedTruss3dNodes.includes(index) || selectedNode === index,
      draft: memberDraftNodes.includes(index),
    })),
    elements: elements.map((element, index) => ({
      index,
      id: element.id,
      nodeI: element.node_i,
      nodeJ: element.node_j,
    })),
  };
}
