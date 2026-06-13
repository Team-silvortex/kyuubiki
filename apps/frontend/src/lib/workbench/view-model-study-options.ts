import type { StudyDomainKey, StudyFamilyKey } from "@/lib/workbench/view-models";

export type StudyKind =
  | "axial_bar_1d"
  | "heat_bar_1d"
  | "electrostatic_plane_triangle_2d"
  | "electrostatic_plane_quad_2d"
  | "heat_plane_triangle_2d"
  | "heat_plane_quad_2d"
  | "thermal_bar_1d"
  | "thermal_beam_1d"
  | "thermal_frame_2d"
  | "thermal_truss_2d"
  | "thermal_truss_3d"
  | "thermal_plane_triangle_2d"
  | "thermal_plane_quad_2d"
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
  domainKey: StudyDomainKey;
  label: string;
  options: Array<{ value: StudyKind; label: string }>;
};

export type StudyDomainOption = {
  key: StudyDomainKey;
  label: string;
};

export function buildStudyKindOptionGroups({
  kinds,
  domains: _domains,
  families,
}: {
  kinds: Record<StudyKind, string>;
  domains: {
    mechanical: string;
    thermal: string;
    thermoMechanical: string;
  };
  families: Record<StudyFamilyKey, string>;
}) {
  return [
    {
      domainKey: "mechanical",
      label: families.axialAndSprings,
      options: [
        { value: "axial_bar_1d" as const, label: kinds.axial_bar_1d },
        { value: "spring_1d" as const, label: kinds.spring_1d },
        { value: "spring_2d" as const, label: kinds.spring_2d },
        { value: "spring_3d" as const, label: kinds.spring_3d },
      ],
    },
    {
      domainKey: "mechanical",
      label: families.beamsAndFrames,
      options: [
        { value: "beam_1d" as const, label: kinds.beam_1d },
        { value: "torsion_1d" as const, label: kinds.torsion_1d },
        { value: "frame_2d" as const, label: kinds.frame_2d },
      ],
    },
    {
      domainKey: "mechanical",
      label: families.trusses,
      options: [
        { value: "truss_2d" as const, label: kinds.truss_2d },
        { value: "truss_3d" as const, label: kinds.truss_3d },
      ],
    },
    {
      domainKey: "mechanical",
      label: families.planes,
      options: [
        { value: "plane_triangle_2d" as const, label: kinds.plane_triangle_2d },
        { value: "plane_quad_2d" as const, label: kinds.plane_quad_2d },
      ],
    },
    {
      domainKey: "thermoMechanical",
      label: families.axialAndSprings,
      options: [{ value: "thermal_bar_1d" as const, label: kinds.thermal_bar_1d }],
    },
    {
      domainKey: "thermoMechanical",
      label: families.beamsAndFrames,
      options: [
        { value: "thermal_beam_1d" as const, label: kinds.thermal_beam_1d },
        { value: "thermal_frame_2d" as const, label: kinds.thermal_frame_2d },
      ],
    },
    {
      domainKey: "thermoMechanical",
      label: families.trusses,
      options: [
        { value: "thermal_truss_2d" as const, label: kinds.thermal_truss_2d },
        { value: "thermal_truss_3d" as const, label: kinds.thermal_truss_3d },
      ],
    },
    {
      domainKey: "thermoMechanical",
      label: families.planes,
      options: [
        { value: "thermal_plane_triangle_2d" as const, label: kinds.thermal_plane_triangle_2d },
        { value: "thermal_plane_quad_2d" as const, label: kinds.thermal_plane_quad_2d },
      ],
    },
    {
      domainKey: "thermal",
      label: families.axialAndSprings,
      options: [{ value: "heat_bar_1d" as const, label: kinds.heat_bar_1d }],
    },
    {
      domainKey: "thermal",
      label: families.planes,
      options: [
        { value: "electrostatic_plane_triangle_2d" as const, label: kinds.electrostatic_plane_triangle_2d },
        { value: "electrostatic_plane_quad_2d" as const, label: kinds.electrostatic_plane_quad_2d },
      ],
    },
    {
      domainKey: "thermal",
      label: families.planes,
      options: [
        { value: "heat_plane_triangle_2d" as const, label: kinds.heat_plane_triangle_2d },
        { value: "heat_plane_quad_2d" as const, label: kinds.heat_plane_quad_2d },
      ],
    },
  ] satisfies StudyKindOptionGroup[];
}

export function buildStudyDomainOptions(domains: {
  mechanical: string;
  thermal: string;
  thermoMechanical: string;
}) {
  return [
    { key: "mechanical" as const, label: domains.mechanical },
    { key: "thermal" as const, label: domains.thermal },
    { key: "thermoMechanical" as const, label: domains.thermoMechanical },
  ] satisfies StudyDomainOption[];
}

export function buildStudyKindOptions(kinds: Record<StudyKind, string>) {
  return [
    { value: "axial_bar_1d" as const, label: kinds.axial_bar_1d },
    { value: "heat_bar_1d" as const, label: kinds.heat_bar_1d },
    { value: "electrostatic_plane_triangle_2d" as const, label: kinds.electrostatic_plane_triangle_2d },
    { value: "electrostatic_plane_quad_2d" as const, label: kinds.electrostatic_plane_quad_2d },
    { value: "heat_plane_triangle_2d" as const, label: kinds.heat_plane_triangle_2d },
    { value: "heat_plane_quad_2d" as const, label: kinds.heat_plane_quad_2d },
    { value: "thermal_bar_1d" as const, label: kinds.thermal_bar_1d },
    { value: "thermal_truss_2d" as const, label: kinds.thermal_truss_2d },
    { value: "spring_1d" as const, label: kinds.spring_1d },
    { value: "spring_2d" as const, label: kinds.spring_2d },
    { value: "spring_3d" as const, label: kinds.spring_3d },
    { value: "beam_1d" as const, label: kinds.beam_1d },
    { value: "thermal_beam_1d" as const, label: kinds.thermal_beam_1d },
    { value: "thermal_frame_2d" as const, label: kinds.thermal_frame_2d },
    { value: "torsion_1d" as const, label: kinds.torsion_1d },
    { value: "truss_2d" as const, label: kinds.truss_2d },
    { value: "truss_3d" as const, label: kinds.truss_3d },
    { value: "thermal_truss_3d" as const, label: kinds.thermal_truss_3d },
    { value: "plane_triangle_2d" as const, label: kinds.plane_triangle_2d },
    { value: "thermal_plane_triangle_2d" as const, label: kinds.thermal_plane_triangle_2d },
    { value: "plane_quad_2d" as const, label: kinds.plane_quad_2d },
    { value: "thermal_plane_quad_2d" as const, label: kinds.thermal_plane_quad_2d },
    { value: "frame_2d" as const, label: kinds.frame_2d },
  ];
}
