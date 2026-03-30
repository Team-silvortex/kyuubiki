import { solveBar1D } from "./fem.mjs";

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
    youngsModulus: Number(data.get("youngsModulus")),
    elements: Number(data.get("elements")),
    tipForce: Number(data.get("tipForce")),
  };
}

function renderSummary(result) {
  document.querySelector("[data-tip-displacement]").textContent = formatScientific(
    result.nodes.at(-1).displacement
  );
  document.querySelector("[data-max-stress]").textContent = formatScientific(result.maxStress);
  document.querySelector("[data-reaction-force]").textContent = formatFixed(result.reactionForce, 2);
  document.querySelector("[data-node-count]").textContent = String(result.nodes.length);
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
  renderSvg(result);
  renderTable(result);
}

function boot() {
  const form = document.querySelector("[data-fem-form]");
  const errorBox = document.querySelector("[data-error]");

  const update = () => {
    try {
      const result = solveBar1D(collectInput(form));
      errorBox.textContent = "";
      errorBox.hidden = true;
      render(result);
    } catch (error) {
      errorBox.textContent = error.message;
      errorBox.hidden = false;
    }
  };

  form.addEventListener("input", update);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    update();
  });

  update();
}

boot();
