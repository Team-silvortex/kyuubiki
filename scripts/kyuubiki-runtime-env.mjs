import fs from "node:fs";
import path from "node:path";
import process from "node:process";

export function createRuntimeEnv({ rootDir, envFile, envExampleFile }) {
  let envCache = null;

  function loadEnvValues() {
    if (envCache) {
      return envCache;
    }

    const values = {};
    for (const envPath of [envExampleFile, envFile]) {
      if (!fs.existsSync(envPath)) {
        continue;
      }

      const lines = fs.readFileSync(envPath, "utf8").split(/\r?\n/u);
      for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line || line.startsWith("#")) {
          continue;
        }
        const separator = line.indexOf("=");
        if (separator <= 0) {
          continue;
        }
        const key = line.slice(0, separator).trim();
        const value = line.slice(separator + 1).trim();
        values[key] = value;
      }
    }

    envCache = values;
    return values;
  }

  function resolveWorkspacePath(value) {
    return path.isAbsolute(value) ? value : path.join(rootDir, value);
  }

  function buildModeEnv(mode) {
    const env = loadEnvValues();
    const merged = { ...env, ...process.env };
    if (mode === "local") {
      merged.KYUUBIKI_STORAGE_BACKEND = "sqlite";
      merged.SQLITE_DATABASE_PATH = env.SQLITE_DATABASE_PATH ?? "./tmp/data/kyuubiki_dev.sqlite3";
      merged.KYUUBIKI_DEPLOYMENT_MODE = "local";
    } else if (mode === "cloud") {
      requireEnv(merged.DATABASE_URL, "DATABASE_URL is required for cloud mode");
      merged.KYUUBIKI_STORAGE_BACKEND = "postgres";
      merged.KYUUBIKI_DEPLOYMENT_MODE = "cloud";
    } else if (mode === "distributed") {
      requireEnv(merged.DATABASE_URL, "DATABASE_URL is required for distributed mode");
      merged.KYUUBIKI_STORAGE_BACKEND = "postgres";
      merged.KYUUBIKI_DEPLOYMENT_MODE = "distributed";
    }
    return merged;
  }

  function storageModeLabel(mode) {
    if (mode === "local") {
      return "sqlite";
    }
    if (mode === "cloud" || mode === "distributed") {
      return "postgres";
    }
    return loadEnvValues().KYUUBIKI_STORAGE_BACKEND ?? "sqlite";
  }

  function deploymentModeLabel(mode) {
    return resolveDeploymentMode(mode);
  }

  function resolveDeploymentMode(mode) {
    if (mode === "local" || mode === "cloud" || mode === "distributed") {
      return mode;
    }
    return loadEnvValues().KYUUBIKI_DEPLOYMENT_MODE ?? "local";
  }

  function controlModeLabel(mode) {
    return resolveDeploymentMode(mode) === "local" ? "standalone" : "orch_managed";
  }

  function authorityModeLabel(mode) {
    return resolveDeploymentMode(mode) === "local" ? "self_directed" : "single_orchestrator";
  }

  function hotModeLabel(mode) {
    return mode === "cloud" || mode === "distributed" ? mode : "local";
  }

  return {
    authorityModeLabel,
    buildModeEnv,
    controlModeLabel,
    deploymentModeLabel,
    hotModeLabel,
    loadEnvValues,
    resolveDeploymentMode,
    resolveWorkspacePath,
    storageModeLabel,
  };
}

function requireEnv(value, message) {
  if (!value) {
    throw new Error(message);
  }
}
