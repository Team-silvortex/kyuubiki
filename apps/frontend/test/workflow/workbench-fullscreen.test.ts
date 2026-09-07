import assert from "node:assert/strict";
import test from "node:test";
import {
  exitWorkbenchViewportFullscreen,
  isWorkbenchViewportFullscreen,
} from "../../src/components/workbench/workbench-fullscreen.ts";

test("empty viewport references never own fullscreen during WebView startup", async () => {
  for (const target of [null, undefined]) {
    for (const fullscreenElement of [null, undefined]) {
      const document = { fullscreenElement };
      assert.equal(isWorkbenchViewportFullscreen(document, target), false);
      await exitWorkbenchViewportFullscreen(document, target);
    }
  }
});

test("fullscreen exit tolerates a WebView without the standard API", async () => {
  const target = {} as Element;
  await exitWorkbenchViewportFullscreen({}, target);
  await exitWorkbenchViewportFullscreen({ fullscreenElement: target }, target);
});

test("fullscreen exit acts only on the actual viewport and retains the document receiver", async () => {
  const target = {} as Element;
  let exits = 0;
  const document = {
    fullscreenElement: target,
    exitFullscreen() {
      assert.equal(this, document);
      exits += 1;
    },
  };
  assert.equal(isWorkbenchViewportFullscreen(document, target), true);
  await exitWorkbenchViewportFullscreen(document, null);
  await exitWorkbenchViewportFullscreen(document, {} as Element);
  assert.equal(exits, 0);
  await exitWorkbenchViewportFullscreen(document, target);
  assert.equal(exits, 1);
});

test("fullscreen exit converts synchronous and asynchronous API errors into catchable rejections", async () => {
  const target = {} as Element;
  for (const exitFullscreen of [
    () => { throw new Error("fullscreen denied"); },
    () => Promise.reject(new Error("fullscreen denied")),
  ]) {
    await assert.rejects(
      exitWorkbenchViewportFullscreen({ fullscreenElement: target, exitFullscreen }, target),
      /fullscreen denied/,
    );
  }
});
