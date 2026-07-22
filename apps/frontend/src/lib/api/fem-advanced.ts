import { requestJson } from "./core.ts";
import type { JobEnvelope } from "./fem-shared.ts";

export type AdvancedFemNodeInput = Record<string, string | number | boolean | null | undefined>;
export type AdvancedFemElementInput = Record<string, string | number | boolean | null | undefined>;
export type AdvancedFemContactInput = Record<string, string | number | boolean | null | undefined>;

export type AdvancedFemGraphJobInput = {
  nodes: AdvancedFemNodeInput[];
  elements: AdvancedFemElementInput[];
  project_id?: string;
  model_version_id?: string;
  [key: string]: unknown;
};

export type ContactGap1dJobInput = AdvancedFemGraphJobInput & {
  contacts: AdvancedFemContactInput[];
};

export type BucklingFrame2dJobInput = {
  frame: AdvancedFemGraphJobInput;
  mode_count?: number;
  project_id?: string;
  model_version_id?: string;
  [key: string]: unknown;
};

export type Frame2dPDeltaJobInput = {
  buckling: BucklingFrame2dJobInput;
  imperfection_amplitude: number;
  kinematics?: "linearized_p_delta" | "corotational";
  imperfection_shape?: number[];
  imperfection_mode_index?: number;
  maximum_load_factor?: number;
  load_steps?: number;
  project_id?: string;
  model_version_id?: string;
};

export type AdvancedFemResult = Record<string, unknown>;

export function createAcousticBar1dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/acoustic-bar-1d/jobs", input);
}

export function createStokesFlowPlaneQuad2dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/stokes-flow-plane-quad-2d/jobs", input);
}

export function createStokesFlowPlaneTriangle2dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/stokes-flow-plane-triangle-2d/jobs", input);
}

export function createNonlinearSpring1dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/nonlinear-spring-1d/jobs", input);
}

export function createContactGap1dJob(
  input: ContactGap1dJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/contact-gap-1d/jobs", input);
}

export function createModalFrame2dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/modal-frame-2d/jobs", input);
}

export function createBucklingBeam1dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/buckling-beam-1d/jobs", input);
}

export function createBucklingFrame2dJob(
  input: BucklingFrame2dJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return requestJson<JobEnvelope<AdvancedFemResult>>("/api/v1/fem/buckling-frame-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createFrame2dPDeltaJob(
  input: Frame2dPDeltaJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return requestJson<JobEnvelope<AdvancedFemResult>>("/api/v1/fem/frame-2d-p-delta/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createModalFrame3dJob(
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return createAdvancedFemJob("/api/v1/fem/modal-frame-3d/jobs", input);
}

function createAdvancedFemJob(
  path: string,
  input: AdvancedFemGraphJobInput,
): Promise<JobEnvelope<AdvancedFemResult>> {
  return requestJson<JobEnvelope<AdvancedFemResult>>(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}
