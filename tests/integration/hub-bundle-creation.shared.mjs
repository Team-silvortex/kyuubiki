import assert from "node:assert/strict";
import { assertLanguageChange, assertNoPageErrors } from "./desktop-shell-regression.shared.mjs";
import { bundleCreationCopy } from "../../apps/hub-gui/ui/hub-bundle-creation-copy.js";

async function creationCount(page) {
  return page.evaluate(() => window.__mockInvocations.filter((entry) =>
    entry.command === "guarded_mutation_action" && entry.payload?.payload?.action === "project_bundle_create").length);
}

async function setPickerReply(page, value, error) {
  await page.evaluate((reply) => { window.__bundlePickerReply = reply; }, { value, error });
}

async function assertDialogFits(page) {
  const bounds = await page.evaluate(() => {
    const dialog = document.querySelector("#bundle-create-dialog");
    const rect = dialog.getBoundingClientRect();
    const name = dialog.querySelector("#bundle-create-name").getBoundingClientRect();
    const directoryRow = dialog.querySelector(".hub-bundle-directory-row").getBoundingClientRect();
    const fields = Array.from(dialog.querySelectorAll("input, button, output")).map((element) => {
      const box = element.getBoundingClientRect();
      return { id: element.id, left: box.left, right: box.right };
    });
    return {
      left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom,
      width: innerWidth, height: innerHeight,
      client: dialog.clientWidth, scroll: dialog.scrollWidth, fields,
      nameWidth: name.width, directoryRowWidth: directoryRow.width,
    };
  });
  assert.ok(bounds.left >= 0 && bounds.right <= bounds.width && bounds.top >= 0 && bounds.bottom <= bounds.height);
  assert.ok(bounds.scroll <= bounds.client + 1, "the create dialog must not require horizontal scrolling");
  assert.ok(Math.abs(bounds.nameWidth - bounds.directoryRowWidth) < 2, "the location row should use the full form width");
  for (const field of bounds.fields) {
    assert.ok(field.left >= bounds.left && field.right <= bounds.right, `${field.id} must fit the dialog`);
  }
}

export async function assertBundleCreationRegression(page) {
  await page.waitForSelector('html[data-hub-ready="true"]');
  await page.evaluate(() => {
    const original = window.__TAURI__.core.invoke;
    window.__bundlePickerReply = { value: null };
    window.__bundleCreateMode = "fail";
    window.__TAURI__.core.invoke = async (command, payload) => {
      if (command === "project_bundle_pick_path") {
        window.__mockInvocations.push({ command, payload });
        if (window.__bundlePickerReply.error) throw new Error(window.__bundlePickerReply.error);
        return window.__bundlePickerReply.value;
      }
      if (command === "guarded_mutation_action" && payload?.payload?.action === "project_bundle_create") {
        window.__mockInvocations.push({ command, payload });
        if (window.__bundleCreateMode === "fail") throw new Error("refusing to overwrite existing project bundle");
        await new Promise((resolve) => { window.__releaseBundleCreation = resolve; });
        return JSON.stringify({ created: true, path: payload.payload.path,
          summary: { project_id: "new-study", project_name: payload.payload.name, schema: "kyuubiki.project/v2" } });
      }
      return original(command, payload);
    };
  });
  const unexpectedConfirmations = [];
  page.on("dialog", async (dialog) => {
    unexpectedConfirmations.push(dialog.message());
    await dialog.dismiss();
  });

  await page.locator("#projects-tab-bundles").click();
  for (const id of ["bundle-advanced-tools", "bundle-result-details", "bundle-history-details"]) {
    assert.equal(await page.locator(`#${id}`).evaluate((element) => element.open), false, id);
  }
  await setPickerReply(page, "/tmp/existing.kyuubiki");
  await page.locator("#bundles-action-browse").click();
  await page.waitForFunction(() => document.querySelector("#project-bundle-path").value === "/tmp/existing.kyuubiki");
  await setPickerReply(page, null);
  await page.locator("#bundles-action-browse").click();
  assert.equal(await page.locator("#project-bundle-path").inputValue(), "/tmp/existing.kyuubiki");
  assert.equal(await creationCount(page), 0, "choosing an existing bundle never creates another file");

  await page.locator("#bundles-action-create").click();
  assert.equal(await page.locator("#bundle-create-directory").inputValue(), "/tmp/");
  assert.equal(await page.locator("#bundle-create-submit").isDisabled(), true);
  await page.locator("#bundle-create-name").fill("../escape");
  assert.equal(await page.locator("#bundle-create-submit").isDisabled(), true);
  assert.equal(await page.locator("#bundle-create-name").getAttribute("aria-invalid"), "true");
  await page.locator("#bundle-create-name").fill("材料研究 Study");
  await page.locator("#bundle-create-directory").fill("relative/location");
  assert.equal(await page.locator("#bundle-create-submit").isDisabled(), true);
  await page.locator("#bundle-create-browse").click();
  assert.equal(await page.locator("#bundle-create-directory").inputValue(), "relative/location", "picker cancellation retains the draft");
  await setPickerReply(page, null, "native chooser unavailable");
  await page.locator("#bundle-create-browse").click();
  await page.waitForFunction(() => document.querySelector("#bundle-create-error").textContent.includes("native chooser unavailable"));
  assert.equal(await page.locator("#bundle-create-name").inputValue(), "材料研究 Study");
  await setPickerReply(page, "/tmp/Research space");
  await page.locator("#bundle-create-browse").click();
  await page.waitForFunction(() => !document.querySelector("#bundle-create-submit").disabled);
  assert.equal(await page.locator("#bundle-create-target").textContent(), "/tmp/Research space/材料研究 Study.kyuubiki");
  await assertDialogFits(page);
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#bundle-create-dialog").isVisible(), false);
  assert.equal(await page.locator("#project-bundle-path").inputValue(), "/tmp/existing.kyuubiki");
  assert.equal(await creationCount(page), 0);

  await page.locator("#bundles-action-create").click();
  assert.equal(await page.locator("#bundle-create-name").inputValue(), "材料研究 Study");
  await page.locator("#bundle-create-submit").click();
  await page.waitForFunction(() => document.querySelector("#bundle-create-error").textContent.includes("refusing to overwrite"));
  assert.equal(await creationCount(page), 1);
  assert.equal(await page.locator("#bundle-create-dialog").isVisible(), true);
  assert.equal(await page.locator("#project-bundle-path").inputValue(), "/tmp/existing.kyuubiki");
  assert.equal(await page.locator("#bundle-create-name").inputValue(), "材料研究 Study");

  await page.locator("#bundle-create-name").fill("材料研究 Retry.KYUUBIKI");
  await page.evaluate(() => { window.__bundleCreateMode = "success"; });
  await page.locator("#bundle-create-name").press("Enter");
  await page.waitForFunction(() => typeof window.__releaseBundleCreation === "function");
  assert.equal(await page.locator("#bundle-create-cancel").isDisabled(), true);
  await page.evaluate(() => {
    document.querySelector("#bundle-create-form").requestSubmit();
    document.querySelector("#bundle-create-form").requestSubmit();
  });
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#bundle-create-dialog").isVisible(), true, "a pending creation cannot be dismissed and lose its outcome");
  assert.equal(await creationCount(page), 2, "one failed attempt and one retry, never duplicate submissions");
  await page.evaluate(() => window.__releaseBundleCreation());
  await page.waitForSelector("#bundle-create-dialog", { state: "hidden" });
  assert.equal(await page.locator("#project-bundle-path").inputValue(), "/tmp/Research space/材料研究 Retry.kyuubiki");
  assert.equal(await page.locator("#bundle-result-details").evaluate((element) => element.open), false);
  const request = await page.evaluate(() => window.__mockInvocations.filter((entry) =>
    entry.payload?.payload?.action === "project_bundle_create").at(-1).payload.payload);
  assert.equal(request.parentPath, "/tmp/Research space");
  assert.equal(request.name, "材料研究 Retry");
  assert.deepEqual(unexpectedConfirmations, [], "the explicit create form is the only confirmation");

  for (const language of ["de", "ar", "zh"]) {
    await assertLanguageChange(page, language);
    await page.waitForFunction((title) => document.querySelector("#bundles-action-create").textContent === title,
      bundleCreationCopy(language).create);
    await page.locator("#bundles-action-create").click();
    await page.locator("#bundle-create-name").fill("Very long research project ".repeat(5));
    await page.locator("#bundle-create-directory").fill(`/tmp/${"Directory with spaces/".repeat(10)}`);
    await page.setViewportSize({ width: 390, height: 700 });
    await assertDialogFits(page);
    await page.locator("#bundle-create-cancel").click();
    await page.setViewportSize({ width: 1180, height: 920 });
  }
  assert.equal(await creationCount(page), 2, "cancelled dialogs and delayed fallback must not create files");
  await assertNoPageErrors(page);
}
