import assert from "node:assert/strict";
import test from "node:test";
import {
  assertNoPageErrors,
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import {
  createWorkbenchRegressionEnvironment,
} from "./workbench-shell-regression.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

const LIFECYCLE = {
  hub: {
    last: "__kyuubikiHubLastAction",
    status: "__kyuubikiHubActionStatus",
  },
  installer: {
    last: "__kyuubikiInstallerLastAction",
    status: "__kyuubikiInstallerActionStatus",
  },
  workbench: {
    last: "__kyuubikiWorkbenchLastAction",
    status: "__kyuubikiWorkbenchActionStatus",
  },
};

async function declarativeActions(page) {
  return page.evaluate(() =>
    [...new Set(
      [...document.querySelectorAll("[data-action]")]
        .map((element) => element.dataset.action)
        .filter(Boolean),
    )].sort(),
  );
}

async function sweepDeclarativeActions(page, shell) {
  const actions = await declarativeActions(page);
  const lifecycle = LIFECYCLE[shell];
  const results = [];
  page.on("dialog", (dialog) => dialog.accept());

  for (const action of actions) {
    const trigger = await page.evaluate((currentAction) => {
      const element = [...document.querySelectorAll("[data-action]")]
        .find((candidate) => candidate.dataset.action === currentAction);
      if (!element) return { found: false, disabled: false };
      if (element.disabled) return { found: true, disabled: true };
      element.click();
      return { found: true, disabled: false };
    }, action);
    assert.equal(trigger.found, true, `${shell} action disappeared before dispatch: ${action}`);
    if (trigger.disabled) {
      results.push({ action, status: "blocked" });
      continue;
    }
    try {
      await page.waitForFunction(
        ({ currentAction, lastKey, statusKey }) => {
          const status = window[statusKey];
          return window[lastKey] === currentAction &&
            ["completed", "blocked", "cancelled", "failed", "missing"].includes(status);
        },
        {
          currentAction: action,
          lastKey: lifecycle.last,
          statusKey: lifecycle.status,
        },
        { timeout: 8_000 },
      );
    } catch (error) {
      throw new Error(`${shell} action did not settle: ${action}; ${String(error)}`);
    }
    results.push(await page.evaluate(
      ({ currentAction, statusKey }) => ({
        action: currentAction,
        status: window[statusKey],
      }),
      { currentAction: action, statusKey: lifecycle.status },
    ));
  }

  assert.ok(actions.length > 0, `${shell} should expose declarative actions`);
  assert.deepEqual(
    results.filter((entry) => entry.status === "missing"),
    [],
    `${shell} contains isolated declarative actions`,
  );
  return results;
}

test(
  "all desktop declarative actions reach an observable terminal state",
  async (t) => {
    const desktop = await createDesktopShellRegressionEnvironment();
    const workbench = await createWorkbenchRegressionEnvironment();
    let browser;

    try {
      browser = await launchIntegrationBrowser(chromium);
      const shells = [
        ["hub", desktop.hubUrl],
        ["installer", desktop.installerUrl],
        ["workbench", workbench.workbenchUrl],
      ];
      for (const [shell, url] of shells) {
        await t.test(`${shell} action sweep`, async () => {
          const page = await browser.newPage();
          try {
            await page.goto(url, { waitUntil: "networkidle", timeout: 60_000 });
            const results = await sweepDeclarativeActions(page, shell);
            const failed = results.filter((entry) => entry.status === "failed");
            const blocked = results.filter((entry) => entry.status === "blocked");
            assert.deepEqual(failed, [], `${shell} actions should complete against the valid GUI fixture`);
            t.diagnostic(
              `${shell}: ${results.length} actions, ${blocked.length} explicitly blocked by preconditions`,
            );
            await assertNoPageErrors(page);
          } finally {
            await page.close();
          }
        });
      }
    } finally {
      await browser?.close();
      await Promise.all([desktop.cleanup(), workbench.cleanup()]);
    }
  },
  { timeout: 180_000 },
);
