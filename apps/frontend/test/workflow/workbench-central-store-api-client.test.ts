import test from "node:test";
import assert from "node:assert/strict";

import { createCentralStoreApiClient } from "@/lib/api/central-store-client";
import { requestJsonWithContext, type WorkbenchApiRequestContext } from "@/lib/api/core";

type SeenRequest = {
  url: string;
  method: string;
};

function createRecordingClient() {
  const seen: SeenRequest[] = [];
  const client = createCentralStoreApiClient(async <T>(url: string, init?: RequestInit) => {
    seen.push({ url, method: init?.method ?? "GET" });
    return { ok: true } as T;
  });
  return { client, seen };
}

test("central store API client fetches catalog with encoded filters", async () => {
  const { client, seen } = createRecordingClient();

  await client.fetchCentralCatalog({ kind: "language_pack", q: "zh TW", source_id: "builtin.language-packs" });

  assert.deepEqual(seen, [
    {
      url: "/api/v1/central/catalog?kind=language_pack&q=zh+TW&source_id=builtin.language-packs",
      method: "GET",
    },
  ]);
});

test("central store API client fetches entry and session policy", async () => {
  const { client, seen } = createRecordingClient();

  await client.fetchCentralStoreEntry("workflow_template", "workflow.alpha/beta");
  await client.fetchCentralSessionPolicy();
  await client.fetchCentralPublishPolicy();
  await client.fetchCentralPublisherPolicy();
  await client.fetchCentralPublishReadiness();
  await client.fetchCentralDatabasePolicy();
  await client.fetchCentralProvenancePolicy();
  await client.fetchCentralArtifactAdmissionPolicy();
  await client.fetchCentralPublishPipeline();
  await client.fetchCentralDatabaseStatus();

  assert.deepEqual(seen, [
    {
      url: "/api/v1/central/catalog/workflow_template/workflow.alpha%2Fbeta",
      method: "GET",
    },
    {
      url: "/api/v1/central/session-policy",
      method: "GET",
    },
    {
      url: "/api/v1/central/publish-policy",
      method: "GET",
    },
    {
      url: "/api/v1/central/publisher-policy",
      method: "GET",
    },
    {
      url: "/api/v1/central/publish-readiness",
      method: "GET",
    },
    {
      url: "/api/v1/central/database-policy",
      method: "GET",
    },
    {
      url: "/api/v1/central/provenance-policy",
      method: "GET",
    },
    {
      url: "/api/v1/central/artifact-admission-policy",
      method: "GET",
    },
    {
      url: "/api/v1/central/publish-pipeline",
      method: "GET",
    },
    {
      url: "/api/v1/central/database-status",
      method: "GET",
    },
  ]);
});

test("central store requests reuse workbench API auth context", async () => {
  const previousFetch = globalThis.fetch;
  const seenHeaders: Array<Record<string, string>> = [];
  const context: WorkbenchApiRequestContext = {
    resolveUrl: (url) => `http://127.0.0.1:4000${url}`,
    buildAuthHeaders: () => ({ "x-kyuubiki-token": "control" }),
  };

  globalThis.fetch = async (_url, init) => {
    seenHeaders.push(Object.fromEntries(new Headers(init?.headers).entries()));
    return new Response(JSON.stringify({ schema_version: "kyuubiki.central-store-catalog/v1" }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  };

  try {
    await createCentralStoreApiClient((url, init) => requestJsonWithContext(context, url, init)).fetchCentralCatalog();
    assert.equal(seenHeaders[0]?.["x-kyuubiki-token"], "control");
  } finally {
    globalThis.fetch = previousFetch;
  }
});

test("central store language-pack policy carries unsafe text evidence", async () => {
  const responses: Record<string, unknown> = {
    "/api/v1/central/publish-policy": {
      schema_version: "kyuubiki.central-publish-policy/v1",
      status: "blocked_preview",
      accepting_submissions: false,
      reason: "preview",
      resource_kinds: [
        {
          kind: "language_pack",
          manifest_schema: "schemas/language-pack.schema.json",
          required_evidence: ["language_pack", "locale_target", "surface_validation", "unsafe_text_scan"],
          distribution_modes: ["language_pack_catalog", "local_import"],
          mutable_after_publish: false,
        },
      ],
      review_stages: [],
      publisher_requirements: {
        login_required: true,
        publisher_account_required: true,
        personal_access_token_supported: false,
        device_code_supported: false,
        anonymous_publish_allowed: false,
      },
    },
    "/api/v1/central/publish-readiness": {
      schema_version: "kyuubiki.central-publish-readiness/v1",
      status: "blocked_preview",
      accepting_submissions: false,
      blocking_reasons: [],
      resource_readiness: [
        {
          kind: "language_pack",
          status: "blocked_preview",
          publish_evidence: ["language_pack", "locale_target", "surface_validation", "unsafe_text_scan"],
          provenance_attestations: [],
          installer_checks: [],
          blocking_reasons: [],
        },
      ],
      required_storage_tables: [],
      next_unlocks: [],
    },
  };
  const client = createCentralStoreApiClient(async <T>(url: string) => responses[url] as T);

  const publishPolicy = await client.fetchCentralPublishPolicy();
  const readiness = await client.fetchCentralPublishReadiness();
  const languagePackPolicy = publishPolicy.resource_kinds.find((entry) => entry.kind === "language_pack");
  const languagePackReadiness = readiness.resource_readiness.find((entry) => entry.kind === "language_pack");

  assert.ok(languagePackPolicy?.required_evidence.includes("unsafe_text_scan"));
  assert.ok(languagePackReadiness?.publish_evidence.includes("unsafe_text_scan"));
});
