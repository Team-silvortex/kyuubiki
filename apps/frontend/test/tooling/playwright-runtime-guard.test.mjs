import test from "node:test";
import assert from "node:assert/strict";

import { launchPlaywrightChromium } from "../../scripts/playwright-runtime-guard.mjs";

test("Playwright launcher falls back to a system Chrome channel", async () => {
  const calls = [];
  const chromium = {
    async launch(options) {
      calls.push(options);
      if (!options.channel) throw new Error("Executable doesn't exist at bundled/chromium");
      if (options.channel === "chrome") return { channel: "chrome" };
      throw new Error("unexpected channel");
    },
  };

  const browser = await launchPlaywrightChromium(chromium, { headless: true });

  assert.deepEqual(browser, { channel: "chrome" });
  assert.deepEqual(calls, [
    { headless: true },
    { headless: true, channel: "chrome" },
  ]);
});

test("Playwright launcher honors an explicit executable path", async () => {
  const previous = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
  process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH = "/opt/kyuubiki/chrome";
  const calls = [];
  const chromium = {
    async launch(options) {
      calls.push(options);
      return { executablePath: options.executablePath };
    },
  };

  try {
    const browser = await launchPlaywrightChromium(chromium, { headless: false });
    assert.deepEqual(browser, { executablePath: "/opt/kyuubiki/chrome" });
    assert.deepEqual(calls, [{
      headless: false,
      executablePath: "/opt/kyuubiki/chrome",
    }]);
  } finally {
    if (previous === undefined) delete process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
    else process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH = previous;
  }
});
