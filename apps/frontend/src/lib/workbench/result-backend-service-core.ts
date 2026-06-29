"use client";

import type { ResultChunkKind, ResultChunkPayload } from "@/lib/api/security-results-types";

export type WorkbenchResultChunkInput = {
  jobId: string;
  kind: ResultChunkKind;
  limit: number;
  offset: number;
};

export type WorkbenchResultBackendService = {
  backendId: string;
  fetchChunk<TItem = Record<string, unknown>>(
    input: WorkbenchResultChunkInput,
  ): Promise<ResultChunkPayload<TItem>>;
};

export type WorkbenchResultBackendTransport = {
  backendId: string;
  fetchChunk<TItem = Record<string, unknown>>(
    jobId: string,
    kind: ResultChunkKind,
    options: { limit?: number; offset?: number },
  ): Promise<ResultChunkPayload<TItem>>;
};

export function createResultBackendService(
  transport: WorkbenchResultBackendTransport,
): WorkbenchResultBackendService {
  return {
    backendId: transport.backendId,
    fetchChunk(input) {
      return transport.fetchChunk(input.jobId, input.kind, {
        limit: input.limit,
        offset: input.offset,
      });
    },
  };
}
