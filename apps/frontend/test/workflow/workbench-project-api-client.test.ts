import test from "node:test";
import assert from "node:assert/strict";

import { createProjectApiClient } from "@/lib/api/project-client";

type SeenRequest = {
  body: unknown;
  method: string;
  url: string;
};

function createRecordingClient() {
  const seen: SeenRequest[] = [];
  const client = createProjectApiClient(async <T>(url: string, init?: RequestInit) => {
    seen.push({
      body: init?.body ? JSON.parse(String(init.body)) : null,
      method: init?.method ?? "GET",
      url,
    });
    return { ok: true } as T;
  });
  return { client, seen };
}

test("project API client routes project CRUD through injected request", async () => {
  const { client, seen } = createRecordingClient();

  await client.fetchProjects();
  await client.createProject({ name: "Demo", description: "Local" });
  await client.updateProject("project-a", { name: "Renamed" });
  await client.deleteProject("project-a");

  assert.deepEqual(seen, [
    { body: null, method: "GET", url: "/api/v1/projects" },
    { body: { name: "Demo", description: "Local" }, method: "POST", url: "/api/v1/projects" },
    { body: { name: "Renamed" }, method: "PATCH", url: "/api/v1/projects/project-a" },
    { body: null, method: "DELETE", url: "/api/v1/projects/project-a" },
  ]);
});

test("project API client routes model CRUD through injected request", async () => {
  const { client, seen } = createRecordingClient();

  await client.fetchModel("model-a");
  await client.createModel("project-a", { kind: "truss_2d", name: "Model", payload: { nodes: [] } });
  await client.updateModel("model-a", { material: "steel" });
  await client.deleteModel("model-a");

  assert.deepEqual(seen, [
    { body: null, method: "GET", url: "/api/v1/models/model-a" },
    {
      body: { kind: "truss_2d", name: "Model", payload: { nodes: [] } },
      method: "POST",
      url: "/api/v1/projects/project-a/models",
    },
    { body: { material: "steel" }, method: "PATCH", url: "/api/v1/models/model-a" },
    { body: null, method: "DELETE", url: "/api/v1/models/model-a" },
  ]);
});

test("project API client routes model version CRUD through injected request", async () => {
  const { client, seen } = createRecordingClient();

  await client.fetchModelVersions("model-a");
  await client.fetchModelVersion("version-a");
  await client.createModelVersion("model-a", { name: "v2", payload: { elements: [] } });
  await client.updateModelVersion("version-a", { name: "v1-renamed" });
  await client.deleteModelVersion("version-a");

  assert.deepEqual(seen, [
    { body: null, method: "GET", url: "/api/v1/models/model-a/versions" },
    { body: null, method: "GET", url: "/api/v1/model-versions/version-a" },
    {
      body: { name: "v2", payload: { elements: [] } },
      method: "POST",
      url: "/api/v1/models/model-a/versions",
    },
    { body: { name: "v1-renamed" }, method: "PATCH", url: "/api/v1/model-versions/version-a" },
    { body: null, method: "DELETE", url: "/api/v1/model-versions/version-a" },
  ]);
});
