import { lineResultFieldValue, planeResultFieldValue } from "@/components/workbench/workbench-result-helpers";
import { scientific } from "@/lib/workbench/helpers";

type PlaneElement = {
  index: number;
  id: string;
  node_i?: number;
  node_j?: number;
  node_k?: number;
  node_l?: number;
  average_temperature?: number;
  average_temperature_delta?: number;
  temperature_gradient_x?: number;
  temperature_gradient_y?: number;
  heat_flux_x?: number;
  heat_flux_y?: number;
  heat_flux_magnitude?: number;
  stress_x?: number;
  stress_y?: number;
  load_y?: number;
};

type LineElement = {
  index: number;
  id: string;
  node_i?: number;
  node_j?: number;
  average_temperature_delta?: number;
  temperature_gradient_y?: number;
  thermal_curvature?: number;
  axial_force_i?: number;
  axial_force_j?: number;
  shear_force_i?: number;
  shear_force_j?: number;
  moment_i?: number;
  moment_j?: number;
  axial_stress?: number;
  fix_x?: boolean;
  fix_y?: boolean;
};

type PointNode = { id: string; x: number; y: number; load_x?: number; load_y?: number; fix_x?: boolean; fix_y?: boolean };
type SpatialNode = PointNode & { index: number; z: number; load_z?: number; fix_z?: boolean };

type FrameNode = {
  id: string;
  x: number;
  y?: number;
  load_y?: number;
  moment_z?: number;
  fix_y?: boolean;
  fix_rz?: boolean;
  temperature_delta?: number;
  load_x?: number;
  fix_x?: boolean;
};

type FrameElement = {
  index?: number;
  id?: string;
  node_i?: number;
  node_j?: number;
  area?: number;
  youngs_modulus?: number;
  moment_of_inertia?: number;
  section_modulus?: number;
  conductivity?: number;
  shear_modulus?: number;
  polar_moment?: number;
};

export function buildWorkbenchHotspotData({
  isHeatPlane,
  isThermalFrame,
  isSpring,
  isThermalBar,
  isThermalTruss2d,
  isThermalTruss3d,
  isTorsion,
  isBeam,
  planeElements,
  planeResultField,
  displayTrussElements,
  activeLineResultField,
  selectedElement,
  planeHotspotLimit,
}: {
  isHeatPlane: boolean;
  isThermalFrame: boolean;
  isSpring: boolean;
  isThermalBar: boolean;
  isThermalTruss2d: boolean;
  isThermalTruss3d: boolean;
  isTorsion: boolean;
  isBeam: boolean;
  planeElements: PlaneElement[];
  planeResultField: string;
  displayTrussElements: LineElement[];
  activeLineResultField: string;
  selectedElement: number | null;
  planeHotspotLimit: number;
}) {
  const planeHotspotElements = planeElements
    .map((element) => ({
      index: element.index,
      id: element.id,
      value: planeResultFieldValue(element, planeResultField as never),
      active: selectedElement === element.index,
      summary: isHeatPlane
        ? `Tavg ${scientific(element.average_temperature)} · ∇Ty ${scientific(element.temperature_gradient_y)} · |q| ${scientific(element.heat_flux_magnitude)}`
        : undefined,
    }))
    .sort((left, right) => right.value - left.value)
    .slice(0, planeHotspotLimit)
    .map((element) => ({
      index: element.index,
      id: element.id,
      value: scientific(element.value),
      active: element.active,
      summary: element.summary,
    }));

  const planeThermalRows = isHeatPlane
    ? planeElements.map((element) => ({
        id: element.id,
        index: element.index,
        active: selectedElement === element.index,
        sortTemperature: Math.abs(element.average_temperature ?? 0),
        sortGradient: Math.max(Math.abs(element.temperature_gradient_y ?? 0), Math.abs(element.temperature_gradient_x ?? 0)),
        sortFlux: Math.abs(element.heat_flux_magnitude ?? 0),
        averageTemperature: scientific(element.average_temperature),
        temperatureGradientX: scientific(element.temperature_gradient_x),
        temperatureGradientY: scientific(element.temperature_gradient_y),
        heatFluxX: scientific(element.heat_flux_x),
        heatFluxY: scientific(element.heat_flux_y),
        heatFluxMagnitude: scientific(element.heat_flux_magnitude),
      }))
    : [];

  const frameHotspotElements = displayTrussElements
    .map((element) => ({
      index: element.index,
      id: element.id,
      value: lineResultFieldValue(element, activeLineResultField as never),
      active: selectedElement === element.index,
      summary: isThermalFrame
        ? `ΔT ${scientific(element.average_temperature_delta)} · ∇T ${scientific(element.temperature_gradient_y)} · κth ${scientific(element.thermal_curvature)}`
        : isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
          ? `Fi ${scientific(element.axial_force_i)} · Fj ${scientific(element.axial_force_j)}`
          : isTorsion
            ? `Ti ${scientific(element.moment_i)} · Tj ${scientific(element.moment_j)}`
            : isBeam
              ? `Vi ${scientific(element.shear_force_i)} · Mi ${scientific(element.moment_i)} · Vj ${scientific(element.shear_force_j)} · Mj ${scientific(element.moment_j)}`
              : `Ai ${scientific(element.axial_force_i)} · Vi ${scientific(element.shear_force_i)} · Mi ${scientific(element.moment_i)} · Aj ${scientific(element.axial_force_j)} · Vj ${scientific(element.shear_force_j)} · Mj ${scientific(element.moment_j)}`,
    }))
    .sort((left, right) => right.value - left.value)
    .slice(0, planeHotspotLimit)
    .map((element) => ({
      index: element.index,
      id: element.id,
      value: scientific(element.value),
      active: element.active,
      summary: element.summary,
    }));

  const frameForceRows = displayTrussElements.map((element) => ({
    id: element.id,
    index: element.index,
    active: selectedElement === element.index,
    sortAxial: Math.max(Math.abs(element.axial_force_i ?? 0), Math.abs(element.axial_force_j ?? 0)),
    sortShear: Math.max(Math.abs(element.shear_force_i ?? 0), Math.abs(element.shear_force_j ?? 0)),
    sortMoment: Math.max(Math.abs(element.moment_i ?? 0), Math.abs(element.moment_j ?? 0)),
    axialForceI: scientific(element.axial_force_i),
    shearForceI: scientific(element.shear_force_i),
    momentI: scientific(element.moment_i),
    axialForceJ: scientific(element.axial_force_j),
    shearForceJ: scientific(element.shear_force_j),
    momentJ: scientific(element.moment_j),
  }));

  const frameMaxAxialForce = Math.max(
    ...displayTrussElements.map((element) => Math.max(Math.abs(element.axial_force_i ?? 0), Math.abs(element.axial_force_j ?? 0))),
    0,
  );
  const frameMaxShearForce = Math.max(
    ...displayTrussElements.map((element) => Math.max(Math.abs(element.shear_force_i ?? 0), Math.abs(element.shear_force_j ?? 0))),
    0,
  );

  return {
    planeHotspotElements,
    planeThermalRows,
    frameHotspotElements,
    frameForceRows,
    frameMaxAxialForce,
    frameMaxShearForce,
  };
}

export function buildWorkbenchSelectionData({
  selectedNode,
  selectedElement,
  displayTrussNodes,
  displayTrussElements,
  displayTruss3dNodes,
  displayTruss3dElements,
  planeNodes,
  planeElements,
  isThermalFrame,
  thermalFrameModel,
  thermalFrameResult,
  frameModel,
  frameResult,
  isThermalBeam,
  thermalBeamModel,
  thermalBeamResult,
  beamModel,
  beamResult,
  torsionModel,
  torsionResult,
  isHeatBar,
  heatBarModel,
  heatBarResult,
  thermalBarModel,
  thermalBarResult,
  activeSpringModel,
}: {
  selectedNode: number | null;
  selectedElement: number | null;
  displayTrussNodes: PointNode[];
  displayTrussElements: LineElement[];
  displayTruss3dNodes: SpatialNode[];
  displayTruss3dElements: Array<LineElement & { material_id?: string }>;
  planeNodes: PointNode[];
  planeElements: PlaneElement[];
  isThermalFrame: boolean;
  thermalFrameModel: { nodes: FrameNode[]; elements: FrameElement[] };
  thermalFrameResult: { nodes?: Array<{ displacement_magnitude?: number; rz?: number }>; elements?: Array<Record<string, unknown>>; max_temperature_delta?: number; max_temperature_gradient?: number } | null;
  frameModel: { nodes: FrameNode[]; elements: FrameElement[] };
  frameResult: { nodes?: Array<{ displacement_magnitude?: number; rz?: number }>; elements?: Array<Record<string, unknown>> } | null;
  isThermalBeam: boolean;
  thermalBeamModel: { nodes: FrameNode[]; elements: FrameElement[] };
  thermalBeamResult: { nodes?: Array<{ displacement_magnitude?: number; rz?: number }>; elements?: Array<Record<string, unknown>>; max_temperature_gradient?: number } | null;
  beamModel: { nodes: FrameNode[]; elements: FrameElement[] };
  beamResult: { nodes?: Array<{ displacement_magnitude?: number; rz?: number }>; elements?: Array<Record<string, unknown>> } | null;
  torsionModel: { nodes: Array<{ id: string; x: number; torque_z: number; fix_rz: boolean }>; elements: Array<{ id: string; node_i: number; node_j: number; shear_modulus: number; polar_moment: number; section_modulus: number }> };
  torsionResult: { nodes?: Array<{ rz?: number }>; elements?: Array<{ shear_stress?: number; torque?: number }> } | null;
  isHeatBar: boolean;
  heatBarModel: { nodes: Array<{ id: string; x: number; heat_load?: number; temperature?: number; fix_temperature?: boolean }>; elements: Array<{ area: number; conductivity: number }> };
  heatBarResult: { nodes?: Array<{ temperature?: number }>; elements?: Array<{ average_temperature?: number; temperature_gradient?: number; heat_flux?: number }> } | null;
  thermalBarModel: { nodes: Array<{ id: string; x: number; load_x: number; fix_x: boolean }>; elements: Array<{ area: number; youngs_modulus: number }> };
  thermalBarResult: { nodes?: Array<{ ux?: number }> } | null;
  activeSpringModel: { elements: Array<Record<string, unknown>> };
}) {
  const selectedNodeData =
    selectedNode !== null && displayTrussNodes[selectedNode]
      ? {
          ...displayTrussNodes[selectedNode],
          load_x: displayTrussNodes[selectedNode].load_x ?? 0,
          load_y: displayTrussNodes[selectedNode].load_y ?? 0,
          fix_x: displayTrussNodes[selectedNode].fix_x ?? false,
          fix_y: displayTrussNodes[selectedNode].fix_y ?? false,
        }
      : null;
  const selectedElementData =
    selectedElement !== null && displayTrussElements[selectedElement]
      ? {
          ...displayTrussElements[selectedElement],
          node_i: displayTrussElements[selectedElement].node_i ?? 0,
          node_j: displayTrussElements[selectedElement].node_j ?? 0,
        }
      : null;
  const selectedTruss3dNodeData =
    selectedNode !== null && displayTruss3dNodes[selectedNode]
      ? {
          ...displayTruss3dNodes[selectedNode],
          load_x: displayTruss3dNodes[selectedNode].load_x ?? 0,
          load_y: displayTruss3dNodes[selectedNode].load_y ?? 0,
          load_z: displayTruss3dNodes[selectedNode].load_z ?? 0,
          fix_x: displayTruss3dNodes[selectedNode].fix_x ?? false,
          fix_y: displayTruss3dNodes[selectedNode].fix_y ?? false,
          fix_z: displayTruss3dNodes[selectedNode].fix_z ?? false,
        }
      : null;
  const selectedTruss3dElementData =
    selectedElement !== null && displayTruss3dElements[selectedElement]
      ? {
          ...displayTruss3dElements[selectedElement],
          node_i: displayTruss3dElements[selectedElement].node_i ?? 0,
          node_j: displayTruss3dElements[selectedElement].node_j ?? 0,
        }
      : null;
  const selectedPlaneNodeData =
    selectedNode !== null && planeNodes[selectedNode]
      ? {
          ...planeNodes[selectedNode],
          load_x: planeNodes[selectedNode].load_x ?? 0,
          load_y: planeNodes[selectedNode].load_y ?? 0,
          fix_x: planeNodes[selectedNode].fix_x ?? false,
          fix_y: planeNodes[selectedNode].fix_y ?? false,
        }
      : null;
  const selectedPlaneElementData =
    selectedElement !== null && planeElements[selectedElement]
      ? {
          ...planeElements[selectedElement],
          node_i: planeElements[selectedElement].node_i ?? 0,
          node_j: planeElements[selectedElement].node_j ?? 0,
          node_k: planeElements[selectedElement].node_k ?? 0,
        }
      : null;
  const selectedFrameNodeData =
    selectedNode !== null && (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode])
      ? {
          ...(isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]),
          y: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).y ?? 0,
          load_x: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).load_x ?? 0,
          load_y: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).load_y ?? 0,
          moment_z: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).moment_z ?? 0,
          fix_x: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).fix_x ?? false,
          fix_y: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).fix_y ?? false,
          fix_rz: (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]).fix_rz ?? false,
          displacement_magnitude: isThermalFrame ? thermalFrameResult?.nodes?.[selectedNode]?.displacement_magnitude : frameResult?.nodes?.[selectedNode]?.displacement_magnitude,
          rz: isThermalFrame ? thermalFrameResult?.nodes?.[selectedNode]?.rz : frameResult?.nodes?.[selectedNode]?.rz,
        }
      : null;
  const selectedFrameElementData =
    selectedElement !== null && (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement])
      ? {
          ...(isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]),
          ...(isThermalFrame ? thermalFrameResult?.elements?.[selectedElement] : frameResult?.elements?.[selectedElement]),
          index: selectedElement,
          id: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).id ?? `frame-${selectedElement}`,
          node_i: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).node_i ?? 0,
          node_j: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).node_j ?? 0,
          area: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).area ?? 0,
          youngs_modulus: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).youngs_modulus ?? 0,
          moment_of_inertia: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).moment_of_inertia ?? 0,
          section_modulus: (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]).section_modulus ?? 0,
        }
      : null;
  const selectedBeamNodeData =
    selectedNode !== null && (isThermalBeam ? thermalBeamModel.nodes[selectedNode] : beamModel.nodes[selectedNode])
      ? {
          ...(isThermalBeam ? thermalBeamModel.nodes[selectedNode] : beamModel.nodes[selectedNode]),
          displacement_magnitude: isThermalBeam ? thermalBeamResult?.nodes?.[selectedNode]?.displacement_magnitude : beamResult?.nodes?.[selectedNode]?.displacement_magnitude,
          rz: isThermalBeam ? thermalBeamResult?.nodes?.[selectedNode]?.rz : beamResult?.nodes?.[selectedNode]?.rz,
          y: 0,
          load_x: 0,
          load_y: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.load_y : beamModel.nodes[selectedNode]?.load_y) ?? 0,
          fix_x: false,
          fix_y: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.fix_y : beamModel.nodes[selectedNode]?.fix_y) ?? false,
          moment_z: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.moment_z : beamModel.nodes[selectedNode]?.moment_z) ?? 0,
          fix_rz: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.fix_rz : beamModel.nodes[selectedNode]?.fix_rz) ?? false,
        }
      : null;
  const selectedBeamElementData =
    selectedElement !== null && (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement])
      ? {
          ...(isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]),
          index: selectedElement,
          id: (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]).id ?? `beam-${selectedElement}`,
          node_i: (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]).node_i ?? 0,
          node_j: (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]).node_j ?? 0,
          youngs_modulus: (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]).youngs_modulus ?? 0,
          moment_of_inertia: (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]).moment_of_inertia ?? 0,
          section_modulus: (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]).section_modulus ?? 0,
          area: 0,
          ...(isThermalBeam ? thermalBeamResult?.elements?.[selectedElement] : beamResult?.elements?.[selectedElement]),
        }
      : null;
  const selectedTorsionNodeData =
    selectedNode !== null && torsionModel.nodes[selectedNode]
      ? {
          id: torsionModel.nodes[selectedNode].id,
          x: torsionModel.nodes[selectedNode].x,
          y: 0,
          load_x: 0,
          load_y: 0,
          moment_z: torsionModel.nodes[selectedNode].torque_z,
          fix_x: false,
          fix_y: true,
          fix_rz: torsionModel.nodes[selectedNode].fix_rz,
          displacement_magnitude: Math.abs(torsionResult?.nodes?.[selectedNode]?.rz ?? 0),
          rz: torsionResult?.nodes?.[selectedNode]?.rz,
        }
      : null;
  const selectedTorsionElementData =
    selectedElement !== null && torsionModel.elements[selectedElement]
      ? {
          index: selectedElement,
          id: torsionModel.elements[selectedElement].id,
          node_i: torsionModel.elements[selectedElement].node_i,
          node_j: torsionModel.elements[selectedElement].node_j,
          area: 0,
          youngs_modulus: torsionModel.elements[selectedElement].shear_modulus,
          moment_of_inertia: torsionModel.elements[selectedElement].polar_moment,
          section_modulus: torsionModel.elements[selectedElement].section_modulus,
          axial_stress: undefined,
          max_bending_stress: torsionResult?.elements?.[selectedElement]?.shear_stress,
          max_combined_stress: undefined,
          axial_force_i: 0,
          shear_force_i: 0,
          moment_i: torsionResult?.elements?.[selectedElement]?.torque,
          axial_force_j: 0,
          shear_force_j: 0,
          moment_j: torsionResult?.elements?.[selectedElement]?.torque,
        }
      : null;
  const selectedThermalNodeData =
    selectedNode !== null && (isHeatBar ? heatBarModel.nodes[selectedNode] : thermalBarModel.nodes[selectedNode])
      ? {
          id: isHeatBar ? heatBarModel.nodes[selectedNode].id : thermalBarModel.nodes[selectedNode].id,
          x: isHeatBar ? heatBarModel.nodes[selectedNode].x : thermalBarModel.nodes[selectedNode].x,
          y: 0,
          load_x: isHeatBar ? (heatBarModel.nodes[selectedNode].heat_load ?? 0) : thermalBarModel.nodes[selectedNode].load_x,
          load_y: 0,
          fix_x: isHeatBar ? (heatBarModel.nodes[selectedNode].fix_temperature ?? false) : thermalBarModel.nodes[selectedNode].fix_x,
          fix_y: true,
          moment_z: 0,
          fix_rz: true,
          displacement_magnitude: Math.abs(isHeatBar ? heatBarResult?.nodes?.[selectedNode]?.temperature ?? 0 : thermalBarResult?.nodes?.[selectedNode]?.ux ?? 0),
          rz: 0,
          temperature: isHeatBar ? heatBarModel.nodes[selectedNode].temperature ?? 0 : undefined,
          heat_load: isHeatBar ? heatBarModel.nodes[selectedNode].heat_load ?? 0 : undefined,
          fix_temperature: isHeatBar ? heatBarModel.nodes[selectedNode].fix_temperature ?? false : undefined,
        }
      : null;
  const selectedThermalElementData =
    selectedElement !== null && (isHeatBar ? heatBarModel.elements[selectedElement] : thermalBarModel.elements[selectedElement])
      ? {
          index: selectedElement,
          ...(isHeatBar ? heatBarModel.elements[selectedElement] : thermalBarModel.elements[selectedElement]),
          id: `thermal-${selectedElement}`,
          node_i: selectedElement,
          node_j: selectedElement + 1,
          area: isHeatBar ? heatBarModel.elements[selectedElement].area : thermalBarModel.elements[selectedElement].area,
          youngs_modulus: isHeatBar ? heatBarModel.elements[selectedElement].conductivity : thermalBarModel.elements[selectedElement].youngs_modulus,
          moment_of_inertia: 0,
          section_modulus: 0,
          axial_stress: displayTrussElements[selectedElement]?.axial_stress,
          average_temperature: isHeatBar ? heatBarResult?.elements?.[selectedElement]?.average_temperature ?? 0 : undefined,
          temperature_gradient_x: isHeatBar ? heatBarResult?.elements?.[selectedElement]?.temperature_gradient ?? 0 : undefined,
          heat_flux_x: isHeatBar ? heatBarResult?.elements?.[selectedElement]?.heat_flux ?? 0 : undefined,
          heat_flux_y: isHeatBar ? 0 : undefined,
          heat_flux_magnitude: isHeatBar ? Math.abs(heatBarResult?.elements?.[selectedElement]?.heat_flux ?? 0) : undefined,
          axial_force_i: displayTrussElements[selectedElement]?.axial_force_i,
          shear_force_i: 0,
          moment_i: 0,
          axial_force_j: displayTrussElements[selectedElement]?.axial_force_j,
          shear_force_j: 0,
          moment_j: 0,
        }
      : null;
  const selectedSpringElementData =
    selectedElement !== null && activeSpringModel.elements[selectedElement]
      ? {
          index: selectedElement,
          ...activeSpringModel.elements[selectedElement],
          id: String((activeSpringModel.elements[selectedElement] as { id?: string }).id ?? `spring-${selectedElement}`),
          node_i: (activeSpringModel.elements[selectedElement] as { node_i?: number }).node_i ?? 0,
          node_j: (activeSpringModel.elements[selectedElement] as { node_j?: number }).node_j ?? 0,
          area: 0,
          youngs_modulus: 0,
          moment_of_inertia: 0,
          section_modulus: 0,
          axial_stress: displayTrussElements[selectedElement]?.axial_stress,
          axial_force_i: displayTrussElements[selectedElement]?.axial_force_i,
          shear_force_i: 0,
          moment_i: 0,
          axial_force_j: displayTrussElements[selectedElement]?.axial_force_j,
          shear_force_j: 0,
          moment_j: 0,
        }
      : null;

  return {
    selectedNodeData,
    selectedElementData,
    selectedTruss3dNodeData,
    selectedTruss3dElementData,
    selectedPlaneNodeData,
    selectedPlaneElementData,
    selectedFrameNodeData,
    selectedFrameElementData,
    selectedBeamNodeData,
    selectedBeamElementData,
    selectedTorsionNodeData,
    selectedTorsionElementData,
    selectedThermalNodeData,
    selectedThermalElementData,
    selectedSpringElementData,
  };
}

export function buildWorkbenchSecurityUi(language: string) {
  return language === "zh"
    ? {
        controlPlaneToken: "控制面 API Token",
        clusterToken: "集群 API Token",
        directMeshToken: "直连网格 Token",
        protectReads: "保护只读接口",
        clusterWindow: "集群时间窗",
        directMeshRoutes: "直连网格路由",
        security: "安全",
        enabled: "已启用",
        disabled: "未启用",
        configured: "已配置",
        notConfigured: "未配置",
        mutatingRoutes: "写入路由保护",
        clusterRoutes: "集群路由保护",
      }
    : {
        controlPlaneToken: "Control-plane API token",
        clusterToken: "Cluster API token",
        directMeshToken: "Direct mesh token",
        protectReads: "Protect reads",
        clusterWindow: "Cluster time window",
        directMeshRoutes: "Direct mesh routes",
        security: "Security",
        enabled: "Enabled",
        disabled: "Disabled",
        configured: "Configured",
        notConfigured: "Not configured",
        mutatingRoutes: "Mutating routes",
        clusterRoutes: "Cluster routes",
      };
}
