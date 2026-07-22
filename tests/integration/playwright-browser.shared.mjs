function isMissingBundledBrowser(error) {
  const message = String(error?.message || error || "");
  return message.includes("Executable doesn't exist") || message.includes("playwright install");
}

export async function launchIntegrationBrowser(chromium, options = {}) {
  const executablePath = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH?.trim();
  if (executablePath) {
    return chromium.launch({ headless: true, ...options, executablePath });
  }

  try {
    return await chromium.launch({ headless: true, ...options });
  } catch (error) {
    if (!isMissingBundledBrowser(error)) throw error;

    const channel = process.env.KYUUBIKI_PLAYWRIGHT_CHANNEL?.trim() || "chrome";
    return chromium.launch({ headless: true, ...options, channel });
  }
}
