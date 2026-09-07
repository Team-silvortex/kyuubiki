import { initialProject, runtime } from "./workbench-ui-project-fixture.shared.mjs";

// Synthetic solver responses exercise data transfer and UI state, not numerical accuracy.
export async function installProjectionSolver(page, { incompleteHeat = false, electrostaticFailure = null, heatFailure = null } = {}) {
  const submissions = [];
  const results = [];
  const solver = { submissions, results, electrostaticFailure, heatFailure, beforeJobResponse: null };
  await page.route("**/api/v1/fem/**/jobs", async (route) => {
    const kind = new URL(route.request().url()).pathname.split("/").at(-2).replaceAll("-", "_");
    const input = route.request().postDataJSON();
    submissions.push({ kind, input });
    const job = { job_id: `projection-job-${submissions.length}`, status: "solving", progress: 0.2,
      worker_id: "qualification-agent", has_result: false,
      project_id: input.project_id, model_version_id: input.model_version_id,
      created_at: initialProject.inserted_at, updated_at: initialProject.updated_at };
    const result = buildProjectionResult(kind, input);
    if (incompleteHeat && kind.startsWith("heat_")) result.nodes.pop();
    if (kind.startsWith("heat_")) {
      if (solver.heatFailure === "conductivity") result.input.elements[0].conductivity *= 2;
      if (solver.heatFailure === "heat_load") result.input.nodes[0].heat_load = 123;
      if (solver.heatFailure === "boundary") result.input.nodes[0].fix_temperature = !result.input.nodes[0].fix_temperature;
    }
    if (kind.startsWith("electrostatic_")) {
      if (solver.electrostaticFailure === "missing-node") result.nodes.pop();
      if (solver.electrostaticFailure === "duplicate-node") result.nodes[1].index = result.nodes[0].index;
      if (solver.electrostaticFailure === "invalid-field") result.elements[0].electric_field_magnitude = null;
      if (solver.electrostaticFailure === "stale-input") result.input.elements[0].thickness *= 2;
    }
    results.push(result);
    const completed = { ...job, status: "completed", progress: 1, has_result: true };
    await page.route(`**/api/v1/jobs/${job.job_id}`, async (poll) => {
      if (!runtime.state.adminJobs.some((entry) => entry.job_id === job.job_id)) {
        runtime.state.adminJobs.push(completed);
        runtime.state.adminResults.push({ job_id: job.job_id, result,
          inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at });
      }
      await poll.fulfill({ json: { job: completed, result } });
    });
    await solver.beforeJobResponse?.(job);
    await route.fulfill({ status: 202, json: { job } });
  });
  return solver;
}

function buildProjectionResult(kind, input) {
  const heat = kind.startsWith("heat_");
  const electrostatic = kind.startsWith("electrostatic_");
  const result = {
    input: structuredClone(input),
    ...(heat ? { max_temperature: 80, max_heat_flux: 1 }
      : electrostatic ? { max_potential: 10, max_electric_field: 1, max_flux_density: 1 }
        : { max_displacement: 0, max_stress: 0, max_axial_force: 0, max_temperature_delta: 80 }),
    nodes: input.nodes.map((node, index) => ({ ...node, index,
      ...(heat ? { temperature: 80 - index * 10, heat_load: 0 }
        : electrostatic ? { potential: 10 - index, charge_density: 0 }
          : { ux: 0, uy: 0, displacement_magnitude: 0, temperature_delta: node.temperature_delta }),
    })),
    elements: input.elements.map((element, index) => ({ ...element, index, area: element.area ?? 1, length: 1,
      ...(heat ? { average_temperature: 50, temperature_gradient: 1, heat_flux: 1,
        temperature_gradient_x: 1, temperature_gradient_y: 0, heat_flux_x: 1, heat_flux_y: 0, heat_flux_magnitude: 1 }
        : electrostatic ? { average_potential: 5, potential_gradient_x: 1, potential_gradient_y: 0,
          electric_field_x: 1, electric_field_y: 0, electric_field_magnitude: 1,
          electric_flux_density_x: 1, electric_flux_density_y: 0, electric_flux_density_magnitude: 1 }
          : { average_temperature_delta: 50, thermal_strain: 0, mechanical_strain: 0, total_strain: 0,
            stress: 0, axial_force: 0, mechanical_strain_x: 0, mechanical_strain_y: 0,
            total_strain_x: 0, total_strain_y: 0, gamma_xy: 0, stress_x: 0, stress_y: 0,
            tau_xy: 0, principal_stress_1: 0, principal_stress_2: 0, max_in_plane_shear: 0, von_mises: 0 }),
    })),
  };
  // Results can arrive out of array order; the node index is the identity contract.
  if (heat || electrostatic) result.nodes.reverse();
  if (electrostatic) result.elements.reverse();
  return result;
}

export async function openProjectionButton(page) {
  await page.locator('[data-workbench-inspector-tab-target="actions"]').click();
  await page.locator('[data-workbench-inspector-actions-target="exports"]').click();
  return page.locator('[data-workbench-project-heat-to-thermo="true"]');
}
