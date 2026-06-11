import { performance } from "node:perf_hooks";
import {
  chromium,
  FRONTEND_URL,
  runKyuubiki,
  waitForFrontend,
} from "./workbench-ui-smoke.shared.mjs";

function round(value) {
  return Math.round(value * 1000) / 1000;
}

async function measureStep(name, fn) {
  const startedAt = performance.now();
  await fn();
  const endedAt = performance.now();
  return {
    name,
    duration_ms: round(endedAt - startedAt),
  };
}

async function waitForNextPaint(page) {
  await page.evaluate(
    () =>
      new Promise((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(resolve));
      }),
  );
}

async function openSample(page, domainLabel, sampleLabel, importedModelLabel, studyLabel) {
  const steps = [];
  steps.push(
    await measureStep("open_history", async () => {
      await page.getByRole("button", { name: "H History" }).click();
    }),
  );
  steps.push(
    await measureStep("open_samples", async () => {
      await page.getByRole("button", { name: /^S\s+Samples$/ }).click();
    }),
  );
  steps.push(
    await measureStep(`select_domain:${domainLabel}`, async () => {
      await page
        .locator("button")
        .filter({ hasText: new RegExp(`^${domainLabel}$`) })
        .first()
        .click();
    }),
  );
  steps.push(
    await measureStep(`select_sample:${sampleLabel}`, async () => {
      await page
        .locator("button.history-item")
        .filter({ hasText: sampleLabel })
        .first()
        .click();
      await page.waitForFunction(
        ({ importedModel, study }) => {
          const text = document.body.innerText || "";
          return text.includes(`Imported model: ${importedModel}`) && text.includes(study);
        },
        { importedModel: importedModelLabel, study: studyLabel },
        { timeout: 60_000 },
      );
    }),
  );
  steps.push(
    await measureStep("open_result", async () => {
      await page.getByRole("button", { name: "Result" }).first().click();
    }),
  );
  steps.push(
    await measureStep("open_actions", async () => {
      await page.getByRole("button", { name: "Actions" }).first().click();
    }),
  );
  steps.push(
    await measureStep("open_export_menu", async () => {
      await page.getByRole("button", { name: "Export Data" }).first().click();
      await page.getByRole("button", { name: "Export Data JSON" }).first().waitFor({ state: "visible", timeout: 15_000 });
      await page.getByRole("button", { name: "Export Data CSV" }).first().waitFor({ state: "visible", timeout: 15_000 });
    }),
  );
  return steps;
}

async function runCase(browser, config) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
  try {
    const load = await measureStep(`goto:${config.id}`, async () => {
      await page.goto(FRONTEND_URL, { waitUntil: "networkidle", timeout: 60_000 });
    });
    const steps = await openSample(
      page,
      config.domain,
      config.sample,
      config.importedModel,
      config.studyLabel,
    );
    return {
      id: config.id,
      domain: config.domain,
      sample: config.sample,
      bootstrap_ms: load.duration_ms,
      steps,
      total_ms: round(load.duration_ms + steps.reduce((sum, step) => sum + step.duration_ms, 0)),
    };
  } finally {
    await page.close();
  }
}

async function runWorkflowCase(browser) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
  try {
    const load = await measureStep("goto:workflow", async () => {
      await page.goto(FRONTEND_URL, { waitUntil: "networkidle", timeout: 60_000 });
    });
    const steps = [];
    steps.push(
      await measureStep("open_workflow_rail", async () => {
        await page.getByRole("button", { name: /Workflow/ }).click();
        await page.getByRole("button", { name: "Catalog" }).first().waitFor({ state: "visible", timeout: 30_000 });
      }),
    );
    steps.push(
      await measureStep("open_workflow_catalog", async () => {
        await page.getByRole("button", { name: "Catalog" }).first().click();
        await page.getByText("Workflow Catalog").first().waitFor({ state: "visible", timeout: 30_000 });
      }),
    );
    const builderButtonCount = await page.getByRole("button", { name: "Open builder" }).count();
    steps.push(
      await measureStep("open_workflow_builder", async () => {
        if (builderButtonCount > 0) {
          await page.getByRole("button", { name: "Open builder" }).first().click();
          await waitForNextPaint(page);
          return;
        }
        await page.getByRole("button", { name: "Builder" }).first().click();
        await waitForNextPaint(page);
      }),
    );
    steps.push(
      await measureStep("open_workflow_runs", async () => {
        await page.getByRole("button", { name: "Runs" }).first().click();
        await waitForNextPaint(page);
      }),
    );
    steps.push(
      await measureStep("return_workflow_builder", async () => {
        await page.getByRole("button", { name: "Builder" }).first().click();
        await waitForNextPaint(page);
      }),
    );
    return {
      id: "workflow.builder-surface",
      domain: "Workflow",
      sample: "Builder surface",
      bootstrap_ms: load.duration_ms,
      steps,
      total_ms: round(load.duration_ms + steps.reduce((sum, step) => sum + step.duration_ms, 0)),
    };
  } finally {
    await page.close();
  }
}

async function main() {
  const browser = await chromium.launch({ headless: true });
  try {
    runKyuubiki(["restart-local"]);
    await waitForFrontend();

    const cases = [];
    cases.push(
      await runCase(browser, {
        id: "mechanical.spring-grid-2d",
        domain: "Mechanical",
        sample: "Spring Grid 2D",
        importedModel: "spring-grid-2d",
        studyLabel: "2D spring",
      }),
    );
    cases.push(
      await runCase(browser, {
        id: "thermal.heat-plane-quad-2d",
        domain: "Thermal",
        sample: "Heat Plane Quad 2D",
        importedModel: "Heat Plane Quad 2D",
        studyLabel: "2D heat plane quad",
      }),
    );
    cases.push(await runWorkflowCase(browser));

    const bootstrapSorted = [...cases].sort((left, right) => right.bootstrap_ms - left.bootstrap_ms);
    const summary = {
      slowest_case: [...cases].sort((left, right) => right.total_ms - left.total_ms)[0]?.id ?? null,
      slowest_step: [...cases.flatMap((entry) => entry.steps.map((step) => ({ case_id: entry.id, ...step })))]
        .sort((left, right) => right.duration_ms - left.duration_ms)[0] ?? null,
      slowest_bootstrap: bootstrapSorted[0]
        ? { case_id: bootstrapSorted[0].id, duration_ms: bootstrapSorted[0].bootstrap_ms }
        : null,
    };

    console.log(
      JSON.stringify(
        {
          url: FRONTEND_URL,
          generated_at: new Date().toISOString(),
          summary,
          cases,
        },
        null,
        2,
      ),
    );
  } finally {
    await browser.close();
    try {
      runKyuubiki(["stop"]);
    } catch {
      // best effort cleanup
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
