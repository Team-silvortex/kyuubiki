import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, before, test } from "node:test";

import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import {
  chromium,
  startIsolatedWorkbenchUiRuntime,
  workbenchUrl,
} from "./workbench-ui-isolated.shared.mjs";

const FIXED_AT = "2026-08-13T00:00:00.000Z";
const ADMIN_JOB_ID = "qualification-admin-job";

let browser;
let runtime;

before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  browser = await launchIntegrationBrowser(chromium);
}, { timeout: 180_000 });

after(async () => {
  await browser?.close();
  await runtime?.stop();
}, { timeout: 90_000 });

function baseProject() {
  return {
    project_id: "qualification-project",
    name: "Qualification project",
    description: "Isolated browser qualification",
    inserted_at: FIXED_AT,
    updated_at: FIXED_AT,
    models: [],
  };
}

function resetBackendState() {
  runtime.state.projects.splice(0, runtime.state.projects.length, baseProject());
  runtime.state.projectMutations.length = 0;
  runtime.state.adminJobs.length = 0;
  runtime.state.adminResults.length = 0;
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
}

function seedAdminRecords() {
  runtime.state.adminJobs.push({
    job_id: ADMIN_JOB_ID,
    status: "completed",
    worker_id: "qualification-agent",
    message: "initial qualification message",
    progress: 1,
    has_result: true,
    project_id: "qualification-project",
    model_version_id: null,
    simulation_case_id: "qualification-case-1",
    created_at: FIXED_AT,
    updated_at: FIXED_AT,
  });
  runtime.state.adminResults.push({
    job_id: ADMIN_JOB_ID,
    result: { metric: 1, verdict: "initial" },
    inserted_at: FIXED_AT,
    updated_at: FIXED_AT,
  });
}

async function click(page, selector, label) {
  const target = page.locator(selector);
  await target.waitFor({ state: "visible", timeout: 30_000 });
  assert.equal(await target.count(), 1, `${label} should resolve to one visible control`);
  await target.click({ timeout: 15_000 });
}

async function openWorkbench(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt), undefined, { timeout: 30_000 });
  await page.evaluate(() => window.__kyuubikiPwdt.waitUntil(
    (state) => state.selectedProjectId === "qualification-project",
    { timeoutMs: 15_000 },
  ));
}

test("Workbench Library GUI creates, updates, exports, and deletes a project", async () => {
  resetBackendState();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await openWorkbench(page);
    await click(page, '[aria-label="workbench-rail:library"]', "Library rail");
    await click(page, '[data-workbench-library-tab="projects"]', "Library projects tab");
    const projectPanel = page.locator('[data-workbench-library-projects="panel"]');
    await projectPanel.waitFor({ state: "visible" });

    await projectPanel.locator('[data-workbench-library-project-field="name"]').fill("GUI qualification project");
    await projectPanel.locator('[data-workbench-library-project-field="description"]').fill("Created through the real Library GUI");
    const createResponse = page.waitForResponse((response) =>
      response.request().method() === "POST" && new URL(response.url()).pathname === "/api/v1/projects",
    );
    await projectPanel.locator('[data-workbench-library-project-action="create"]').click();
    assert.equal((await createResponse).status(), 201);

    const selection = projectPanel.locator('[data-workbench-library-project-field="selection"]');
    await page.waitForFunction(() =>
      document.querySelector('[data-workbench-library-project-field="selection"]')?.value === "qualification-created-project-1",
    );
    assert.equal(await selection.inputValue(), "qualification-created-project-1");
    assert.deepEqual(runtime.state.projectMutations[0], {
      method: "POST",
      project_id: "qualification-created-project-1",
      body: {
        name: "GUI qualification project",
        description: "Created through the real Library GUI",
      },
    });

    await projectPanel.locator('[data-workbench-library-project-field="name"]').fill("GUI qualification project updated");
    await projectPanel.locator('[data-workbench-library-project-field="description"]').fill("Updated through the same visible form");
    const updateResponse = page.waitForResponse((response) =>
      response.request().method() === "PATCH" &&
      new URL(response.url()).pathname === "/api/v1/projects/qualification-created-project-1",
    );
    await projectPanel.locator('[data-workbench-library-project-action="update"]').click();
    assert.equal((await updateResponse).status(), 200);
    await page.waitForFunction(() =>
      [...document.querySelectorAll('[data-workbench-library-project-field="selection"] option')]
        .some((option) => option.textContent === "GUI qualification project updated"),
    );

    await projectPanel.locator('[data-workbench-library-project-page="exchange"]').click();
    const downloadPromise = page.waitForEvent("download");
    await projectPanel.locator('[data-workbench-library-project-action="export-json"]').click();
    const download = await downloadPromise;
    assert.equal(download.suggestedFilename(), "GUI qualification project updated.kyuubiki.json");
    const downloadPath = await download.path();
    assert.ok(downloadPath);
    const bundle = JSON.parse(await readFile(downloadPath, "utf8"));
    assert.equal(bundle.project.project_id, "qualification-created-project-1");
    assert.equal(bundle.project.name, "GUI qualification project updated");

    await projectPanel.locator('[data-workbench-library-project-page="manage"]').click();
    page.once("dialog", (dialog) => dialog.accept());
    const deleteResponse = page.waitForResponse((response) =>
      response.request().method() === "DELETE" &&
      new URL(response.url()).pathname === "/api/v1/projects/qualification-created-project-1",
    );
    await projectPanel.locator('[data-workbench-library-project-action="delete"]').click();
    assert.equal((await deleteResponse).status(), 200);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({
      selectedProjectId: null, selectedModelId: null, selectedVersionId: null,
    }));
    assert.equal(await selection.inputValue(), "", "deletion must not silently select another project");
    assert.equal(runtime.state.projects.length, 1);
    assert.equal(runtime.state.projectMutations.at(-1)?.method, "DELETE");
    await selection.selectOption("qualification-project");
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: "qualification-project" }));
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench Data GUI updates, exports, and deletes Job and Result records", async () => {
  resetBackendState();
  seedAdminRecords();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true });
  const page = await context.newPage();
  const pageErrors = [];
  const requests = [];
  const requestFailures = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("request", (request) => requests.push(`${request.method()} ${request.url()}`));
  page.on("requestfailed", (request) => requestFailures.push({
    request: `${request.method()} ${request.url()}`,
    failure: request.failure()?.errorText ?? "unknown",
  }));

  try {
    await openWorkbench(page);
    await click(page, '[aria-label="workbench-rail:system"]', "System rail");
    await click(page, '[data-workbench-system-surface-tab="data"]', "System Data surface");
    const dataPanel = page.locator('[data-workbench-data-admin="panel"]');
    await dataPanel.waitFor({ state: "visible" });

    await dataPanel.locator('[data-workbench-data-page="browse"]').click();
    const jobRecord = dataPanel.locator(`[data-workbench-data-record-kind="job"][data-workbench-data-record-id="${ADMIN_JOB_ID}"]`);
    await jobRecord.waitFor({ state: "visible" });
    await jobRecord.click();
    await dataPanel.locator('[data-workbench-data-page="edit"]').click();
    const messageField = dataPanel.locator('[data-workbench-data-field="job-message"]');
    await messageField.waitFor({ state: "visible" });
    await messageField.fill("updated through the Data GUI");
    await dataPanel.locator('[data-workbench-data-field="job-case"]').fill("qualification-case-2");
    const updateJobResponse = page.waitForResponse((response) =>
      response.request().method() === "PATCH" && new URL(response.url()).pathname === `/api/v1/jobs/${ADMIN_JOB_ID}`,
      { timeout: 10_000 },
    );
    await dataPanel.locator('[data-workbench-data-action="save-job"]').click();
    try {
      assert.equal((await updateJobResponse).status(), 200);
    } catch (error) {
      const diagnostics = await page.evaluate(() => ({
        state: window.__kyuubikiPwdt.state(),
        message: document.querySelector('[data-workbench-data-field="job-message"]')?.value,
        actions: [...document.querySelectorAll("[data-workbench-data-action]")].map((entry) => ({
          action: entry.getAttribute("data-workbench-data-action"),
          disabled: entry.disabled,
        })),
      }));
      throw new Error(`${String(error)}\ndiagnostics=${JSON.stringify(diagnostics)}\nbackend=${JSON.stringify(runtime.state)}`);
    }
    assert.equal(runtime.state.adminJobs[0]?.message, "updated through the Data GUI");
    assert.equal(runtime.state.adminJobs[0]?.simulation_case_id, "qualification-case-2");

    await dataPanel.locator('[data-workbench-data-tab="results"]').click();
    await dataPanel.locator('[data-workbench-data-page="browse"]').click();
    const resultRecord = dataPanel.locator(`[data-workbench-data-record-kind="result"][data-workbench-data-record-id="${ADMIN_JOB_ID}"]`);
    await resultRecord.waitFor({ state: "visible" });
    await resultRecord.click();
    await dataPanel.locator('[data-workbench-data-page="edit"]').click();
    const resultField = dataPanel.locator('[data-workbench-data-field="result-payload"]');
    await resultField.fill(JSON.stringify({ metric: 42, verdict: "updated" }, null, 2));
    const updateResultResponse = page.waitForResponse((response) =>
      response.request().method() === "PATCH" && new URL(response.url()).pathname === `/api/v1/results/${ADMIN_JOB_ID}`,
      { timeout: 10_000 },
    );
    await dataPanel.locator('[data-workbench-data-action="save-result"]').click();
    try {
      assert.equal((await updateResultResponse).status(), 200);
    } catch (error) {
      const diagnostics = await page.evaluate(() => ({
        state: window.__kyuubikiPwdt.state(),
        resultDraft: document.querySelector('[data-workbench-data-field="result-payload"]')?.value,
        actions: [...document.querySelectorAll("[data-workbench-data-action]")].map((entry) => ({
          action: entry.getAttribute("data-workbench-data-action"),
          disabled: entry.disabled,
        })),
      }));
      throw new Error(
        `${String(error)}\ndiagnostics=${JSON.stringify(diagnostics)}\n` +
        `requests=${JSON.stringify(requests)}\nfailures=${JSON.stringify(requestFailures)}\n` +
        `backend=${JSON.stringify(runtime.state)}`,
      );
    }
    assert.deepEqual(runtime.state.adminResults[0]?.result, { metric: 42, verdict: "updated" });

    const resultDownloadPromise = page.waitForEvent("download");
    await dataPanel.locator('[data-workbench-data-action="export-result"]').click();
    const resultDownload = await resultDownloadPromise;
    assert.equal(resultDownload.suggestedFilename(), `${ADMIN_JOB_ID}-result.json`);
    const resultDownloadPath = await resultDownload.path();
    assert.ok(resultDownloadPath);
    assert.deepEqual(JSON.parse(await readFile(resultDownloadPath, "utf8")), { metric: 42, verdict: "updated" });

    const deleteResultResponse = page.waitForResponse((response) =>
      response.request().method() === "DELETE" && new URL(response.url()).pathname === `/api/v1/results/${ADMIN_JOB_ID}`,
    );
    await dataPanel.locator('[data-workbench-data-action="delete-result"]').click();
    assert.equal((await deleteResultResponse).status(), 200);
    await dataPanel.locator('[data-workbench-data-page="browse"]').click();
    await resultRecord.waitFor({ state: "detached" });

    await dataPanel.locator('[data-workbench-data-tab="jobs"]').click();
    await jobRecord.waitFor({ state: "visible" });
    await jobRecord.click();
    await dataPanel.locator('[data-workbench-data-page="edit"]').click();
    const deleteJobResponse = page.waitForResponse((response) =>
      response.request().method() === "DELETE" && new URL(response.url()).pathname === `/api/v1/jobs/${ADMIN_JOB_ID}`,
    );
    await dataPanel.locator('[data-workbench-data-action="delete-job"]').click();
    assert.equal((await deleteJobResponse).status(), 200);
    await dataPanel.locator('[data-workbench-data-page="browse"]').click();
    await jobRecord.waitFor({ state: "detached" });
    assert.equal(runtime.state.jobRecordMutations.at(-1)?.method, "DELETE");
    assert.equal(runtime.state.resultRecordMutations.at(-1)?.method, "DELETE");
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });
