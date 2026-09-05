import { existsSync, statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const REPO_ROOT = path.resolve(ROOT, "../..");
const SRC_ROOT = path.join(ROOT, "src");

function resolveSourcePath(basePath) {
  const candidates = [
    `${basePath}.ts`,
    `${basePath}.tsx`,
    `${basePath}.js`,
    `${basePath}.mjs`,
    path.join(basePath, "index.ts"),
    path.join(basePath, "index.tsx"),
    basePath,
  ];
  return candidates.find((candidate) => existsSync(candidate) && statSync(candidate).isFile());
}

export async function resolve(specifier, context, nextResolve) {
  if (specifier.startsWith("@/")) {
    const resolvedPath = resolveSourcePath(path.join(SRC_ROOT, specifier.slice(2)));
    if (resolvedPath) return nextResolve(pathToFileURL(resolvedPath).href, context);
  }
  if (specifier.startsWith(".") && context.parentURL?.startsWith("file:")) {
    const parentPath = fileURLToPath(context.parentURL);
    if (parentPath.startsWith(ROOT) && !path.extname(specifier)) {
      const resolvedPath = resolveSourcePath(path.resolve(path.dirname(parentPath), specifier));
      if (resolvedPath) return nextResolve(pathToFileURL(resolvedPath).href, context);
    }
  }
  return nextResolve(specifier, context);
}

export async function load(url, context, nextLoad) {
  if (url.endsWith(".ts") || url.endsWith(".tsx")) {
    const source = await readFile(fileURLToPath(url), "utf8");
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        jsx: ts.JsxEmit.Preserve,
        module: ts.ModuleKind.ESNext,
        target: ts.ScriptTarget.ES2022,
      },
      fileName: fileURLToPath(url),
    });
    return {
      format: "module",
      shortCircuit: true,
      source: transpiled.outputText,
    };
  }
  if (url.endsWith(".json") && fileURLToPath(url).startsWith(REPO_ROOT)) {
    const source = JSON.parse(await readFile(fileURLToPath(url), "utf8"));
    return {
      format: "module",
      shortCircuit: true,
      source: `export default ${JSON.stringify(source)};`,
    };
  }

  return nextLoad(url, context);
}
