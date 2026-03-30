import { parsePlaygroundModel } from "./model-import.mjs";

function formatScientific(value, fractionDigits = 3) {
  return Number(value).toExponential(fractionDigits);
}

function formatFixed(value, fractionDigits = 3) {
  return Number(value).toFixed(fractionDigits);
}

function collectInput(form) {
  const data = new FormData(form);

  return {
    length: Number(data.get("length")),
    area: Number(data.get("area")),
    youngs_modulus_gpa: Number(data.get("youngsModulusGpa")),
    elements: Number(data.get("elements")),
    tip_force: Number(data.get("tipForce")),
  };
}

function renderSummary(payload) {
  const { job, result } = payload;

  document.querySelector("[data-tip-displacement]").textContent = formatScientific(
    result.tip_displacement
  );
  document.querySelector("[data-max-stress]").textContent = formatScientific(result.maxStress);
  document.querySelector("[data-reaction-force]").textContent = formatFixed(result.reactionForce, 2);
  document.querySelector("[data-node-count]").textContent = String(result.nodes.length);
  document.querySelector("[data-job-id]").textContent = job.job_id;
  document.querySelector("[data-job-status]").textContent = job.status;
}

function renderSvg(result) {
  const svg = document.querySelector("[data-bar-svg]");
  const width = 760;
  const height = 240;
  const padding = 48;
  const baseY = 92;
  const deformedY = 168;
  const usableWidth = width - padding * 2;
  const scale = result.maxDisplacement === 0 ? 1 : (usableWidth * 0.18) / result.maxDisplacement;

  const originalPoints = result.nodes
    .map((node) => {
      const x = padding + (node.x / result.input.length) * usableWidth;
      return `${x},${baseY}`;
    })
    .join(" ");

  const deformedPoints = result.nodes
    .map((node) => {
      const x =
        padding +
        (node.x / result.input.length) * usableWidth +
        node.displacement * scale;
      return `${x},${deformedY}`;
    })
    .join(" ");

  const markers = result.nodes
    .map((node, index) => {
      const x = padding + (node.x / result.input.length) * usableWidth;
      const dx = x + node.displacement * scale;

      return `
        <circle cx="${x}" cy="${baseY}" r="5" fill="#1c4f6b"></circle>
        <circle cx="${dx}" cy="${deformedY}" r="6" fill="#f08a24"></circle>
        <text x="${dx}" y="${deformedY + 24}" text-anchor="middle" class="node-label">n${index}</text>
      `;
    })
    .join("");

  svg.innerHTML = `
    <defs>
      <linearGradient id="bar-gradient" x1="0%" x2="100%">
        <stop offset="0%" stop-color="#146c94"></stop>
        <stop offset="100%" stop-color="#f08a24"></stop>
      </linearGradient>
    </defs>
    <rect x="0" y="0" width="${width}" height="${height}" rx="24" fill="#fffdf8"></rect>
    <text x="${padding}" y="34" class="chart-title">Axial Bar FEM Response</text>
    <line x1="${padding}" y1="${baseY}" x2="${width - padding}" y2="${baseY}" class="guide-line"></line>
    <line x1="${padding}" y1="${deformedY}" x2="${width - padding}" y2="${deformedY}" class="guide-line guide-line--soft"></line>
    <polyline points="${originalPoints}" class="bar-line"></polyline>
    <polyline points="${deformedPoints}" class="bar-line bar-line--deformed"></polyline>
    <text x="${padding}" y="${baseY - 16}" class="axis-label">undeformed</text>
    <text x="${padding}" y="${deformedY - 16}" class="axis-label">deformed (scaled)</text>
    ${markers}
  `;
}

function renderTable(result) {
  const body = document.querySelector("[data-element-table]");

  body.innerHTML = result.elements
    .map(
      (element) => `
        <tr>
          <td>${element.index + 1}</td>
          <td>${formatFixed(element.x1, 3)} - ${formatFixed(element.x2, 3)}</td>
          <td>${formatScientific(element.strain)}</td>
          <td>${formatScientific(element.stress)}</td>
          <td>${formatFixed(element.axialForce, 2)}</td>
        </tr>
      `
    )
    .join("");
}

function render(result) {
  renderSummary(result);
  renderSvg(result.result);
  renderTable(result.result);
}

function normalizePayload(payload) {
  return {
    job: payload.job,
    result: {
      ...payload.result,
      reactionForce: payload.result.reaction_force,
      maxDisplacement: payload.result.max_displacement,
      maxStress: payload.result.max_stress,
    },
  };
}

async function runAnalysis(form) {
  const response = await fetch("/api/playground/run", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(collectInput(form)),
  });

  const payload = await response.json();

  if (!response.ok) {
    throw new Error(payload.error || "analysis failed");
  }

  return normalizePayload(payload);
}

function bindMaterialPreset(form) {
  const materialSelect = form.querySelector("[data-material]");
  const modulusInput = form.querySelector("[data-youngs-modulus]");

  materialSelect.addEventListener("change", () => {
    const preset = materialSelect.value;

    if (preset !== "custom") {
      modulusInput.value = preset;
    }
  });
}

function applyImportedModel(form, imported) {
  form.querySelector("#length").value = String(imported.length);
  form.querySelector("#area").value = String(imported.area);
  form.querySelector("#elements").value = String(imported.elements);
  form.querySelector("#tipForce").value = String(imported.tipForce);
  form.querySelector("[data-material]").value = imported.material;
  form.querySelector("[data-youngs-modulus]").value = String(imported.youngsModulusGpa);
  document.querySelector("[data-model-name]").textContent = imported.name;
}

function bindModelImport(form, errorBox) {
  const input = form.querySelector("[data-model-file]");

  input.addEventListener("change", async () => {
    const [file] = input.files;

    if (!file) {
      return;
    }

    try {
      const imported = parsePlaygroundModel(await file.text());
      applyImportedModel(form, imported);
      errorBox.textContent = "";
      errorBox.hidden = true;
    } catch (error) {
      errorBox.textContent = `import failed: ${error.message}`;
      errorBox.hidden = false;
    }
  });
}

function boot() {
  const form = document.querySelector("[data-fem-form]");
  const errorBox = document.querySelector("[data-error]");
  const runButton = document.querySelector("[data-run-button]");

  bindMaterialPreset(form);
  bindModelImport(form, errorBox);

  const update = async () => {
    try {
      runButton.disabled = true;
      runButton.textContent = "Running...";
      const result = await runAnalysis(form);
      errorBox.textContent = "";
      errorBox.hidden = true;
      render(result);
    } catch (error) {
      errorBox.textContent = error.message;
      errorBox.hidden = false;
    } finally {
      runButton.disabled = false;
      runButton.textContent = "Run Through Orchestration";
    }
  };

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    update();
  });

  update();
}

boot();
