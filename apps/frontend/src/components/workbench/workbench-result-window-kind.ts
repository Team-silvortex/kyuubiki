export type ResultWindowStudyKind =
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

export type ResultWindowGuards = {
  isAxialResult: (value: unknown) => boolean;
  isTrussResult: (value: unknown) => boolean;
  isHeatBar1dResult: (value: unknown) => boolean;
  isElectrostaticPlaneQuad2dResult: (value: unknown) => boolean;
  isElectrostaticPlaneTriangle2dResult: (value: unknown) => boolean;
  isHeatPlaneQuad2dResult: (value: unknown) => boolean;
  isHeatPlaneTriangle2dResult: (value: unknown) => boolean;
  isThermalBar1dResult: (value: unknown) => boolean;
  isThermalBeam1dResult: (value: unknown) => boolean;
  isThermalFrame2dResult: (value: unknown) => boolean;
  isThermalTruss2dResult: (value: unknown) => boolean;
  isThermalTruss3dResult: (value: unknown) => boolean;
  isTruss3dResult: (value: unknown) => boolean;
  isSpring1dResult: (value: unknown) => boolean;
  isSpring2dResult: (value: unknown) => boolean;
  isSpring3dResult: (value: unknown) => boolean;
  isBeam1dResult: (value: unknown) => boolean;
  isTorsion1dResult: (value: unknown) => boolean;
  isFrame2dResult: (value: unknown) => boolean;
};

export function resolveResultWindowStudyKind(
  result: unknown,
  studyKind: ResultWindowStudyKind,
  guards: ResultWindowGuards,
): Exclude<ResultWindowStudyKind, "axial_bar_1d"> {
  if (studyKind !== "axial_bar_1d") return studyKind;

  return guards.isHeatBar1dResult(result)
      ? "heat_bar_1d"
      : guards.isElectrostaticPlaneQuad2dResult(result)
        ? "electrostatic_plane_quad_2d"
        : guards.isElectrostaticPlaneTriangle2dResult(result)
          ? "electrostatic_plane_triangle_2d"
          : guards.isHeatPlaneQuad2dResult(result)
            ? "heat_plane_quad_2d"
            : guards.isHeatPlaneTriangle2dResult(result)
              ? "heat_plane_triangle_2d"
              : guards.isThermalBar1dResult(result)
                ? "thermal_bar_1d"
                : guards.isThermalFrame2dResult(result)
                  ? "thermal_frame_2d"
                  : guards.isThermalBeam1dResult(result)
                    ? "thermal_beam_1d"
                    : guards.isThermalTruss3dResult(result)
                      ? "thermal_truss_3d"
                      : guards.isThermalTruss2dResult(result)
                        ? "thermal_truss_2d"
                        : guards.isSpring3dResult(result)
                          ? "spring_3d"
                          : guards.isSpring2dResult(result)
                            ? "spring_2d"
                            : guards.isSpring1dResult(result)
                              ? "spring_1d"
                              : guards.isTorsion1dResult(result)
                                ? "torsion_1d"
                                : guards.isBeam1dResult(result)
                                  ? "beam_1d"
                                  : guards.isFrame2dResult(result)
                                    ? "frame_2d"
                                    : guards.isTruss3dResult(result)
                                      ? "truss_3d"
                                      : guards.isTrussResult(result)
                                        ? "truss_2d"
                                        : "plane_triangle_2d";
}
