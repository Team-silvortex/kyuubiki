import assert from "node:assert/strict";
import test from "node:test";
import {
  exitWorkbenchViewportFullscreen,
  isWorkbenchViewportFullscreen,
  requestWorkbenchViewportFullscreen,
  getWindowImmersiveTarget,
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

test("window-local fullscreen retains an exit target while Suspense clears a React ref", async () => {
  const attrs = new Map<string, string>(), listeners = new Map<string, EventListener>();
  const target = {
    parentElement: null,
    setAttribute: (key: string, value: string) => attrs.set(key, value),
    removeAttribute: (key: string) => attrs.delete(key),
  } as unknown as HTMLElement;
  const document = {
    activeElement: null,
    addEventListener: (type: string, listener: EventListener) => listeners.set(type, listener),
    removeEventListener: (type: string) => listeners.delete(type),
    dispatchEvent: () => true,
  } as unknown as Document;
  await requestWorkbenchViewportFullscreen(document, target);
  assert.equal(attrs.get("data-workbench-window-fullscreen"), "true");
  assert.equal(getWindowImmersiveTarget(document), target);
  await exitWorkbenchViewportFullscreen(document, getWindowImmersiveTarget(document));
  assert.equal(getWindowImmersiveTarget(document), null);
  assert.equal(attrs.size, 0);
  assert.equal(listeners.size, 0);
});

test("WebKit-prefixed entry and exit retain their receivers and viewport ownership", async () => {
  const document = { webkitFullscreenElement: null as Element | null, webkitExitFullscreen() {
    assert.equal(this, document);
    this.webkitFullscreenElement = null;
  } };
  const target = { webkitRequestFullscreen() {
    assert.equal(this, target);
    document.webkitFullscreenElement = target as unknown as Element;
  } } as unknown as HTMLElement;
  await requestWorkbenchViewportFullscreen(document as unknown as Document, target);
  assert.equal(isWorkbenchViewportFullscreen(document, target), true);
  await exitWorkbenchViewportFullscreen(document, target);
  assert.equal(isWorkbenchViewportFullscreen(document, target), false);
});

test("available but rejected fullscreen APIs do not silently fall back", async () => {
  for (const method of ["requestFullscreen", "webkitRequestFullscreen"]) {
    const target = { [method]: async () => { throw new Error("entry denied"); } } as unknown as HTMLElement;
    await assert.rejects(requestWorkbenchViewportFullscreen({} as Document, target), /entry denied/);
  }
});

test("entry cannot take over another fullscreen element", async () => {
  let called = false;
  const target = { requestFullscreen: async () => { called = true; } } as unknown as HTMLElement;
  await assert.rejects(requestWorkbenchViewportFullscreen({ fullscreenElement: {} } as Document, target), /owned_elsewhere/);
  assert.equal(called, false);
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
