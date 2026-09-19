import { bundleCreationCopy, type BundleCreationCopy } from "./hub-bundle-creation-copy.js";
import type { HubActionOutcome } from "./hub-action-runner.js";

type CreatePayload = { path: string; parentPath: string; name: string };
type Target = { valid: true; payload: CreatePayload } | { valid: false; field: "name" | "directory" };
type CreationContext = {
  invokeTauri: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  runActionWithOptions: (action: string, options: { skipConfirmation: boolean; bundleCreate: CreatePayload }) => Promise<HubActionOutcome>;
  setProjectBundlePath: (path: string) => void;
  language: () => string;
  isBusy: () => boolean;
};

export function bundleCreationTarget(directory: string, inputName: string): Target {
  const name = inputName.trim().replace(/\.kyuubiki$/iu, "");
  const stem = name.split(".")[0];
  if (!name || new TextEncoder().encode(name).length > 200 || /[. ]$/u.test(name)
    || /[\x00-\x1f\x7f-\x9f/\\:*?"<>|]/u.test(name) || /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/iu.test(stem)) {
    return { valid: false, field: "name" };
  }
  const parentPath = directory;
  const absolute = parentPath.startsWith("/") || /^[a-z]:[\\/]/iu.test(parentPath)
    || /^\\\\[^\\/]+[\\/][^\\/]+/u.test(parentPath);
  if (!absolute || /[\x00-\x1f\x7f-\x9f]/u.test(parentPath)) return { valid: false, field: "directory" };
  const windowsPath = !parentPath.startsWith("/");
  const separator = windowsPath && parentPath.includes("\\") ? "\\" : "/";
  const parent = parentPath.replace(windowsPath ? /[\\/]+$/u : /\/+$/u, "");
  const path = `${parent}${separator}${name}.kyuubiki`;
  return { valid: true, payload: { path, parentPath, name } };
}

export function renderBundleCreationCopy(language: string): void {
  const copy = bundleCreationCopy(language);
  document.querySelectorAll<HTMLElement>("[data-bundle-copy]").forEach((element) => {
    const value = copy[element.dataset.bundleCopy as keyof BundleCreationCopy];
    if (value) element.textContent = value;
  });
}

export function bindHubBundleCreation(context: CreationContext) {
  const get = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
  const dialog = get<HTMLDialogElement>("bundle-create-dialog");
  const form = get<HTMLFormElement>("bundle-create-form");
  const name = get<HTMLInputElement>("bundle-create-name");
  const directory = get<HTMLInputElement>("bundle-create-directory");
  const target = get<HTMLOutputElement>("bundle-create-target");
  const error = get<HTMLParagraphElement>("bundle-create-error");
  const submitButton = get<HTMLButtonElement>("bundle-create-submit");
  const browse = get<HTMLButtonElement>("bundle-create-browse");
  const openButton = get<HTMLButtonElement>("bundles-action-browse");
  const newButton = get<HTMLButtonElement>("bundles-action-create");
  const cancel = get<HTMLButtonElement>("bundle-create-cancel");
  const feedback = get<HTMLParagraphElement>("bundle-feedback");
  let pending = false;
  let choosing = false;
  const copy = () => bundleCreationCopy(context.language());

  function showError(message: string) {
    error.textContent = message;
    error.hidden = !message;
  }

  function update() {
    const next = bundleCreationTarget(directory.value, name.value);
    target.textContent = next.valid ? next.payload.path : "...";
    submitButton.disabled = pending || choosing || context.isBusy() || !next.valid;
    name.disabled = directory.disabled = cancel.disabled = pending || choosing;
    browse.disabled = pending || choosing;
    name.setAttribute("aria-invalid", String(!next.valid && next.field === "name" && Boolean(name.value)));
    directory.setAttribute("aria-invalid", String(!next.valid && next.field === "directory" && Boolean(directory.value)));
    return next;
  }

  function edit() {
    const next = update();
    const invalidInput = !next.valid && (next.field === "name" ? name.value : directory.value);
    showError(invalidInput ? copy().invalid : "");
  }

  function open() {
    if (context.isBusy() || pending || choosing) return;
    if (!directory.value) {
      const active = get<HTMLInputElement>("project-bundle-path").value;
      const slash = active.startsWith("/") ? active.lastIndexOf("/")
        : Math.max(active.lastIndexOf("/"), active.lastIndexOf("\\"));
      if (slash >= 0) directory.value = active.slice(0, slash + 1);
    }
    showError("");
    update();
    dialog.showModal();
    name.focus();
  }

  function close() {
    if (!pending && !choosing) {
      dialog.close();
      newButton.focus();
    }
  }

  async function pick(kind: "directory" | "bundle") {
    if (pending || choosing || context.isBusy()) return;
    choosing = true;
    openButton.disabled = true;
    update();
    try {
      const path = await context.invokeTauri("project_bundle_pick_path", {
        payload: { kind, initialPath: kind === "directory" ? directory.value : get<HTMLInputElement>("project-bundle-path").value },
      });
      if (path === null || path === undefined) return;
      if (typeof path !== "string" || !path.trim()) throw new Error("Invalid native path chooser response");
      if (kind === "directory") {
        directory.value = path;
        showError("");
      } else {
        context.setProjectBundlePath(path);
        feedback.dataset.bundleCopy = "ready";
        feedback.textContent = copy().ready;
      }
    } catch (failure) {
      const message = String(failure);
      if (kind === "directory") showError(message);
      else {
        delete feedback.dataset.bundleCopy;
        feedback.textContent = message;
      }
    } finally {
      choosing = false;
      openButton.disabled = false;
      update();
    }
  }

  async function submit() {
    if (!dialog.open || pending || choosing) return;
    const next = update();
    if (!next.valid) {
      showError(copy().invalid);
      (next.field === "name" ? name : directory).focus();
      return;
    }
    if (context.isBusy()) return;
    pending = true;
    showError("");
    update();
    try {
      // This explicit form submission confirms the previewed target once. The
      // same guarded, audited action remains available to PWDT with a raw path.
      const outcome = await context.runActionWithOptions("project-create", {
        skipConfirmation: true, bundleCreate: next.payload,
      });
      if (outcome.status !== "completed") {
        showError(get<HTMLElement>("project-bundle-output").textContent || outcome.status);
        return;
      }
      feedback.dataset.bundleCopy = "ready";
      feedback.textContent = copy().ready;
      get<HTMLDetailsElement>("bundle-result-details").open = false;
      dialog.close();
      name.value = "";
      get<HTMLButtonElement>("bundles-action-inspect").focus();
    } catch (failure) {
      showError(String(failure));
    } finally {
      pending = false;
      update();
    }
  }

  name.addEventListener("input", edit);
  directory.addEventListener("input", edit);
  newButton.addEventListener("click", open);
  cancel.addEventListener("click", close);
  browse.addEventListener("click", () => void pick("directory"));
  openButton.addEventListener("click", () => void pick("bundle"));
  dialog.addEventListener("cancel", (event) => { event.preventDefault(); close(); });
  form.addEventListener("submit", (event) => { event.preventDefault(); void submit(); });
  return { submit };
}
