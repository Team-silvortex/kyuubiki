import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SRC_ROOT = path.join(ROOT, "src");

function resolveAliasPath(specifier) {
  const basePath = path.join(SRC_ROOT, specifier.slice(2));
  const candidates = [
    basePath,
    `${basePath}.ts`,
    `${basePath}.tsx`,
    `${basePath}.js`,
    `${basePath}.mjs`,
    path.join(basePath, "index.ts"),
    path.join(basePath, "index.tsx"),
  ];
  return candidates.find((candidate) => existsSync(candidate)) ?? basePath;
}

export async function resolve(specifier, context, nextResolve) {
  if (specifier.startsWith("@/")) {
    const resolvedPath = resolveAliasPath(specifier);
    return nextResolve(pathToFileURL(resolvedPath).href, context);
  }
  return nextResolve(specifier, context);
}
