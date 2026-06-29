import { requestJson } from "./core";
import type { JobEnvelope } from "./fem-shared";

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
