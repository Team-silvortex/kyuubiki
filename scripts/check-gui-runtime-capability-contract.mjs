#!/usr/bin/env node
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/gui-runtime-capability-manifest.schema.json";
const examplePath = "schemas/examples.gui-runtime-capability-manifest.json";
const manifestDir = "config/gui-runtime-capabilities";
const schemasReadmePath = "schemas/README.md";
const boundaryDocPath = "docs/app-runtime-boundaries.md";
const configReadmePath = "config/README.md";
const frontendCapabilityPath = "apps/frontend/src/lib/api/gui-runtime-capabilities.ts";
const frontendApiIndexPath = "apps/frontend/src/lib/api/index.ts";
const frontendCapabilityTestPath = "apps/frontend/test/workflow/workbench-gui-runtime-capabilities.test.ts";
const runtimeClientPath = "apps/frontend/src/lib/api/runtime-client.ts";
const securityResultsClientPath = "apps/frontend/src/lib/api/security-results-client.ts";
const projectClientPath = "apps/frontend/src/lib/api/project-client.ts";
const headlessResultsClientPath = "apps/frontend/src/lib/api/headless-results-client.ts";
const headlessHandoffClientPath = "apps/frontend/src/lib/api/headless-handoff-client.ts";
const backendServiceComposerPath = "apps/frontend/src/lib/workbench/backend-service-composer.ts";
const workbenchRootPath = "apps/frontend/src/components/workbench/workbench.tsx";
const projectLibraryServicePath = "apps/frontend/src/lib/workbench/project-library-backend-service.ts";
const headlessExecutionPath = "apps/frontend/src/lib/scripting/workbench-headless-execution.ts";
const headlessWorkflowPanelPath = "apps/frontend/src/components/workbench/workbench-headless-workflow-panel.tsx";
const headlessWorkflowPanelActionsPath = "apps/frontend/src/components/workbench/workbench-headless-workflow-panel-actions.ts";
const headlessWorkflowPanelStatePath = "apps/frontend/src/components/workbench/workbench-headless-workflow-panel-state.ts";
const headlessWorkflowBackendServicePath = "apps/frontend/src/lib/workbench/headless-workflow-backend-service.ts";
const resultBackendServicePath = "apps/frontend/src/lib/workbench/result-backend-service.ts";
const securityEventBackendServicePath = "apps/frontend/src/lib/workbench/security-event-backend-service.ts";
const workbenchRuntimeServiceFiles = [
  "apps/frontend/src/lib/workbench/admin-data-backend-service.ts",
  "apps/frontend/src/lib/workbench/job-history-backend-service.ts",
  "apps/frontend/src/lib/workbench/runtime-status-backend-service.ts",
  "apps/frontend/src/lib/workbench/workflow-backend-service.ts",
];

const SCHEMA_VERSION = "kyuubiki.gui-runtime-capability-manifest/v1";
const UI_OWNERSHIP = "product_owned_static_ui";
const REQUIRED_SURFACE_KINDS = new Set(["hub", "workbench", "installer", "mobile_webview"]);
const FORBIDDEN_MOBILE_TARGETS = new Set(["agent", "mesh", "direct_runtime", "installer_runtime"]);
const FORBIDDEN_HUB_TARGETS = new Set(["agent", "mesh", "direct_runtime", "installer_runtime"]);
const FORBIDDEN_WORKBENCH_TARGETS = new Set(["installer_runtime"]);

function fail(message) {
  console.error(`GUI runtime capability contract check failed: ${message}`);
  process.exit(1);
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function listManifestPaths() {
  return fs
    .readdirSync(path.join(repoRoot, manifestDir))
    .filter((fileName) => fileName.endsWith(".json"))
    .sort()
    .map((fileName) => `${manifestDir}/${fileName}`);
}

function requireArray(value, field, context, minLength = 0) {
  if (!Array.isArray(value) || value.length < minLength) {
    fail(`${context}: ${field} must be an array with at least ${minLength} item(s)`);
  }
}

function requireString(value, field, context) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${context}: ${field} must be a non-empty string`);
  }
}

function checkSchema(schema) {
  if (schema?.properties?.schema_version?.const !== SCHEMA_VERSION) {
    fail(`${schemaPath}: schema_version const must be ${SCHEMA_VERSION}`);
  }
  if (schema?.properties?.ui_ownership?.const !== UI_OWNERSHIP) {
    fail(`${schemaPath}: ui_ownership const must be ${UI_OWNERSHIP}`);
  }
  const automation = schema?.properties?.automation_contract?.properties;
  if (automation?.wasm_python_stability_required?.const !== true) {
    fail(`${schemaPath}: automation contract must require WASM Python stability`);
  }
  if (automation?.user_extensible_ui_allowed?.const !== false) {
    fail(`${schemaPath}: GUI automation UI must remain non-user-extensible`);
  }
}

function checkManifest(manifest, context) {
  if (manifest.schema_version !== SCHEMA_VERSION) {
    fail(`${context}: schema_version must be ${SCHEMA_VERSION}`);
  }
  if (manifest.ui_ownership !== UI_OWNERSHIP) {
    fail(`${context}: ui_ownership must be ${UI_OWNERSHIP}`);
  }
  requireString(manifest.surface_id, "surface_id", context);
  requireString(manifest.surface_kind, "surface_kind", context);
  if (manifest.automation_contract?.wasm_python_stability_required !== true) {
    fail(`${context}: automation_contract.wasm_python_stability_required must be true`);
  }
  if (manifest.automation_contract?.user_extensible_ui_allowed !== false) {
    fail(`${context}: automation_contract.user_extensible_ui_allowed must be false`);
  }

  requireArray(manifest.runtime_bindings, "runtime_bindings", context, 1);
  const bindingIds = new Set();
  for (const [index, binding] of manifest.runtime_bindings.entries()) {
    const bindingContext = `${context}#runtime_bindings/${index}`;
    requireString(binding.binding_id, "binding_id", bindingContext);
    if (bindingIds.has(binding.binding_id)) {
      fail(`${bindingContext}: duplicate binding_id ${binding.binding_id}`);
    }
    bindingIds.add(binding.binding_id);
    requireString(binding.target_kind, "target_kind", bindingContext);
    requireString(binding.binding_mode, "binding_mode", bindingContext);
    requireArray(binding.required_capabilities, "required_capabilities", bindingContext, 1);
    requireArray(binding.optional_capabilities, "optional_capabilities", bindingContext);
    if (binding.headless_sdk_parity_required !== true) {
      fail(`${bindingContext}: headless_sdk_parity_required must stay true for decoupled surfaces`);
    }
    if (typeof binding.mobile_supported !== "boolean") {
      fail(`${bindingContext}: mobile_supported must be a boolean`);
    }
    if (manifest.surface_kind === "mobile_webview") {
      if (binding.mobile_supported !== true) {
        fail(`${bindingContext}: mobile WebView bindings must be mobile_supported`);
      }
      if (FORBIDDEN_MOBILE_TARGETS.has(binding.target_kind)) {
        fail(`${bindingContext}: mobile WebView must not bind to ${binding.target_kind}`);
      }
    }
    if (manifest.surface_kind === "hub" && FORBIDDEN_HUB_TARGETS.has(binding.target_kind)) {
      fail(`${bindingContext}: Hub must not bind directly to ${binding.target_kind}`);
    }
    if (manifest.surface_kind === "workbench" && FORBIDDEN_WORKBENCH_TARGETS.has(binding.target_kind)) {
      fail(`${bindingContext}: Workbench must not bind directly to ${binding.target_kind}`);
    }
    requireString(binding.credential_surface, "credential_surface", bindingContext);
    requireString(binding.notes, "notes", bindingContext);
  }

  if (!manifest.runtime_bindings.some((binding) => binding.target_kind === "orchestra")) {
    fail(`${context}: at least one orchestra binding is required`);
  }
  if (!manifest.runtime_bindings.some((binding) => binding.mobile_supported === true)) {
    fail(`${context}: at least one binding must support mobile WebView clients`);
  }
  if (
    manifest.surface_kind === "installer" &&
    !manifest.runtime_bindings.some((binding) => binding.target_kind === "installer_runtime")
  ) {
    fail(`${context}: Installer must expose an installer_runtime binding`);
  }
  requireArray(manifest.degraded_modes, "degraded_modes", context, 1);
}

function checkManifestSet(manifestPaths) {
  const surfaceKinds = new Set();
  for (const manifestPath of manifestPaths) {
    const manifest = readJson(manifestPath);
    checkManifest(manifest, manifestPath);
    surfaceKinds.add(manifest.surface_kind);
  }

  for (const requiredKind of REQUIRED_SURFACE_KINDS) {
    if (!surfaceKinds.has(requiredKind)) {
      fail(`${manifestDir}: missing product manifest for ${requiredKind}`);
    }
  }
}

function checkDocumentation() {
  for (const docPath of [schemasReadmePath, boundaryDocPath, configReadmePath]) {
    const text = readText(docPath);
    for (const needle of [schemaPath, examplePath, manifestDir, SCHEMA_VERSION]) {
      if (!text.includes(needle)) {
        fail(`${docPath}: missing ${needle}`);
      }
    }
  }
}

function requireContains(text, needle, context) {
  if (!text.includes(needle)) {
    fail(`${context}: missing ${needle}`);
  }
}

function checkFrontendCapabilityApi() {
  const capabilityApi = readText(frontendCapabilityPath);
  const apiIndex = readText(frontendApiIndexPath);
  const tests = readText(frontendCapabilityTestPath);

  for (const exportedName of [
    "GuiRuntimeCapabilityManifest",
    "listGuiRuntimeManifestCapabilities",
    "hasGuiRuntimeManifestCapability",
    "selectGuiRuntimeManifestBindings",
    "resolveWorkbenchGuiRuntimeCapabilityFromManifest",
  ]) {
    requireContains(capabilityApi, exportedName, frontendCapabilityPath);
    requireContains(tests, exportedName, frontendCapabilityTestPath);
  }

  requireContains(apiIndex, 'export * from "./gui-runtime-capabilities.ts";', frontendApiIndexPath);
  requireContains(capabilityApi, 'options.hostKind === "mobile_webview"', frontendCapabilityPath);
  requireContains(tests, "mobile binding selection hides desktop-only direct mesh capabilities", frontendCapabilityTestPath);
  requireContains(readText(boundaryDocPath), "GUI code should select bindings by declared capability", boundaryDocPath);
}

function checkRuntimeClientBoundary() {
  const runtimeClient = readText(runtimeClientPath);
  const securityResultsClient = readText(securityResultsClientPath);
  const projectClient = readText(projectClientPath);
  const headlessResultsClient = readText(headlessResultsClientPath);
  const headlessHandoffClient = readText(headlessHandoffClientPath);
  const composer = readText(backendServiceComposerPath);
  const projectLibraryService = readText(projectLibraryServicePath);
  const workbenchRoot = readText(workbenchRootPath);
  const headlessExecution = readText(headlessExecutionPath);
  const headlessWorkflowPanel = readText(headlessWorkflowPanelPath);
  const headlessWorkflowPanelActions = readText(headlessWorkflowPanelActionsPath);
  const headlessWorkflowPanelState = readText(headlessWorkflowPanelStatePath);
  const headlessWorkflowBackendService = readText(headlessWorkflowBackendServicePath);
  const resultService = readText(resultBackendServicePath);
  const securityEventService = readText(securityEventBackendServicePath);
  requireContains(runtimeClient, "createRuntimeApiClient", runtimeClientPath);
  requireContains(runtimeClient, "defaultRuntimeApiClient", runtimeClientPath);
  requireContains(securityResultsClient, "createSecurityResultsApiClient", securityResultsClientPath);
  requireContains(securityResultsClient, "defaultSecurityResultsApiClient", securityResultsClientPath);
  requireContains(projectClient, "createProjectApiClient", projectClientPath);
  requireContains(projectClient, "defaultProjectApiClient", projectClientPath);
  requireContains(headlessResultsClient, "createHeadlessResultsApiClient", headlessResultsClientPath);
  requireContains(headlessResultsClient, "defaultHeadlessResultsApiClient", headlessResultsClientPath);
  requireContains(headlessHandoffClient, "createHeadlessHandoffApiClient", headlessHandoffClientPath);
  requireContains(headlessHandoffClient, "defaultHeadlessHandoffApiClient", headlessHandoffClientPath);
  requireContains(composer, "createWorkbenchRuntimeBackedBackendServices", backendServiceComposerPath);
  requireContains(composer, "defaultWorkbenchRuntimeBackedBackendServices", backendServiceComposerPath);
  requireContains(composer, "defaultRuntimeApiClient", backendServiceComposerPath);
  requireContains(composer, "defaultSecurityResultsApiClient", backendServiceComposerPath);
  requireContains(projectLibraryService, "defaultProjectApiClient", projectLibraryServicePath);
  requireContains(workbenchRoot, "workbenchProjectLibraryBackendService", workbenchRootPath);
  requireContains(headlessExecution, "HeadlessExecutionBackendClients", headlessExecutionPath);
  requireContains(headlessExecution, "defaultHeadlessResultsApiClient", headlessExecutionPath);
  requireContains(headlessExecution, "defaultProjectApiClient", headlessExecutionPath);
  requireContains(headlessExecution, "defaultRuntimeApiClient", headlessExecutionPath);
  requireContains(headlessWorkflowBackendService, "createWorkbenchHeadlessWorkflowBackendService", headlessWorkflowBackendServicePath);
  requireContains(headlessWorkflowBackendService, "defaultHeadlessHandoffApiClient", headlessWorkflowBackendServicePath);
  requireContains(headlessWorkflowBackendService, "defaultRuntimeApiClient", headlessWorkflowBackendServicePath);
  requireContains(headlessWorkflowPanel, "defaultWorkbenchHeadlessWorkflowBackendService", headlessWorkflowPanelPath);
  requireContains(headlessWorkflowPanel, "workbench-headless-workflow-panel-actions", headlessWorkflowPanelPath);
  requireContains(headlessWorkflowPanelActions, "buildHeadlessAgentDispatchPlanFromBackend", headlessWorkflowPanelActionsPath);
  requireContains(headlessWorkflowPanelActions, "buildHeadlessOrchestraHandoffFromBackend", headlessWorkflowPanelActionsPath);
  requireContains(headlessWorkflowPanelActions, "submitHeadlessOrchestraHandoffFromBackend", headlessWorkflowPanelActionsPath);
  requireContains(headlessWorkflowPanel, "workbench-headless-workflow-panel-state", headlessWorkflowPanelPath);
  requireContains(headlessWorkflowPanelState, "buildFrontendMacroBridgePayload", headlessWorkflowPanelStatePath);
  requireContains(headlessWorkflowPanelState, "parseFrontendMacroBridgePayload", headlessWorkflowPanelStatePath);
  requireContains(headlessWorkflowPanelState, "moveItem", headlessWorkflowPanelStatePath);
  requireContains(resultService, "defaultSecurityResultsApiClient", resultBackendServicePath);
  requireContains(securityEventService, "defaultSecurityResultsApiClient", securityEventBackendServicePath);

  for (const filePath of workbenchRuntimeServiceFiles) {
    const text = readText(filePath);
    requireContains(text, "defaultWorkbenchRuntimeBackedBackendServices", filePath);
    if (/import\s*\{[^}]*\b(fetch|submit|create|update|delete|cancel)[A-Za-z0-9_]*\b[^}]*\}\s*from\s*["']@\/lib\/api\/runtime-client["']/u.test(text)) {
      fail(`${filePath}: Workbench services must use the runtime client instance instead of importing runtime functions directly`);
    }
  }

  requireContains(readText("apps/frontend/src/lib/workbench/study-run-backend-service.ts"), "createStudyRunBackendServiceFromRuntimeClient", "apps/frontend/src/lib/workbench/study-run-backend-service.ts");

  for (const [filePath, text] of [
    [backendServiceComposerPath, composer],
    [resultBackendServicePath, resultService],
    [securityEventBackendServicePath, securityEventService],
  ]) {
    if (/import\s*\{[^}]*\b(fetch|submit|create|update|delete|cancel)[A-Za-z0-9_]*\b[^}]*\}\s*from\s*["']@\/lib\/api\/security-results-client["']/u.test(text)) {
      fail(`${filePath}: Workbench services must use the security results client instance instead of importing API functions directly`);
    }
  }

  if (/import\s*\{[^}]*\b(create|fetch|update|delete)[A-Za-z0-9_]*\b[^}]*\}\s*from\s*["']@\/lib\/api\/project-client["']/u.test(projectLibraryService)) {
    fail(`${projectLibraryServicePath}: project library service must use the project client instance instead of importing API functions directly`);
  }
  if (/import\s*\{[^}]*\b(create|fetch|update|delete)(Project|Model|ModelVersion)[A-Za-z0-9_]*\b[^}]*\}\s*from\s*["']@\/lib\/api["']/u.test(workbenchRoot)) {
    fail(`${workbenchRootPath}: Workbench root must use project library backend service instead of importing project API functions directly`);
  }
  if (/from\s*["']@\/lib\/api["']/u.test(headlessExecution)) {
    fail(`${headlessExecutionPath}: headless execution must import concrete API contract files instead of the API facade`);
  }
  if (/import\s*\{[^}]*\b(fetchProtocolAgents|submitHeadlessOrchestraHandoff|fetchHeadlessOrchestraHandoff[A-Za-z0-9_]*)\b[^}]*\}\s*from\s*["']@\/lib\/api["']/u.test(headlessWorkflowPanel)) {
    fail(`${headlessWorkflowPanelPath}: headless workflow panel must use the backend service instead of importing headless/runtime API functions directly`);
  }
}

function checkContracts() {
  checkSchema(readJson(schemaPath));
  checkManifest(readJson(examplePath), examplePath);
  checkManifestSet(listManifestPaths());
  checkDocumentation();
  checkFrontendCapabilityApi();
  checkRuntimeClientBoundary();
  console.log("GUI runtime capability contract check passed");
}

function runSelfTest() {
  const example = readJson(examplePath);
  example.runtime_bindings[0].headless_sdk_parity_required = false;
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    assert.throws(() => checkManifest(example, examplePath), /self-test-fail/);
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  assert.equal(failed, true);

  const mobileManifest = readJson("config/gui-runtime-capabilities/mobile-webview.json");
  mobileManifest.runtime_bindings[0].target_kind = "agent";
  failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    assert.throws(() => checkManifest(mobileManifest, "mobile-self-test"), /self-test-fail/);
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  assert.equal(failed, true);
  console.log("GUI runtime capability contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
