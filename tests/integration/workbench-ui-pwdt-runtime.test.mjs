import assert from "node:assert/strict";
import { test } from "node:test";
import {
  initialProject, usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks,
} from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

test("Workbench PWDT switches every study kind without an empty model or client crash", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    const kinds = ["axial_bar_1d", "heat_bar_1d", "heat_plane_triangle_2d", "heat_plane_quad_2d",
      "electrostatic_plane_triangle_2d", "electrostatic_plane_quad_2d", "thermal_bar_1d", "thermal_beam_1d",
      "thermal_frame_2d", "thermal_truss_2d", "thermal_truss_3d", "thermal_plane_triangle_2d", "thermal_plane_quad_2d",
      "spring_1d", "spring_2d", "spring_3d", "beam_1d", "torsion_1d", "truss_2d", "truss_3d",
      "plane_triangle_2d", "plane_quad_2d", "frame_2d"];
    for (const kind of kinds) {
      await invoke(page, "nav/setStudyKind", { studyKind: kind });
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.studyKind, kind);
      assert.equal(state.hasResult, false);
    }
  });
});

for (const kind of ["axial_bar_1d", "heat_plane_triangle_2d", "heat_plane_quad_2d", "electrostatic_plane_triangle_2d", "electrostatic_plane_quad_2d"]) {
test(`Workbench PWDT hasResult tracks ${kind} completion and reset`, { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    let input;
    const job = { job_id: "runtime-result-job", status: "solving", worker_id: "qualification-agent", progress: 0.25,
      has_result: false, created_at: initialProject.inserted_at, updated_at: initialProject.updated_at };
    await page.route("**/api/v1/fem/**/jobs", async (route) => {
      input = route.request().postDataJSON();
      await route.fulfill({ status: 202, json: { job } });
    });
    await page.route(`**/api/v1/jobs/${job.job_id}`, async (route) => {
      const heat = kind.startsWith("heat_");
      const result = kind === "axial_bar_1d" ? {
        input, tip_displacement: 0, reaction_force: 0, max_displacement: 0, max_stress: 0,
        nodes: [{ index: 0, x: 0, displacement: 0 }, { index: 1, x: input.length, displacement: 0 }],
        elements: [{ index: 0, x1: 0, x2: input.length, strain: 0, stress: 0, axial_force: 0 }],
      } : {
        input,
        ...(heat ? { max_temperature: 0, max_heat_flux: 0 } : { max_potential: 0, max_electric_field: 0, max_flux_density: 0 }),
        nodes: input.nodes.map((node, index) => ({ ...node, index,
          ...(heat ? { temperature: 0, heat_load: 0 } : { potential: 0, charge_density: 0 }),
        })),
        elements: input.elements.map((element, index) => ({ ...element, index, area: 1,
          ...(heat ? { average_temperature: 0, temperature_gradient_x: 0, temperature_gradient_y: 0,
            heat_flux_x: 0, heat_flux_y: 0, heat_flux_magnitude: 0 } : { average_potential: 0,
            potential_gradient_x: 0, potential_gradient_y: 0, electric_field_x: 0, electric_field_y: 0,
            electric_field_magnitude: 0, electric_flux_density_x: 0, electric_flux_density_y: 0, electric_flux_density_magnitude: 0 }),
        })),
      };
      await route.fulfill({ json: { job: { ...job, status: "completed", progress: 1, has_result: true }, result } });
    });
    await openWorkbench(page);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    await invoke(page, "nav/setStudyKind", { studyKind: kind });
    await invoke(page, "job/run");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), true);
    await invoke(page, "nav/setStudyKind", { studyKind: "truss_2d" });
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
  });
});
}

async function openPythonPanel(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.locator('[data-workbench-system-surface-tab="settings"]').click();
  await page.locator('[data-workbench-system-settings-page="scripts"]').click();
  const launch = page.getByRole("button", { name: "Load runtime", exact: true }).locator("xpath=ancestor::section[1]");
  await launch.waitFor({ state: "visible" });
  return launch;
}

// This adapter tests browser download/initialization recovery, not Python or WASM execution.
function runtimeAdapter(failure) {
  return `window.loadPyodide = async () => {
    window.pwdtInitializationCount = (window.pwdtInitializationCount || 0) + 1;
    if (${JSON.stringify(failure)} === "initialization" && window.pwdtInitializationCount === 1) {
      throw new Error("qualification initialization failed");
    }
    return { runPythonAsync: async (source) => {
      window.pwdtSources = [...(window.pwdtSources || []), source];
      if (${JSON.stringify(failure)} === "execution" && window.pwdtSources.length === 1) {
        throw new Error("qualification execution failed");
      }
      window.__kyuubikiBridge.log("qualification runtime adapter ready");
    } };
  };`;
}

for (const failure of ["network", "missing-loader", "initialization"]) {
for (const revisit of [false, true]) {
test(`Workbench PWDT recovers ${failure} without a page reload${revisit ? " after a panel round trip" : ""}`, { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    let downloads = 0;
    await page.route("https://cdn.jsdelivr.net/pyodide/**/pyodide.js", async (route) => {
      downloads += 1;
      await route.fulfill({
        status: downloads === 1 && failure === "network" ? 503 : 200,
        contentType: "application/javascript",
        body: downloads === 1 && failure !== "initialization" ? "/* unavailable runtime */" : runtimeAdapter(failure),
      });
    });
    await openWorkbench(page);
    let launch = await openPythonPanel(page);
    const editor = page.locator("textarea.script-panel__editor").first();
    await editor.fill("ky.log('keep my script after failure')");
    await launch.getByRole("button", { name: "Load runtime", exact: true }).click();
    await launch.locator(".status-chip--risk").waitFor();
    assert.equal(await editor.inputValue(), "ky.log('keep my script after failure')");
    if (revisit) {
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
      launch = await openPythonPanel(page);
    }
    await launch.getByRole("button", { name: "Load runtime", exact: true }).click();
    await launch.locator(".status-chip--good").waitFor({ timeout: 10_000 });
    assert.equal(downloads, failure === "initialization" ? 1 : 2);
    assert.equal(await page.evaluate(() => window.pwdtInitializationCount), failure === "initialization" ? 2 : 1);
    assert.equal(await editor.inputValue(), "ky.log('keep my script after failure')");
    await launch.getByRole("button", { name: "Run script", exact: true }).click();
    await page.waitForFunction(() => window.pwdtSources?.length === 2);
    assert.match(await page.evaluate(() => window.pwdtSources.at(-1)), /keep my script after failure/u);
    assert.equal(await page.locator('script[data-pyodide="true"]').count(), 1);
  });
});
}
}

test("Workbench PWDT retries script execution without discarding the loaded runtime", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    let downloads = 0;
    await page.route("https://cdn.jsdelivr.net/pyodide/**/pyodide.js", async (route) => {
      downloads += 1;
      await route.fulfill({ contentType: "application/javascript", body: runtimeAdapter("execution") });
    });
    await openWorkbench(page);
    const launch = await openPythonPanel(page);
    await launch.getByRole("button", { name: "Run script", exact: true }).click();
    await launch.locator(".status-chip--risk").waitFor();
    await launch.getByRole("button", { name: "Run script", exact: true }).click();
    await launch.locator(".status-chip--good").waitFor();
    assert.equal(downloads, 1);
    assert.equal(await page.evaluate(() => window.pwdtInitializationCount), 1);
    assert.equal(await page.evaluate(() => window.pwdtSources.length), 2);
  });
});

test("Workbench PWDT retains an in-flight execution across panel unload and allows a second run", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    // This adapter exercises view lifetime and concurrency, not Python/WASM computation.
    await page.route("https://cdn.jsdelivr.net/pyodide/**/pyodide.js", (route) => route.fulfill({
      contentType: "application/javascript",
      body: `window.loadPyodide = async () => ({ runPythonAsync: async () => {
        window.pwdtExecutionCount = (window.pwdtExecutionCount || 0) + 1;
        const bridge = window.__kyuubikiBridge;
        bridge.log("execution-start-" + window.pwdtExecutionCount);
        if (window.pwdtExecutionCount === 1) {
          await window.__kyuubikiPwdt.openSidebar("model");
          await new Promise(resolve => { window.releasePwdtExecution = resolve; });
        }
        bridge.log("execution-end-" + window.pwdtExecutionCount);
      } });`,
    }));
    await openWorkbench(page);
    let launch = await openPythonPanel(page);
    await launch.getByRole("button", { name: "Run script", exact: true }).click();
    await page.waitForFunction(() => typeof window.releasePwdtExecution === "function");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().sidebarSection), "model");
    launch = await openPythonPanel(page);
    assert.equal(await launch.getByRole("button", { name: "Run script", exact: true }).isDisabled(), true);
    assert.equal(await launch.getByRole("button", { name: "Load runtime", exact: true }).isDisabled(), true);
    assert.match(await page.locator("body").innerText(), /execution-start-1/u);
    await page.evaluate(() => window.releasePwdtExecution());
    await launch.locator(".status-chip--good").waitFor();
    assert.match(await page.locator("body").innerText(), /execution-end-1/u);
    await launch.getByRole("button", { name: "Run script", exact: true }).click();
    await page.waitForFunction(() => window.pwdtExecutionCount === 2);
    await launch.locator(".status-chip--good").waitFor();
    assert.match(await page.locator("body").innerText(), /execution-end-2/u);
  });
});

for (const status of ["failed", "cancelled"]) {
test(`Workbench PWDT recipe stops at a real action-chain ${status} response`, { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    let submissions = 0;
    const job = { job_id: "runtime-terminal-job", status: "solving", worker_id: "qualification-agent", progress: 0.25,
      has_result: false, created_at: initialProject.inserted_at, updated_at: initialProject.updated_at };
    await page.route("**/api/v1/fem/truss-2d/jobs", async (route) => {
      submissions += 1;
      await route.fulfill({ status: 202, json: { job } });
    });
    await page.route(`**/api/v1/jobs/${job.job_id}`, (route) => route.fulfill({ json: {
      job: { ...job, status, message: `qualification computation ${status}` },
    } }));
    await openWorkbench(page);
    await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.runRecipe("recipe/truss2d/closed-loop", {
      bays: 3, modelName: "Retained failed research",
    })), new RegExp(status, "iu"));
    assert.equal(submissions, 1, "a failed recipe must not automatically resubmit");
    assert.equal(library.models.length, 1, "the saved input remains available for inspection");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().jobStatus), status);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    assert.equal((await invoke(page, "nav/setSidebarSection", { section: "model" })).ok, true);
  });
});
}
