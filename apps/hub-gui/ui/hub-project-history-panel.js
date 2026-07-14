import { runProjectBundleAction as executeProjectBundleAction } from "./hub-project-bundles.js";
import {
  actionIdentity,
  copyProjectCliCommand,
  copyPythonMacroStub,
  projectActionStateClass,
  pushRecentValue,
  rememberProjectBundleAction,
  sortProjectActionHistory,
  summarizeProjectActionResult,
} from "./hub-project-history.js";

export function createHubProjectHistoryPanel(context) {
  function saveProjectBundleRecents({
    action = "",
    bundlePath = "",
    comparePath = "",
    outputPath = "",
    status = "idle",
    note = "",
    executedAt = "",
  } = {}) {
    const next = context.loadHubRecents();
    next.bundles = pushRecentValue(next.bundles, bundlePath);
    next.compares = pushRecentValue(next.compares, comparePath);
    next.outputs = pushRecentValue(next.outputs, outputPath);
    next.actions = rememberProjectBundleAction(action, { bundlePath, comparePath, outputPath, status, note, executedAt });
    context.saveHubRecents(next);
  }

  function renderRecentPathList(container, values, input) {
    if (!container) {
      return;
    }

    container.innerHTML = "";
    if (!values.length) {
      renderEmptyHistoryState(
        container,
        context.hubCopy().dynamic?.recentEntriesEmpty
          || context.hubI18n.en.dynamic?.recentEntriesEmpty
          || "No recent entries yet.",
      );
      return;
    }

    values.forEach((value) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "hub-recent-item desktop-shell-button-ghost";
      button.textContent = value;
      button.title = value;
      button.addEventListener("click", () => {
        input.value = value;
        input.focus();
      });
      container.appendChild(button);
    });
  }

  function renderHubRecents(recents = context.loadHubRecents()) {
    renderRecentPathList(context.elements.recentBundleList, recents.bundles, context.elements.projectBundlePath);
    renderRecentPathList(context.elements.recentCompareList, recents.compares, context.elements.projectBundleComparePath);
    renderRecentPathList(context.elements.recentOutputList, recents.outputs, context.elements.projectBundleOutPath);
    renderHistoryFilters();
    renderRecentActionHistory(sortProjectActionHistory(recents.actions ?? []));
    context.renderHubWorkloadLibrary();
    context.renderAssistantContext();
    context.renderHubAssistantLocalCards();
  }

  function renderRecentActionHistory(actions) {
    if (!context.elements.recentActionList || !context.elements.favoriteActionList) {
      return;
    }

    const filteredActions = actions.filter((entry) => matchesHistoryFilter(entry));
    const favoriteActions = filteredActions.filter((entry) => entry.pinned);
    const recentActions = filteredActions.filter((entry) => !entry.pinned);
    context.elements.favoriteActionList.innerHTML = "";
    context.elements.recentActionList.innerHTML = "";

    if (!actions.length) {
      renderEmptyHistoryState(
        context.elements.favoriteActionList,
        context.hubCopy().dynamic?.favoritesEmpty || context.hubI18n.en.dynamic?.favoritesEmpty || "No favorite actions yet.",
      );
      renderEmptyHistoryState(
        context.elements.recentActionList,
        context.hubCopy().dynamic?.recentActionsEmpty || context.hubI18n.en.dynamic?.recentActionsEmpty || "No recent project actions yet.",
      );
      return;
    }

    if (!filteredActions.length) {
      renderEmptyHistoryState(
        context.elements.favoriteActionList,
        context.hubMessage(
          context.hubCopy().dynamic?.favoritesFilterEmpty
            || context.hubI18n.en.dynamic?.favoritesFilterEmpty
            || "No favorites match the {filter} filter.",
          { filter: context.localizedHistoryFilterLabel(context.state.historyFilter) },
        ),
      );
      renderEmptyHistoryState(
        context.elements.recentActionList,
        context.hubMessage(
          context.hubCopy().dynamic?.actionsFilterEmpty
            || context.hubI18n.en.dynamic?.actionsFilterEmpty
            || "No actions match the {filter} filter.",
          { filter: context.localizedHistoryFilterLabel(context.state.historyFilter) },
        ),
      );
      return;
    }

    if (!favoriteActions.length) {
      renderEmptyHistoryState(
        context.elements.favoriteActionList,
        context.hubCopy().dynamic?.pinnedFavoritesEmpty || context.hubI18n.en.dynamic?.pinnedFavoritesEmpty || "No pinned favorites yet.",
      );
    } else {
      renderProjectActionEntries(context.elements.favoriteActionList, favoriteActions);
    }

    if (!recentActions.length) {
      renderEmptyHistoryState(
        context.elements.recentActionList,
        context.hubCopy().dynamic?.nonPinnedEmpty || context.hubI18n.en.dynamic?.nonPinnedEmpty || "No non-pinned actions in this view.",
      );
    } else {
      renderProjectActionEntries(context.elements.recentActionList, recentActions);
    }
  }

  function renderEmptyHistoryState(container, message) {
    const empty = document.createElement("div");
    empty.className = "hub-recent-empty";
    empty.textContent = message;
    container.appendChild(empty);
  }

  function renderProjectActionEntries(container, actions) {
    actions.forEach((entry) => {
      const shell = document.createElement("div");
      shell.className = "hub-history-item";
      shell.append(renderProjectActionSummary(entry), renderProjectActionControls(entry));
      container.appendChild(shell);
    });
  }

  function renderProjectActionSummary(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "hub-history-item__summary desktop-shell-button-ghost";
    const paths = [entry.bundlePath, entry.comparePath, entry.outputPath].filter(Boolean).join("  •  ");
    const time = context.formatProjectActionTime(entry.executedAt);
    const details = summarizeProjectActionResult(entry.note) || paths || "No stored paths";
    const title = entry.pinned && entry.favoriteLabel ? entry.favoriteLabel : entry.action;
    const heading = document.createElement("div");
    heading.className = "hub-history-item__heading";
    const titleElement = document.createElement("strong");
    titleElement.textContent = title;
    const meta = document.createElement("div");
    meta.className = "hub-history-item__meta";
    const badge = document.createElement("span");
    badge.className = projectActionStateClass(entry.status);
    badge.textContent = entry.status || "idle";
    meta.appendChild(badge);
    if (time) {
      const timeElement = document.createElement("span");
      timeElement.textContent = time;
      meta.appendChild(timeElement);
    }
    heading.append(titleElement, meta);
    button.appendChild(heading);
    if (entry.pinned && entry.favoriteLabel) {
      const alias = document.createElement("span");
      alias.className = "hub-history-item__alias";
      alias.textContent = entry.action;
      button.appendChild(alias);
    }
    const detailsElement = document.createElement("span");
    detailsElement.textContent = details;
    button.appendChild(detailsElement);
    button.addEventListener("click", () => restoreProjectActionWithNotice(entry));
    return button;
  }

  function renderProjectActionControls(entry) {
    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";
    controls.append(renderRestoreButton(entry));

    if (entry.pinned) {
      controls.append(renderRenameButton(entry), renderCopyCliButton(entry), renderCopyPythonButton(entry));
    }

    controls.append(renderPinButton(entry), renderRerunButton(entry));
    return controls;
  }

  function renderRestoreButton(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = context.hubCopy().dynamic?.actionRestore || context.hubI18n.en.dynamic?.actionRestore || "Restore";
    button.addEventListener("click", () => restoreProjectActionWithNotice(entry));
    return button;
  }

  function renderRerunButton(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-primary";
    button.textContent = context.hubCopy().dynamic?.actionRerun || context.hubI18n.en.dynamic?.actionRerun || "Re-run";
    button.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      void rerunProjectActionEntry(entry);
    });
    return button;
  }

  function renderPinButton(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = entry.pinned ? "desktop-shell-button-primary" : "desktop-shell-button-ghost";
    button.textContent = entry.pinned
      ? context.hubCopy().dynamic?.actionPinned || context.hubI18n.en.dynamic?.actionPinned || "Pinned"
      : context.hubCopy().dynamic?.actionPin || context.hubI18n.en.dynamic?.actionPin || "Pin";
    button.addEventListener("click", () => togglePinnedProjectAction(entry));
    return button;
  }

  function renderRenameButton(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = context.hubCopy().dynamic?.actionLabel || context.hubI18n.en.dynamic?.actionLabel || "Label";
    button.addEventListener("click", () => renamePinnedProjectAction(entry));
    return button;
  }

  function renderCopyCliButton(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = context.hubCopy().dynamic?.actionCopyCli || context.hubI18n.en.dynamic?.actionCopyCli || "Copy CLI";
    button.addEventListener("click", () => {
      void copyProjectCliCommand(entry, { setProjectBundleOutput: context.setProjectBundleOutput });
    });
    return button;
  }

  function renderCopyPythonButton(entry) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = context.hubCopy().dynamic?.actionCopyPython || context.hubI18n.en.dynamic?.actionCopyPython || "Copy Python";
    button.addEventListener("click", () => {
      void copyPythonMacroStub(entry, { setProjectBundleOutput: context.setProjectBundleOutput });
    });
    return button;
  }

  function renderHistoryFilters() {
    context.elements.historyFilterButtons.forEach((button) => {
      const isActive = button.dataset.historyFilter === context.state.historyFilter;
      button.classList.toggle("desktop-shell-button-primary", isActive);
      button.classList.toggle("desktop-shell-button-ghost", !isActive);
      button.setAttribute("aria-pressed", String(isActive));
    });
  }

  function matchesHistoryFilter(entry) {
    switch (context.state.historyFilter) {
      case "failed":
        return entry.status === "failed";
      case "inspect":
        return entry.action === "project inspect";
      case "normalize":
        return entry.action === "project normalize";
      case "diff":
        return entry.action === "project diff";
      case "all":
      default:
        return true;
    }
  }

  function togglePinnedProjectAction(entry) {
    const recents = context.loadHubRecents();
    const identity = actionIdentity(entry);
    recents.actions = (recents.actions ?? []).map((candidate) => {
      if (actionIdentity(candidate) !== identity) {
        return candidate;
      }

      return {
        ...candidate,
        pinned: !candidate.pinned,
        favoriteLabel: candidate.pinned ? "" : candidate.favoriteLabel,
      };
    });
    context.saveHubRecents(recents);
    context.setProjectBundleOutput(`${entry.pinned ? "unpinned" : "pinned"} ${entry.action}`);
  }

  function renamePinnedProjectAction(entry) {
    const currentLabel = String(entry.favoriteLabel || "");
    const nextLabel = window.prompt("Favorite label", currentLabel || entry.action);
    if (nextLabel === null) {
      return;
    }

    const recents = context.loadHubRecents();
    const identity = actionIdentity(entry);
    recents.actions = (recents.actions ?? []).map((candidate) => {
      if (actionIdentity(candidate) !== identity) {
        return candidate;
      }

      return {
        ...candidate,
        favoriteLabel: String(nextLabel || "").trim(),
      };
    });
    context.saveHubRecents(recents);
    context.setProjectBundleOutput(`updated label for ${entry.action}`);
  }

  function restoreProjectActionWithNotice(entry) {
    restoreProjectActionContext(entry);
    context.setProjectBundleOutput(
      context.hubMessage(
        context.hubCopy().dynamic?.restoredActionContext
          || context.hubI18n.en.dynamic?.restoredActionContext
          || "restored {action} context",
        { action: entry.action },
      ),
    );
  }

  function restoreProjectActionContext(entry) {
    context.elements.projectBundlePath.value = entry.bundlePath || "";
    context.elements.projectBundleComparePath.value = entry.comparePath || "";
    context.elements.projectBundleOutPath.value = entry.outputPath || "";
  }

  async function rerunProjectActionEntry(entry) {
    const action = context.projectActionLabels[entry.action];
    if (!action) {
      context.setProjectBundleOutput(`cannot re-run unknown action: ${entry.action}`);
      return;
    }

    await context.runAction(action);
  }

  async function runProjectBundleAction({ action, command, payload, outputTarget, successOutput }) {
    return executeProjectBundleAction({
      action,
      command,
      payload,
      invokeTauri: context.invokeTauri,
      saveProjectBundleRecents,
      outputTarget,
      elements: context.elements,
      setBusy: context.setBusy,
      projectActionLabels: context.projectActionLabels,
      successOutput,
    });
  }

  return {
    renderEmptyHistoryState,
    renderHistoryFilters,
    renderHubRecents,
    runProjectBundleAction,
    saveProjectBundleRecents,
  };
}
