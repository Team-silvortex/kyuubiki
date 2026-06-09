import { findAutomationActionContract } from "./kyuubiki-automation-actions.mjs";
import { createPlaywrightExecutor } from "./kyuubiki-playwright-executor.mjs";
import { createServiceExecutor } from "./kyuubiki-service-executor.mjs";

export async function createHybridAutomationExecutor(options = {}) {
  const service = await createServiceExecutor({ baseUrl: options.apiBaseUrl });
  let browser = null;

  return {
    artifactsDir: null,
    executor: async (step) => {
      const contract = findAutomationActionContract(step.action);
      if (contract?.engine === "service") {
        return service.executor(step);
      }
      if (!browser) {
        browser = await createPlaywrightExecutor({
          artifactsDir: options.artifactsDir,
        });
      }
      return browser.executor(step);
    },
    dispose: async () => {
      if (browser) await browser.dispose();
      await service.dispose();
    },
    getArtifactsDir: () => browser?.artifactsDir ?? null,
    getApiBaseUrl: () => service.baseUrl ?? null,
  };
}
