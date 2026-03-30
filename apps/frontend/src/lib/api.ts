export type AxialBarJobInput = {
  length: number;
  area: number;
  elements: number;
  tip_force: number;
  youngs_modulus_gpa: number;
};

export type AxialBarJobPayload = {
  job: {
    job_id: string;
    status: string;
    worker_id: string;
    progress: number;
  };
  result: {
    tip_displacement: number;
    reaction_force: number;
    max_displacement: number;
    max_stress: number;
    nodes: Array<{ index: number; x: number; displacement: number }>;
    elements: Array<{
      index: number;
      x1: number;
      x2: number;
      strain: number;
      stress: number;
      axial_force: number;
    }>;
    input: {
      length: number;
      area: number;
      elements: number;
      tip_force: number;
      youngs_modulus: number;
    };
  };
};

export type HealthPayload = {
  service: string;
  status: string;
  transport?: {
    http: number;
    solver_agent_tcp: number;
  };
};

export async function runAxialBarJob(
  input: AxialBarJobInput
): Promise<AxialBarJobPayload> {
  const response = await fetch("/api/v1/fem/axial-bar/jobs", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(input),
  });

  const payload = (await response.json()) as AxialBarJobPayload & { error?: string };

  if (!response.ok) {
    throw new Error(payload.error ?? "analysis failed");
  }

  return payload;
}

export async function fetchHealth(): Promise<HealthPayload> {
  const response = await fetch("/api/health", {
    method: "GET",
    cache: "no-store",
  });

  const payload = (await response.json()) as HealthPayload & { error?: string };

  if (!response.ok) {
    throw new Error(payload.error ?? "backend unavailable");
  }

  return payload;
}
