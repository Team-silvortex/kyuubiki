const PLAYWRIGHT_RESTRICTED_HINTS = [
  "mach_port_rendezvous_mac",
  "bootstrap_check_in",
  "permission denied (1100)",
  "Target page, context or browser has been closed",
];

const RESTRICTED_PLAYWRIGHT_MESSAGE = "Playwright browser launch is restricted in this environment.";

function isMissingPlaywrightExecutable(error) {
  const message = error instanceof Error ? error.message : String(error ?? "");
  return message.includes("Executable doesn't exist");
}

export async function launchPlaywrightChromium(chromium, options = { headless: true }) {
  const executablePath = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH?.trim();
  if (executablePath) return chromium.launch({ ...options, executablePath });

  try {
    return await chromium.launch(options);
  } catch (error) {
    if (!isMissingPlaywrightExecutable(error)) throw error;
  }

  let lastError;
  for (const channel of ["chrome", "msedge"]) {
    try {
      return await chromium.launch({ ...options, channel });
    } catch (error) {
      lastError = error;
    }
  }
  throw lastError ?? new Error("No compatible Playwright Chromium executable was found.");
}

export function isRestrictedPlaywrightLaunchError(error) {
  const message = error instanceof Error ? error.message : String(error ?? "");
  const stack = error instanceof Error ? error.stack ?? "" : "";
  const text = `${message}\n${stack}`;
  return PLAYWRIGHT_RESTRICTED_HINTS.some((hint) => text.includes(hint));
}

export function buildRestrictedPlaywrightLaunchError(error, detail) {
  const friendly = new Error(
    detail ? `${RESTRICTED_PLAYWRIGHT_MESSAGE} ${detail}` : RESTRICTED_PLAYWRIGHT_MESSAGE,
    { cause: error instanceof Error ? error : undefined },
  );
  friendly.code = "PLAYWRIGHT_RESTRICTED";
  return friendly;
}

export function reportRestrictedPlaywrightSkip(label, error) {
  const detail = error instanceof Error ? error.message : String(error ?? "unknown launch failure");
  const summary = detail.split("\n")[0]?.trim() || "unknown launch failure";
  console.warn(`${label} skipped: Playwright browser launch is restricted in this environment.`);
  console.warn(summary);
}
