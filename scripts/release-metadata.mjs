import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

export const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
export const deployDir = path.join(rootDir, "deploy");
export const releasesDir = path.join(rootDir, "releases");
export const snapshotsDir = path.join(releasesDir, "snapshots");

export const releaseIndexPath = path.join(releasesDir, "index.json");
export const updateChannelsPath = path.join(deployDir, "update-channels.json");
export const installationIntegrityContractPath = path.join(
  deployDir,
  "installation-integrity-contract.json",
);
export const updateCatalogPath = path.join(releasesDir, "update-catalog.json");
export const updateCatalogDocPaths = [
  path.join(rootDir, "docs/update-catalog.html"),
  path.join(rootDir, "apps/hub-gui/ui/docs/update-catalog.html"),
];

export const installationIntegrityDocs = {
  docsOutputPath: path.join(rootDir, "docs/installation-integrity-contract.html"),
  hubOutputPath: path.join(rootDir, "apps/hub-gui/ui/docs/installation-integrity.html"),
  docsCssHref: "../apps/hub-gui/ui/docs/docs.css",
  hubCssHref: "./docs.css",
  docsJsonHref: "../deploy/installation-integrity-contract.json",
  hubJsonHref: "../../../../deploy/installation-integrity-contract.json",
};

export function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

export function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

export function writeText(filePath, value) {
  fs.writeFileSync(filePath, value);
}

export function fileExists(relativePath) {
  return fs.existsSync(path.join(rootDir, relativePath));
}

export function git(args) {
  return execFileSync("git", args, { cwd: rootDir, encoding: "utf8" }).trim();
}

export function gitStatusLines() {
  const output = git(["status", "--short"]);
  if (!output) {
    return [];
  }

  return output
    .split("\n")
    .map((line) => line.trimEnd())
    .filter(Boolean);
}

export function isoDate() {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function packageVersion(relativePath) {
  return readJson(path.join(rootDir, relativePath)).version;
}

export function releaseLineLabel(codename, line) {
  return `${codename} ${line}`;
}

export function snapshotFilePath(version) {
  return path.join(snapshotsDir, `${version}.json`);
}

export function snapshotRelativePath(version) {
  return `snapshots/${version}.json`;
}

export function updateVersionAliases(codename, version) {
  const [major = version, minor = version] = version.split(".");
  return [
    `${codename}:latest`,
    `${codename}:${major}`,
    `${codename}:${major}.${minor}`,
    `${codename}:${version}`,
  ];
}

export function readDesktopBundleVersion(appRelativePath) {
  const infoPlistPath = path.join(rootDir, appRelativePath, "Contents", "Info.plist");
  if (!fs.existsSync(infoPlistPath)) {
    return null;
  }

  try {
    const shortVersion = execFileSync(
      "plutil",
      ["-extract", "CFBundleShortVersionString", "raw", "-o", "-", infoPlistPath],
      { cwd: rootDir, encoding: "utf8" },
    ).trim();
    const buildVersion = execFileSync(
      "plutil",
      ["-extract", "CFBundleVersion", "raw", "-o", "-", infoPlistPath],
      { cwd: rootDir, encoding: "utf8" },
    ).trim();

    return {
      short_version: shortVersion || null,
      build_version: buildVersion || null,
      source: path.relative(rootDir, infoPlistPath),
    };
  } catch {
    return null;
  }
}

export function desktopArtifactPaths(version) {
  return {
    hub_app: "apps/hub-gui/src-tauri/target/release/bundle/macos/Kyuubiki Hub.app",
    hub_dmg: `apps/hub-gui/src-tauri/target/release/bundle/dmg/Kyuubiki Hub_${version}_aarch64.dmg`,
    hub_macos_manifest: "dist/macos/desktop/hub-gui/manifest.json",
    hub_linux_manifest: "dist/linux/desktop/hub-gui/manifest.json",
    hub_windows_manifest: "dist/windows/desktop/hub-gui/manifest.json",
    workbench_app: "apps/workbench-gui/src-tauri/target/release/bundle/macos/Kyuubiki Workbench.app",
    workbench_dmg: `apps/workbench-gui/src-tauri/target/release/bundle/dmg/Kyuubiki Workbench_${version}_aarch64.dmg`,
    workbench_macos_manifest: "dist/macos/desktop/workbench-gui/manifest.json",
    workbench_linux_manifest: "dist/linux/desktop/workbench-gui/manifest.json",
    workbench_windows_manifest: "dist/windows/desktop/workbench-gui/manifest.json",
    installer_app: "apps/installer-gui/src-tauri/target/release/bundle/macos/Kyuubiki Installer.app",
    installer_dmg: `apps/installer-gui/src-tauri/target/release/bundle/dmg/Kyuubiki Installer_${version}_aarch64.dmg`,
    installer_macos_manifest: "dist/macos/desktop/installer-gui/manifest.json",
    installer_linux_manifest: "dist/linux/desktop/installer-gui/manifest.json",
    installer_windows_manifest: "dist/windows/desktop/installer-gui/manifest.json",
  };
}

export function syncCurrentReleaseContracts({ version, codename, line }) {
  const channels = readJson(updateChannelsPath);
  const contract = readJson(installationIntegrityContractPath);
  const lineLabel = releaseLineLabel(codename, line);
  const aliases = updateVersionAliases(codename, version);

  const nextChannels = {
    ...channels,
    shipping_version: version,
    line: lineLabel,
    channels: (channels.channels ?? []).map((channel) => {
      if (channel.id !== (channels.default_channel ?? "stable")) {
        return channel;
      }

      return {
        ...channel,
        version,
        aliases,
      };
    }),
  };

  const nextContract = {
    ...contract,
    product_line: lineLabel,
    shipping_version: version,
    workspace_version: version,
    components: (contract.components ?? []).map((component) => ({
      ...component,
      version,
      visible_rules: (component.visible_rules ?? []).map((rule) =>
        rule.label === "release_version" ? { ...rule, value: version } : rule,
      ),
      checks: (component.checks ?? []).map((check) =>
        check.label === "release_version" ? { ...check, value: version } : check,
      ),
    })),
    visible_rules: (contract.visible_rules ?? []).map((rule) =>
      rule.label === "required development version" || rule.label === "required shipping version"
        ? { ...rule, value: version }
        : rule,
    ),
  };

  writeJson(updateChannelsPath, nextChannels);
  writeJson(installationIntegrityContractPath, nextContract);

  return {
    channels: nextChannels,
    contract: nextContract,
  };
}
