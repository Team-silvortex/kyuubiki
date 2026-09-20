import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const tab = (page, name) => page.locator(`[data-workbench-pwdt-page="${name}"]`);
const panel = (page, name) => page.locator(`[data-workbench-pwdt-content="${name}"]`);
const python = (page) => panel(page, "script").locator("textarea");
const translations = [
  { language: "ar", run: "تشغيل البرنامج النصي", load: "تحميل بيئة التشغيل", editor: "برنامج Python النصي",
    idle: "خامل", running: "قيد التشغيل", ready: "جاهز", clear: "مسح المخرجات",
    start: "بدء التسجيل", stop: "إيقاف التسجيل", compile: "تحويل إلى برنامج نصي",
    snippets: "مقاطع الشيفرة", insert: "إدراج بالمعاملات الحالية", invalid: "صيغة JSON للمعاملات غير صالحة.",
    timeline: "الخط الزمني", emptyPresets: "لا توجد إعدادات مسبقة محفوظة للمشروع الحالي بعد." },
  { language: "fa", run: "اجرای اسکریپت", load: "بارگذاری محیط اجرا", editor: "اسکریپت Python",
    idle: "بیکار", running: "در حال اجرا", ready: "آماده", clear: "پاک کردن خروجی",
    start: "شروع ضبط", stop: "توقف ضبط", compile: "کامپایل به اسکریپت",
    snippets: "قطعه‌کدها", insert: "درج با پارامترهای فعلی", invalid: "پارامترهای JSON نامعتبر هستند.",
    timeline: "خط زمانی", emptyPresets: "هنوز پیش‌تنظیمی برای پروژهٔ فعلی ذخیره نشده است." },
  { language: "es", run: "Ejecutar script", load: "Cargar entorno de ejecución", editor: "Script de Python",
    idle: "Inactivo", running: "En ejecución", ready: "Listo", clear: "Limpiar salida",
    start: "Iniciar grabación", stop: "Detener grabación", compile: "Compilar a script",
    snippets: "Fragmentos", insert: "Insertar con estos parámetros", invalid: "El JSON de parámetros no es válido.",
    timeline: "Cronología", emptyPresets: "Aún no hay preajustes guardados para el proyecto actual." },
];

async function openPwdt(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.locator('[data-workbench-system-surface-tab="settings"]').click();
  await page.locator('[data-workbench-system-settings-page="scripts"]').click();
  await page.locator('[data-workbench-pwdt="workspace"]').waitFor();
}

for (const copy of translations) {
test(`${copy.language} PWDT localizes controls while preserving running scripts, recordings and raw output`, { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    let downloads = 0;
    // Test UI state/language dispatch, not Python or WASM execution.
    await page.route("https://cdn.jsdelivr.net/pyodide/**/pyodide.js", (route) => {
      downloads += 1;
      return route.fulfill({ contentType: "application/javascript", body: `window.loadPyodide = async () => ({
        runPythonAsync: async source => {
          window.pwdtSources = [...(window.pwdtSources || []), source];
          window.__kyuubikiBridge.log("raw-user-output: result_id=123");
          await new Promise(resolve => { window.releasePwdtRun = resolve; });
          window.__kyuubikiBridge.log("raw-user-output: finished");
        }
      });` });
    });
    await openWorkbench(page);
    await invoke(page, "settings/patch", { language: copy.language });
    await openPwdt(page);
    const source = "ky.log('retained code: العربية فارسی español')";
    await python(page).fill(source);
    assert.equal(await python(page).getAttribute("aria-label"), copy.editor);
    assert.equal(await python(page).evaluate((input) => getComputedStyle(input).direction), "ltr");
    assert.equal(await panel(page, "script").locator(".status-chip").innerText(), copy.idle);
    assert.equal(await panel(page, "headless").locator("input").count(), 0, "SDK bridge remains lazily mounted");
    assert.equal(downloads, 0, "opening PWDT does not download Pyodide");
    await panel(page, "script").getByRole("button", { name: copy.load, exact: true }).waitFor();
    await panel(page, "script").getByRole("button", { name: copy.run, exact: true }).click();
    await page.waitForFunction(() => typeof window.releasePwdtRun === "function");
    assert.equal(await panel(page, "script").locator(".status-chip").innerText(), copy.running);
    const recordedOutput = await panel(page, "script").locator(".script-panel__output").innerText();
    await invoke(page, "settings/patch", { language: "en" });
    await panel(page, "script").getByRole("button", { name: "Run script", exact: true }).waitFor();
    assert.equal(await panel(page, "script").getByRole("button", { name: "Run script", exact: true }).isDisabled(), true);
    assert.equal(await panel(page, "script").locator(".status-chip").innerText(), "Running");
    assert.equal(await python(page).inputValue(), source);
    assert.equal(await panel(page, "script").locator(".script-panel__output").innerText(), recordedOutput);
    await invoke(page, "settings/patch", { language: copy.language });
    await panel(page, "script").getByRole("button", { name: copy.run, exact: true }).waitFor();
    assert.equal(await panel(page, "script").getByRole("button", { name: copy.run, exact: true }).isDisabled(), true);
    await page.evaluate(() => window.releasePwdtRun());
    await panel(page, "script").locator(".status-chip--good").waitFor();
    assert.equal(await panel(page, "script").locator(".status-chip").innerText(), copy.ready);
    assert.match(await panel(page, "script").innerText(), /raw-user-output: finished/);
    assert.equal(await page.evaluate(() => window.pwdtSources.length), 1, "locale changes must not rerun scripts");
    assert.equal(downloads, 1);
    await tab(page, "record").click();
    await panel(page, "record").getByRole("button", { name: copy.start, exact: true }).click();
    await invoke(page, "settings/patch", { language: "en" });
    await panel(page, "record").getByRole("button", { name: "Stop recording", exact: true }).waitFor();
    await invoke(page, "settings/patch", { language: copy.language });
    await panel(page, "record").getByRole("button", { name: copy.stop, exact: true }).click();
    await tab(page, "inspect").click();
    await panel(page, "inspect").getByRole("button", { name: copy.timeline, exact: true }).click();
    await tab(page, "dsl").click();
    await panel(page, "dsl").getByRole("button", { name: copy.compile, exact: true }).waitFor();
    await panel(page, "dsl").locator("textarea").fill('{"draft":"preserved DSL"}');
    await tab(page, "catalog").click();
    await panel(page, "catalog").getByText(copy.emptyPresets, { exact: true }).waitFor();
    await panel(page, "catalog").getByRole("button", { name: copy.snippets, exact: true }).click();
    const snippet = panel(page, "catalog").locator("article").filter({ has: page.locator("textarea") }).first();
    await snippet.locator("textarea").fill("{broken");
    await snippet.getByRole("button", { name: copy.insert, exact: true }).click();
    await snippet.getByText(copy.invalid, { exact: true }).waitFor();
    await tab(page, "script").click();
    assert.equal(await python(page).inputValue(), source, "invalid snippet parameters do not overwrite the script");
    await panel(page, "script").getByRole("button", { name: copy.clear, exact: true }).click();
    assert.equal(await panel(page, "script").locator(".script-panel__output").count(), 0);
    await tab(page, "dsl").click();
    assert.equal(await panel(page, "dsl").locator("textarea").inputValue(), '{"draft":"preserved DSL"}');
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await openPwdt(page);
    await panel(page, "script").getByRole("button", { name: copy.run, exact: true }).waitFor();
    assert.equal(await python(page).inputValue(), source);
    assert.equal(downloads, 1, "reload does not eagerly initialize Python");
    assert.deepEqual(library.writes, [], "localization testing does not submit research models");
  });
});
}
