import assert from "node:assert/strict";
import { before, after, test } from "node:test";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

let browser, runtime;
before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  browser = await launchIntegrationBrowser(chromium);
}, { timeout: 180_000 });
after(async () => { await browser?.close(); await runtime?.stop(); }, { timeout: 90_000 });

for (const width of [1440, 390]) {
  test(`Agent API configuration, verification and secret clearing at ${width}px`, async () => {
    const context = await browser.newContext({ viewport: { width, height: 900 } });
    const page = await context.newPage();
    const token = "test-only-agent-token-0123456789abcdef";
    const endpoint = "127.0.0.1:5001";
    const calls = [];
    try {
      await page.route("**/api/direct-mesh/agents", async route => {
        const headers = route.request().headers(); calls.push(headers);
        await route.fulfill({ json: { mode:"direct_mesh_gui", discovery:"manual", endpoint_count:1,
          agents:[{ id:"tls-test", host:"127.0.0.1", port:5001, descriptor: {
            runtime: {runtime_mode:"standalone",headless:true,peers:[]},
            connection_access: { authenticated: headers["x-kyuubiki-agent-token"] === token, encrypted:true, control_allowed:true },
          }}] } });
      });
      await page.goto(workbenchUrl(runtime), { waitUntil:"networkidle", timeout:60_000 });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await page.evaluate(endpoint => window.__kyuubikiPwdt.invoke("settings/patch", { language:"en", frontendRuntimeMode:"direct_mesh_gui", directMeshEndpointsText:endpoint }), endpoint);
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
      await page.locator('[data-workbench-system-surface-tab="settings"]').click();
      await page.locator('[data-workbench-system-settings-page="config"]').click();
      const panel = page.getByTestId("agent-api-security-panel");
      await panel.locator("summary").click();
      await panel.getByLabel("Pinned Agent address (host:port)", { exact:true }).fill(endpoint);
      await panel.getByLabel("Certificate server name", { exact:true }).fill("localhost");
      await panel.getByLabel("Trusted CA / self-signed certificate (PEM)", { exact:true }).fill("-----BEGIN CERTIFICATE-----\nZmFrZQ==\n-----END CERTIFICATE-----");
      await panel.getByLabel("Agent API token", { exact:true }).fill(token);
      await panel.getByRole("button", { name:"Apply to this session", exact:true }).click();
      assert.equal(await panel.getByLabel("Agent API token", { exact:true }).inputValue(), "");
      await panel.getByRole("button", { name:"Verify Agent connection (direct mode)", exact:true }).click();
      await page.waitForFunction(() => document.querySelector('[data-testid="agent-api-security-panel"] [role="status"]')?.textContent.includes("authentication verified"));
      const secured = calls.find(headers => headers["x-kyuubiki-agent-token"] === token);
      assert.ok(secured);
      assert.equal(secured["x-kyuubiki-agent-endpoint"], endpoint);
      assert.equal(secured["x-kyuubiki-agent-server-name"], "localhost");
      const persisted = await page.evaluate(() => JSON.stringify([localStorage, sessionStorage]));
      assert.ok(!persisted.includes(token));
      const box = await panel.boundingBox();
      assert.ok(box.x >= 0 && box.x + box.width <= width + 1);
      await panel.getByRole("button", { name:"Clear credentials", exact:true }).click();
      assert.equal(await panel.getByRole("button", { name:"Verify Agent connection (direct mode)", exact:true }).isDisabled(), true);
      await page.reload({ waitUntil:"networkidle" });
      assert.ok(!(await page.evaluate(() => JSON.stringify([localStorage, sessionStorage]))).includes(token));
    } finally { await context.close(); }
  }, { timeout:120_000 });
}
