import assert from "node:assert/strict";
import test from "node:test";

import {
  assertActionInvokes,
  assertNoPageErrors,
  assertTauriInvocations,
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

test(
  "Hub-to-Installer deployment update closes handoff, source, download, apply, and integrity layers",
  async () => {
    const environment = await createDesktopShellRegressionEnvironment();
    let browser;

    try {
      browser = await launchIntegrationBrowser(chromium);
      const hub = await browser.newPage();
      await hub.goto(environment.hubUrl, { waitUntil: "networkidle", timeout: 60_000 });
      await hub.waitForSelector('html[data-hub-ready="true"] .hub-nav__item[data-target="deploy"]');
      await hub.locator('.hub-nav__item[data-target="deploy"]').click();
      await hub.locator('[data-panel-page-group="deploy"][data-panel-page="bootstrap"]').click();
      await assertActionInvokes(
        hub,
        "open-installer",
        "launch_installer_gui",
        undefined,
        "#deploy-bootstrap-stage",
        { acceptConfirmation: true },
      );
      await assertNoPageErrors(hub);
      await hub.close();

      const installer = await browser.newPage();
      await installer.goto(environment.installerUrl, { waitUntil: "networkidle", timeout: 60_000 });
      await installer.locator('button.sidebar-tab[data-tab="updates"]').click();
      await installer.waitForSelector('[data-panel="updates"].panel-visible #update-state-headline');
      await installer.locator('[data-installer-section-target="source"]').click();
      await assertActionInvokes(
        installer,
        "save-update-source",
        "guarded_mutation_action",
        "write_update_source_config",
      );
      await assertActionInvokes(
        installer,
        "download-update",
        "guarded_mutation_action",
        "download_update",
        undefined,
        { acceptConfirmation: true },
      );

      await installer.locator('[data-installer-section-target="delivery"]').click();
      assert.equal(await installer.locator("#downloaded-update-version").textContent(), "2.7.1");
      assert.equal(await installer.locator("#apply-downloaded-update-button").isEnabled(), true);
      await assertActionInvokes(
        installer,
        "apply-downloaded-update",
        "guarded_mutation_action",
        "apply_downloaded_update",
        undefined,
        { acceptConfirmation: true },
      );
      assert.equal(await installer.locator("#applied-update-version").textContent(), "2.7.1");
      assert.match(
        await installer.locator("#applied-update-source-manifest").textContent(),
        /downloaded-update\.json/u,
      );

      await installer.locator('button.sidebar-tab[data-tab="integrity"]').click();
      await assertActionInvokes(installer, "refresh-integrity", "installation_integrity_report");
      await assertTauriInvocations(installer, [
        "update_source_config",
        "latest_downloaded_update_record",
        "latest_applied_update_record",
        "installation_integrity_report",
      ]);
      await assertNoPageErrors(installer);
      await installer.close();
    } finally {
      await browser?.close();
      await environment.cleanup();
    }
  },
  { timeout: 120_000 },
);
