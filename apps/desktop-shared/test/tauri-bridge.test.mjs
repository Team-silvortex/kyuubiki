import assert from "node:assert/strict";
import test from "node:test";
import { watchDesktopLanguagePreference } from "../ui/tauri-bridge.js";

test("desktop language watcher preserves the current language across transient IPC failure", async () => {
  const previousDocument = globalThis.document;
  const previousWindow = globalThis.window;
  const windowListeners = new Map();
  const documentListeners = new Map();
  const changes = [];
  let timerCallback;
  let readCount = 0;

  try {
    globalThis.document = {
      visibilityState: "visible",
      addEventListener: (type, listener) => documentListeners.set(type, listener),
      removeEventListener: (type) => documentListeners.delete(type),
    };
    globalThis.window = {
      __TAURI__: {
        core: {
          invoke: async () => {
            readCount += 1;
            if (readCount === 1) throw new Error("temporary language bridge failure");
            return { language: "zh-TW" };
          },
        },
      },
      addEventListener: (type, listener) => windowListeners.set(type, listener),
      removeEventListener: (type) => windowListeners.delete(type),
      setInterval: (callback) => {
        timerCallback = callback;
        return 7;
      },
      clearInterval: () => {},
    };

    const stop = watchDesktopLanguagePreference({
      getCurrentLanguage: () => "fr",
      onChange: (language) => changes.push(language),
      intervalMs: 500,
    });
    timerCallback();
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(changes, []);

    timerCallback();
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(changes, ["zh-TW"]);
    stop();
    assert.deepEqual([...windowListeners.keys()], []);
    assert.deepEqual([...documentListeners.keys()], []);
  } finally {
    globalThis.document = previousDocument;
    globalThis.window = previousWindow;
  }
});
