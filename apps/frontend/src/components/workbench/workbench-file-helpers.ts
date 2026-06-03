import type {
  AxialBarResult,
  Beam1dResult,
  Frame2dResult,
  HeatBar1dResult,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dResult,
  JobEnvelope,
  PlaneQuad2dResult,
  PlaneTriangle2dResult,
  Spring1dResult,
  Spring2dResult,
  Spring3dResult,
  ThermalBar1dResult,
  ThermalBeam1dResult,
  ThermalFrame2dResult,
  ThermalPlaneQuad2dResult,
  ThermalPlaneTriangle2dResult,
  ThermalTruss2dResult,
  ThermalTruss3dResult,
  Torsion1dResult,
  Truss2dResult,
  Truss3dResult,
} from "@/lib/api";
import type { Dispatch, SetStateAction } from "react";

export function downloadTextFile(filename: string, contents: string) {
  const blob = new Blob([contents], { type: "application/json;charset=utf-8" });
  downloadBlobFile(filename, blob);
}

export function downloadBlobFile(filename: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

export function resetActiveResult(
  setResult: Dispatch<
    SetStateAction<
      | AxialBarResult
      | HeatBar1dResult
      | HeatPlaneTriangle2dResult
      | HeatPlaneQuad2dResult
      | ThermalBar1dResult
      | ThermalBeam1dResult
      | ThermalFrame2dResult
      | ThermalTruss2dResult
      | ThermalTruss3dResult
      | ThermalPlaneTriangle2dResult
      | ThermalPlaneQuad2dResult
      | Spring1dResult
      | Spring2dResult
      | Spring3dResult
      | Beam1dResult
      | Torsion1dResult
      | Truss2dResult
      | Truss3dResult
      | PlaneTriangle2dResult
      | PlaneQuad2dResult
      | Frame2dResult
      | null
    >
  >,
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>,
) {
  setResult(null);
  setJob(null);
}
