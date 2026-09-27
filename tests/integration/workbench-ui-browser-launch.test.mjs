import assert from "node:assert/strict";
import test from "node:test";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

test("browser launch retries one pre-page libdbus crash without changing options", async (t) => {
  t.mock.method(console, "warn", () => {});
  const calls = [];
  const browser = {};
  const chromium = { launch: async (options) => {
    calls.push(options);
    if (calls.length === 1) throw new Error("browserType.launch: libdbus Received signal 11");
    return browser;
  } };
  const prior = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
  delete process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
  t.after(() => {
    if (prior === undefined) delete process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
    else process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH = prior;
  });
  assert.equal(await launchIntegrationBrowser(chromium, { timeout: 2000 }), browser);
  assert.deepEqual(calls, [
    { headless: true, timeout: 2000 }, { headless: true, timeout: 2000 },
  ]);
});

test("browser launch preserves both startup failures and bounds retry", async (t) => {
  t.mock.method(console, "warn", () => {});
  const prior = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
  delete process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
  t.after(() => {
    if (prior === undefined) delete process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
    else process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH = prior;
  });
  const failures = [new Error("browserType.launch: libdbus SIGSEGV"), new Error("still broken")];
  let calls = 0;
  await assert.rejects(launchIntegrationBrowser({ launch: async () => {
    throw failures[calls++];
  } }), (error) => {
    assert.deepEqual(error.errors, failures);
    assert.equal(error.cause, failures[1]);
    return true;
  });
  assert.equal(calls, 2);
});

test("browser launch does not retry arbitrary errors", async () => {
  let calls = 0;
  const failure = new Error("browserType.launch: launch timed out");
  await assert.rejects(launchIntegrationBrowser({ launch: async () => {
    calls += 1;
    throw failure;
  } }), (error) => error === failure);
  assert.equal(calls, 1);
});

test("browser launch respects an explicit executable and never substitutes it", async (t) => {
  const prior = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
  process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH = "/test/browser";
  t.after(() => {
    if (prior === undefined) delete process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH;
    else process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH = prior;
  });
  const calls = [];
  const failure = new Error("browserType.launch: libdbus SIGSEGV");
  await assert.rejects(launchIntegrationBrowser({ launch: async (options) => {
    calls.push(options);
    throw failure;
  } }), (error) => error === failure);
  assert.deepEqual(calls, [{ headless: true, executablePath: "/test/browser" }]);
});
