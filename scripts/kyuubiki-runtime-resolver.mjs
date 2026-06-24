import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const PLATFORM_MAP = {
  darwin: "macos",
  linux: "linux",
  win32: "windows",
};

const COMMAND_RUNTIME = {
  elixir: "elixir-otp",
  mix: "elixir-otp",
  node: "node",
  npm: "node",
};

export function createRuntimeResolver({ rootDir }) {
  let manifestCache = null;

  function manifestPath() {
    const platform = PLATFORM_MAP[process.platform] || process.platform;
    const stagedPath = path.join(rootDir, "dist", platform, "manifests", "embedded-runtimes.json");
    if (fs.existsSync(stagedPath)) {
      return stagedPath;
    }
    return path.join(rootDir, "manifests", "embedded-runtimes.json");
  }

  function loadManifest() {
    if (manifestCache) {
      return manifestCache;
    }
    const candidate = manifestPath();
    if (!fs.existsSync(candidate)) {
      manifestCache = { path: candidate, manifest: null };
      return manifestCache;
    }
    manifestCache = {
      path: candidate,
      manifest: JSON.parse(fs.readFileSync(candidate, "utf8")),
    };
    return manifestCache;
  }

  function runtimeFor(commandName) {
    const runtimeId = COMMAND_RUNTIME[commandName];
    const { manifest } = loadManifest();
    if (!runtimeId || !manifest || !Array.isArray(manifest.runtimes)) {
      return null;
    }
    return manifest.runtimes.find((runtime) => runtime.id === runtimeId) || null;
  }

  function existingBinDirs(runtime) {
    if (!runtime || !Array.isArray(runtime.bin_dirs)) {
      return [];
    }
    return runtime.bin_dirs
      .map((entry) => path.resolve(rootDir, entry))
      .filter((entry) => fs.existsSync(entry));
  }

  function commandCandidates(commandName) {
    if (process.platform !== "win32") {
      return [commandName];
    }
    if (commandName === "npm") {
      return ["npm.cmd", "npm.exe", "npm"];
    }
    if (commandName === "mix") {
      return ["mix.bat", "mix.cmd", "mix"];
    }
    if (commandName === "cargo") {
      return ["cargo.exe", "cargo"];
    }
    if (commandName === "node") {
      return ["node.exe", "node"];
    }
    return [commandName];
  }

  function resolveRuntimeCommand(commandName) {
    const runtime = runtimeFor(commandName);
    const binDirs = existingBinDirs(runtime);
    for (const binDir of binDirs) {
      for (const candidate of commandCandidates(commandName)) {
        const fullPath = path.join(binDir, candidate);
        if (fs.existsSync(fullPath)) {
          return {
            command: fullPath,
            source: "embedded",
            runtimeId: runtime.id,
            manifestPath: loadManifest().path,
          };
        }
      }
    }

    if (process.env.KYUUBIKI_RUNTIME_STRICT === "1" && runtime?.required_for_self_host) {
      throw new Error(
        `required embedded runtime ${runtime.id} is missing command ${commandName}; expected under ${runtime.target_dir}`,
      );
    }

    return {
      command: platformCommand(commandName),
      source: runtime ? "host-fallback" : "host",
      runtimeId: runtime?.id || "host",
      manifestPath: loadManifest().path,
    };
  }

  function runtimePathPrefix() {
    const { manifest } = loadManifest();
    if (!manifest || !Array.isArray(manifest.runtimes)) {
      return [];
    }
    return manifest.runtimes.flatMap(existingBinDirs);
  }

  function withRuntimePath(env) {
    const prefix = runtimePathPrefix();
    if (prefix.length === 0) {
      return env;
    }
    return {
      ...env,
      PATH: [prefix.join(path.delimiter), env.PATH || process.env.PATH || ""]
        .filter(Boolean)
        .join(path.delimiter),
    };
  }

  function renderRuntimeResolution() {
    const { path: activeManifestPath, manifest } = loadManifest();
    const lines = [
      `runtime-manifest: ${manifest ? activeManifestPath : "missing"}`,
      `runtime-policy: ${manifest?.policy?.mode || "host"}`,
    ];
    for (const commandName of ["node", "npm", "mix"]) {
      const resolved = resolveRuntimeCommand(commandName);
      lines.push(
        `runtime-command[${commandName}]: ${resolved.source} ${resolved.runtimeId} -> ${resolved.command}`,
      );
    }
    return lines;
  }

  return {
    renderRuntimeResolution,
    resolveRuntimeCommand,
    withRuntimePath,
  };
}

function platformCommand(name) {
  if (process.platform !== "win32") {
    return name;
  }
  if (name === "npm") {
    return "npm.cmd";
  }
  if (name === "mix") {
    return "mix.bat";
  }
  if (name === "cargo") {
    return "cargo.exe";
  }
  if (name === "node") {
    return "node.exe";
  }
  return name;
}
