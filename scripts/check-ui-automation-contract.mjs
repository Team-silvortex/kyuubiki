#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const REQUIRED_SELECTORS = [
  {
    key: "shell",
    value: '[data-workbench-shell="root"]',
    implementationFiles: ["apps/frontend/src/components/workbench/workbench-shell-frame.tsx"],
    implementationNeedle: 'data-workbench-shell="root"',
  },
  {
    key: "sidebar",
    value: '[data-workbench-panel="sidebar"]',
    implementationFiles: ["apps/frontend/src/components/workbench/workbench-sidebar-panel.tsx"],
    implementationNeedle: 'data-workbench-panel="sidebar"',
  },
  {
    key: "railButton(section)",
    value: "workbench-rail:${section}",
    tsNeedle: "workbench-rail:",
    implementationFiles: ["apps/frontend/src/components/workbench/workbench-app-rail.tsx"],
    implementationNeedle: "workbench-rail:",
  },
  {
    key: "loadedModelState",
    value: '[data-workbench-state="loaded-model"]',
    implementationFiles: ["apps/frontend/src/components/workbench/workbench-main-shell-mount.tsx"],
    implementationNeedle: 'data-workbench-state="loaded-model"',
  },
  {
    key: "libraryTab(tab)",
    value: "workbench-library-tab:${tab}",
    tsNeedle: "workbench-library-tab:",
    implementationFiles: ["apps/frontend/src/components/workbench/library/workbench-library-sidebar.tsx"],
    implementationNeedle: "workbench-library-tab:",
  },
  {
    key: "sampleDomain(domain)",
    value: "workbench-sample-domain:${domain}",
    tsNeedle: "workbench-sample-domain:",
    implementationFiles: ["apps/frontend/src/components/workbench/library/workbench-library-sidebar.tsx"],
    implementationNeedle: "workbench-sample-domain:",
  },
  {
    key: "sample(sampleId)",
    value: "workbench-sample:${sampleId}",
    tsNeedle: "workbench-sample:",
    implementationFiles: ["apps/frontend/src/components/workbench/library/workbench-library-sidebar.tsx"],
    implementationNeedle: "workbench-sample:",
  },
  {
    key: "runtimePanel",
    value: '[data-workbench-runtime="panel"]',
    implementationFiles: ["apps/frontend/src/components/workbench/system/workbench-system-runtime-panel.tsx"],
    implementationNeedle: 'data-workbench-runtime="panel"',
  },
  {
    key: "controlWindow",
    value: '[data-workbench-control-window="root"]',
    implementationFiles: ["apps/frontend/src/components/workbench/system/workbench-system-control-mode-window.tsx"],
    implementationNeedle: 'data-workbench-control-window="root"',
  },
];

function readText(relativePath) {
  return readFileSync(path.join(ROOT, relativePath), "utf8");
}

function readContract() {
  return JSON.parse(readText("docs/ui-automation-contract.json"));
}

function hasContractSelector(contract, key, value) {
  return contract.selectors?.[key] === value;
}

function requireContains(issues, file, text, needle, label) {
  if (!text.includes(needle)) {
    issues.push(`${file} is missing ${label}: ${needle}`);
  }
}

function auditUiAutomationContract() {
  const issues = [];
  const contract = readContract();
  const tsContract = readText("apps/frontend/src/components/workbench/workbench-ui-automation-contract.ts");
  const htmlContract = readText("docs/ui-automation-contract.html");

  if (contract.name !== "Kyuubiki Workbench UI Automation Contract") {
    issues.push("docs/ui-automation-contract.json has an unexpected contract name");
  }
  if (contract.productOwned !== true || contract.userExtensible !== false) {
    issues.push("UI automation contract must remain product-owned and non-user-extensible");
  }
  if (contract.contractVersion !== 1) {
    issues.push(`expected contractVersion 1, got ${contract.contractVersion}`);
  }
  requireContains(issues, "docs/ui-automation-contract.html", htmlContract, contract.version, "contract version line");
  if (!contract.rules?.some((rule) => rule.includes("stable data-* selectors"))) {
    issues.push("contract rules must require stable selectors instead of visual text");
  }
  if (!contract.rules?.some((rule) => rule.includes("product-owned ids"))) {
    issues.push("contract rules must mark automation labels as product-owned ids");
  }

  requireContains(
    issues,
    "apps/frontend/src/components/workbench/workbench-ui-automation-contract.ts",
    tsContract,
    "WORKBENCH_UI_AUTOMATION_CONTRACT_VERSION = 1",
    "matching contract version",
  );

  for (const selector of REQUIRED_SELECTORS) {
    if (!hasContractSelector(contract, selector.key, selector.value)) {
      issues.push(`docs/ui-automation-contract.json selector ${selector.key} must equal ${selector.value}`);
    }

    requireContains(
      issues,
      "apps/frontend/src/components/workbench/workbench-ui-automation-contract.ts",
      tsContract,
      selector.tsNeedle ?? selector.value,
      `selector ${selector.key}`,
    );
    requireContains(issues, "docs/ui-automation-contract.html", htmlContract, selector.value, `documented selector ${selector.key}`);

    for (const file of selector.implementationFiles) {
      requireContains(issues, file, readText(file), selector.implementationNeedle, `implementation for ${selector.key}`);
    }
  }

  if (issues.length > 0) {
    console.error("UI automation contract drift detected:");
    for (const issue of issues) console.error(`- ${issue}`);
    process.exitCode = 1;
    return;
  }

  console.log("UI automation contract ok");
}

function selfTest() {
  const contract = {
    name: "Kyuubiki Workbench UI Automation Contract",
    productOwned: true,
    userExtensible: false,
    contractVersion: 1,
    selectors: Object.fromEntries(REQUIRED_SELECTORS.map((selector) => [selector.key, selector.value])),
    rules: ["Automation must target stable data-* selectors.", "Accessible labels are product-owned ids."],
  };

  assert.equal(hasContractSelector(contract, "railButton(section)", "workbench-rail:${section}"), true);
  assert.equal(hasContractSelector(contract, "missing", "value"), false);
  assert.ok(REQUIRED_SELECTORS.some((selector) => selector.key === "loadedModelState"));
  console.log("UI automation contract self-test passed");
}

if (process.argv.includes("--self-test")) {
  selfTest();
} else {
  auditUiAutomationContract();
}
