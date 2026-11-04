import assert from "node:assert/strict";
import { before, after, test } from "node:test";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

let browser, runtime;
before(async () => { runtime = await startIsolatedWorkbenchUiRuntime(); browser = await launchIntegrationBrowser(chromium); }, { timeout:180_000 });
after(async () => { await browser?.close(); await runtime?.stop(); }, { timeout:90_000 });

function withinDeadline(promise) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error("Controlled model request did not settle")), 10_000);
  })]).finally(() => clearTimeout(timer));
}

for (const [width, height] of [[1440, 1000], [1024, 768], [390, 844]]) {
  test(`custom model connection workflow at ${width}px`, { timeout:120_000 }, async () => {
    const context = await browser.newContext({ viewport:{ width, height } });
    const page = await context.newPage(); const calls = [];
    page.setDefaultTimeout(10_000);
    let status = 200, generation = 0, holdModels = false, releaseModel, finishModel, beginModel;
    let modelGate;
    try {
      await page.route("**/api/assistant/capabilities", route => route.fulfill({ json:{ schema_version:"kyuubiki.model-gateway/v1" } }));
      await page.route("**/api/assistant/request", async route => {
        const body = route.request().postDataJSON(); calls.push(body);
        if (body.operation === "models") {
          if (holdModels) { beginModel?.(); await modelGate; }
          try {
            await route.fulfill(status === 200 ? { json:{ data:[{ id:"research-model" }, { id:"fast-model" }, ...Array.from({ length:200 }, (_, i) => ({ id:`catalog-model-${i}` }))] } }
              : { status, body:"private-secret-must-not-be-shown" });
          } catch (error) { if (!holdModels) throw error; }
          finally { if (holdModels) finishModel?.(); }
        } else {
          generation++;
          await route.fulfill({ json:{ choices:[{ message:{ content:JSON.stringify({ summary:"Test research plan", rationale:"Review before execution", suggested_actions:[] }) } }] } });
        }
      });
      await page.goto(workbenchUrl(runtime), { waitUntil:"networkidle", timeout:60_000 });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await page.evaluate(() => window.__kyuubikiPwdt.invoke("settings/patch", { language:"en" }));
      await page.locator(".assistant-float-launcher").click();
      await page.locator(".assistant-float-panel").getByRole("button", { name:"LLM", exact:true }).click();
      const panel = page.getByTestId("assistant-model-connection");
      await panel.getByLabel("API base URL", { exact:true }).fill("https://provider.example/v1/chat/completions");
      await panel.getByLabel("Model API key", { exact:true }).fill("private-secret-must-not-be-shown");
      await panel.getByLabel("Model ID", { exact:true }).fill("manual-model");
      assert.equal(await panel.getByRole("button", { name:"Test connection / load models", exact:true }).isDisabled(), true);
      await panel.getByRole("checkbox").check();
      await panel.getByRole("button", { name:"Use normalized URL", exact:true }).click();
      assert.equal(await panel.getByLabel("Model API key", { exact:true }).inputValue(), "private-secret-must-not-be-shown", "normalization preserves a key bound to the same endpoint");
      await panel.getByRole("button", { name:"Test connection / load models", exact:true }).click();
      await panel.getByText("Connection accepted. Select a model or keep your manually entered ID.", { exact:true }).waitFor();
      assert.equal(calls.length, 1); assert.equal(calls[0].operation, "models");
      assert.equal(calls[0].payload, undefined); assert.equal(generation, 0);
      assert.equal(await panel.getByTestId("assistant-current-model").innerText(), "manual-model");
      assert.ok(await panel.getByLabel("Available models", { exact:true }).locator("option").count() <= 41);
      await panel.getByLabel("Filter models", { exact:true }).fill("not-found");
      await panel.getByText("No matching models. Change the search or enter an ID.", { exact:true }).waitFor();
      await panel.getByLabel("Filter models", { exact:true }).fill("research");
      await panel.getByLabel("Available models", { exact:true }).selectOption("research-model");
      assert.equal(await panel.getByTestId("assistant-current-model").innerText(), "research-model");
      await panel.getByRole("button", { name:"Enter ID", exact:true }).click();
      assert.equal(await panel.getByLabel("Model ID", { exact:true }).inputValue(), "research-model");
      await panel.getByRole("button", { name:"Browse models", exact:true }).click();
      assert.equal(await panel.locator(".assistant-model-connection__advanced").getAttribute("open"), null);
      const layout = await panel.locator("fieldset").evaluateAll(elements => elements.map(e => {
        const r = e.getBoundingClientRect(); return { x:r.x, y:r.y, width:r.width, height:r.height };
      }));
      if (width > 1000) assert.ok(Math.abs(layout[0].y - layout[1].y) < 2, JSON.stringify(layout));
      else assert.ok(layout[1].y >= layout[0].y + layout[0].height, JSON.stringify(layout));
      await panel.locator("summary").first().scrollIntoViewIfNeeded();
      await page.screenshot({ path:`/tmp/kyuubiki-model-connection-ready-${width}.png` });
      await page.evaluate(() => window.__kyuubikiPwdt.invoke("settings/patch", { language:"zh" }));
      await panel.getByText("连接已被服务接受。请选择模型，或保留手动填写的 ID。", { exact:true }).waitFor();
      await page.locator(".assistant-float-panel__body").evaluate(element => { element.scrollTop = 0; });
      await page.screenshot({ path:`/tmp/kyuubiki-model-connection-ready-zh-${width}.png` });
      await page.evaluate(() => window.__kyuubikiPwdt.invoke("settings/patch", { language:"en" }));
      const dialog = page.locator(".assistant-float-panel");
      await panel.getByRole("button", { name:"Use this model / write request", exact:true }).click();
      assert.equal(await panel.locator("details").first().getAttribute("open"), null);
      await page.waitForFunction(() => document.activeElement?.matches(".assistant-float-panel textarea"));
      await dialog.getByLabel("Request", { exact:true }).fill("Suggest a safe study");
      await dialog.getByRole("button", { name:"Generate plan", exact:true }).click();
      await dialog.getByText("Test research plan", { exact:true }).waitFor();
      assert.equal(generation, 1);
      assert.equal(calls.at(-1).payload.model, "research-model");
      assert.ok(!JSON.stringify(calls.at(-1).payload).includes("private-secret"));
      status = 401;
      await panel.locator("summary").first().click();
      await panel.getByRole("button", { name:"Edit connection", exact:true }).click();
      await panel.getByRole("button", { name:"Test connection / load models", exact:true }).click();
      await panel.getByText(/service rejected authentication/).waitFor();
      assert.equal(await panel.getByLabel("Available models", { exact:true }).count(), 0, "failed revalidation must clear stale model discovery");
      assert.ok(!(await dialog.innerText()).includes("private-secret-must-not-be-shown"));
      await panel.getByLabel("API base URL", { exact:true }).fill("https://new-provider.example/v1");
      assert.equal(await panel.getByLabel("Model API key", { exact:true }).inputValue(), "");
      assert.equal(await dialog.getByText("Test research plan", { exact:true }).count(), 0);
      assert.equal(await panel.getByRole("checkbox").isChecked(), false);
      await panel.getByLabel("API base URL", { exact:true }).fill("http://localhost:1234/v1");
      status = 404;
      await panel.getByRole("button", { name:"Test connection / load models", exact:true }).click();
      await panel.getByText(/If \/models is unsupported/).waitFor();
      await panel.getByLabel("Model ID", { exact:true }).fill("manual-only-model");
      assert.equal(await panel.getByRole("button", { name:"Use this model / write request", exact:true }).isDisabled(), false);
      holdModels = true; status = 200;
      modelGate = new Promise(resolve => { releaseModel = resolve; });
      const started = new Promise(resolve => { beginModel = resolve; });
      const delivered = new Promise(resolve => { finishModel = resolve; });
      await panel.getByRole("button", { name:"Test connection / load models", exact:true }).click();
      await withinDeadline(started);
      await panel.getByLabel("API base URL", { exact:true }).fill("http://localhost:1235/v1");
      releaseModel(); await withinDeadline(delivered);
      await page.evaluate(() => new Promise(requestAnimationFrame));
      assert.equal(await panel.getByLabel("Available models", { exact:true }).count(), 0, "late results cannot repopulate another provider");
      assert.equal(await panel.getByLabel("Model ID", { exact:true }).inputValue(), "manual-only-model");
      await panel.getByLabel("API base URL", { exact:true }).fill("https://provider.example/v1");
      await panel.getByRole("checkbox").check();
      modelGate = new Promise(resolve => { releaseModel = resolve; });
      const revokeStarted = new Promise(resolve => { beginModel = resolve; });
      const revokeDelivered = new Promise(resolve => { finishModel = resolve; });
      await panel.getByRole("button", { name:"Test connection / load models", exact:true }).click();
      await withinDeadline(revokeStarted);
      await panel.getByRole("checkbox").uncheck();
      assert.equal(await panel.getByRole("button", { name:"Test connection / load models", exact:true }).isDisabled(), true);
      releaseModel(); await withinDeadline(revokeDelivered);
      assert.equal(await panel.getByLabel("Available models", { exact:true }).count(), 0, "revoked trust must reject an in-flight list");
      assert.equal(await panel.getByRole("button", { name:"Cancel", exact:true }).count(), 0, "revocation stops the pending UI immediately");
      assert.equal(generation, 1, "connection checks must never generate a completion");
      const stored = await page.evaluate(() => JSON.stringify([localStorage, sessionStorage]));
      assert.ok(!stored.includes("private-secret-must-not-be-shown"));
      const overflow = await panel.evaluate(element => ({ client:element.clientWidth, scroll:element.scrollWidth, right:element.getBoundingClientRect().right }));
      assert.ok(overflow.scroll <= overflow.client + 1 && overflow.right <= width + 1, JSON.stringify(overflow));
    } catch (error) {
      await page.screenshot({ path:`/tmp/kyuubiki-model-connection-${width}.png` });
      console.error(await page.getByTestId("assistant-model-connection").evaluate(element => ({
        text:element.innerText,
        detailsOpen:element.querySelector("details")?.open,
        labels:[...element.querySelectorAll("label")].map(label => ({ text:label.textContent, height:label.getBoundingClientRect().height })),
      })).catch(() => "panel not mounted"));
      throw error;
    } finally { releaseModel?.(); await context.close(); }
  });
}
