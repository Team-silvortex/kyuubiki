import { randomUUID } from "node:crypto";
import net from "node:net";

type RpcMethod =
  | "ping"
  | "describe_agent"
  | "solve_bar_1d"
  | "solve_truss_2d"
  | "solve_truss_3d"
  | "solve_plane_triangle_2d";

type RpcFrame =
  | {
      rpc_version: number;
      id: string;
      event: string;
      progress: Record<string, unknown>;
    }
  | {
      rpc_version: number;
      id: string;
      ok: boolean;
      result?: unknown;
      error?: { code: string; message: string };
    };

export type DirectMeshAgentSummary = {
  id: string;
  host: string;
  port: number;
  role: string;
  descriptor?: Record<string, unknown>;
  descriptor_error?: string;
};

export type DirectMeshSelectionMode = "first_reachable" | "healthiest";

export type DirectMeshSolveEnvelope = {
  endpoint: string;
  strategy: DirectMeshSelectionMode;
  result: unknown;
  progress_frames: Array<Record<string, unknown>>;
};

const DEFAULT_TIMEOUT_MS = 15_000;

function normalizeEndpoints(input: string[]): string[] {
  return [...new Set(input.map((value) => value.trim()).filter(Boolean))];
}

function endpointsFromEnv() {
  return normalizeEndpoints(
    (process.env.KYUUBIKI_DIRECT_MESH_ENDPOINTS ?? process.env.KYUUBIKI_AGENT_ENDPOINTS ?? "")
      .split(","),
  );
}

export function resolveDirectMeshEndpoints(input?: string[]) {
  const normalized = normalizeEndpoints(input ?? []);
  return normalized.length > 0 ? normalized : endpointsFromEnv();
}

export function normalizeDirectMeshEndpoints(input: string[]) {
  return normalizeEndpoints(input);
}

function descriptorHealthScore(agent: DirectMeshAgentSummary) {
  const score = (agent.descriptor as { runtime?: { health_score?: number } } | undefined)?.runtime?.health_score;
  return typeof score === "number" ? score : 0;
}

function splitEndpoint(endpoint: string) {
  const [host, portValue] = endpoint.split(":");
  const port = Number(portValue);
  if (!host || Number.isNaN(port) || port <= 0) {
    throw new Error(`invalid direct mesh endpoint: ${endpoint}`);
  }
  return { host, port };
}

async function requestRpcFrameSequence(
  endpoint: string,
  method: RpcMethod,
  params: Record<string, unknown>,
): Promise<{ response: Extract<RpcFrame, { ok: boolean }>; progressFrames: Array<Record<string, unknown>> }> {
  const { host, port } = splitEndpoint(endpoint);

  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ host, port });
    const requestId = randomUUID();
    let buffer = Buffer.alloc(0);
    let settled = false;
    const progressFrames: Array<Record<string, unknown>> = [];

    const fail = (error: Error) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      reject(error);
    };

    const succeed = (response: Extract<RpcFrame, { ok: boolean }>) => {
      if (settled) return;
      settled = true;
      socket.end();
      resolve({ response, progressFrames });
    };

    socket.setTimeout(DEFAULT_TIMEOUT_MS, () => {
      fail(new Error(`timed out talking to direct mesh agent ${endpoint}`));
    });

    socket.on("connect", () => {
      const payload = Buffer.from(
        JSON.stringify({
          rpc_version: 1,
          id: requestId,
          method,
          params,
        }),
        "utf8",
      );
      const header = Buffer.allocUnsafe(4);
      header.writeUInt32BE(payload.length, 0);
      socket.write(Buffer.concat([header, payload]));
    });

    socket.on("data", (chunk) => {
      buffer = Buffer.concat([buffer, Buffer.from(chunk)]);

      while (buffer.length >= 4) {
        const frameLength = buffer.readUInt32BE(0);
        if (buffer.length < 4 + frameLength) break;

        const payload = buffer.subarray(4, 4 + frameLength);
        buffer = buffer.subarray(4 + frameLength);

        let frame: RpcFrame;
        try {
          frame = JSON.parse(payload.toString("utf8")) as RpcFrame;
        } catch (error) {
          fail(error instanceof Error ? error : new Error("invalid JSON frame"));
          return;
        }

        if ("event" in frame) {
          progressFrames.push(frame as Record<string, unknown>);
          continue;
        }

        if (frame.ok) {
          succeed(frame);
        } else {
          fail(new Error(frame.error?.message ?? "direct mesh rpc failed"));
        }
        return;
      }
    });

    socket.on("error", (error) => fail(error));
    socket.on("close", () => {
      if (!settled) {
        fail(new Error(`direct mesh agent ${endpoint} closed the connection`));
      }
    });
  });
}

export async function describeDirectMeshAgents(input?: string[]) {
  const endpoints = resolveDirectMeshEndpoints(input);

  const agents = await Promise.all(
    endpoints.map(async (endpoint): Promise<DirectMeshAgentSummary> => {
      const { host, port } = splitEndpoint(endpoint);

      try {
        const { response } = await requestRpcFrameSequence(endpoint, "describe_agent", {});
        const descriptor = (response.result ?? {}) as Record<string, unknown>;
        return {
          id:
            typeof descriptor.program === "string"
              ? `${descriptor.program}@${endpoint}`
              : `direct-agent@${endpoint}`,
          host,
          port,
          role: "solver",
          descriptor,
        };
      } catch (error) {
        return {
          id: `direct-agent@${endpoint}`,
          host,
          port,
          role: "solver",
          descriptor_error: error instanceof Error ? error.message : "failed to describe agent",
        };
      }
    }),
  );

  return { endpoints, agents };
}

function sortAgentsForSelection(
  agents: DirectMeshAgentSummary[],
  mode: DirectMeshSelectionMode,
) {
  if (mode === "healthiest") {
    return [...agents].sort((left, right) => descriptorHealthScore(right) - descriptorHealthScore(left));
  }

  return agents;
}

export async function solveViaDirectMesh(
  method: Exclude<RpcMethod, "ping" | "describe_agent">,
  params: Record<string, unknown>,
  input?: string[],
  selectionMode: DirectMeshSelectionMode = "healthiest",
): Promise<DirectMeshSolveEnvelope> {
  const { agents } = await describeDirectMeshAgents(input);
  const orderedAgents = sortAgentsForSelection(
    agents.filter((agent) => !agent.descriptor_error),
    selectionMode,
  );
  const endpoints = orderedAgents.length > 0
    ? orderedAgents.map((agent) => `${agent.host}:${agent.port}`)
    : resolveDirectMeshEndpoints(input);
  let lastError: Error | null = null;

  for (const endpoint of endpoints) {
    try {
      const { response, progressFrames } = await requestRpcFrameSequence(endpoint, method, params);
      return {
        endpoint,
        strategy: selectionMode,
        result: response.result,
        progress_frames: progressFrames,
      };
    } catch (error) {
      lastError = error instanceof Error ? error : new Error("direct mesh solve failed");
    }
  }

  throw lastError ?? new Error("no reachable direct mesh agents");
}
