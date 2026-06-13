export function buildHubAssistantLocalCards(options) {
  const snapshot = options.currentAssistantSnapshot();
  const cards = [];

  if (!snapshot.bundlePath) {
    cards.push({
      id: "bundle-path",
      title: options.hubDynamic("cardBundlePathTitle"),
      summary: options.hubDynamic("cardBundlePathSummary"),
      actionLabel: options.homeBundlesTitle,
      tone: "watch",
      onAction: () => {
        options.setSection("projects");
        options.setProjectsPage("bundles");
        options.projectBundlePath?.focus();
        options.setProjectBundleOutput(options.hubDynamic("focusedBundleField"));
      },
    });
  }

  if (!/ready|healthy/i.test(snapshot.runtimeStatus)) {
    cards.push({
      id: "start-local",
      title: options.hubDynamic("cardStartLocalTitle"),
      summary: options.hubDynamic("cardStartLocalSummary"),
      actionLabel: options.shellStartLocal,
      tone: "risk",
      onAction: () => {
        void options.runAction("start-local");
      },
    });
  }

  if (snapshot.bundlePath) {
    cards.push({
      id: "inspect-bundle",
      title: options.hubDynamic("cardInspectTitle"),
      summary: options.hubDynamic("cardInspectSummary"),
      actionLabel: options.bundleInspect,
      tone: "good",
      onAction: () => {
        void options.runAction("project-inspect");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.outputPath) {
    cards.push({
      id: "normalize-bundle",
      title: options.hubDynamic("cardNormalizeTitle"),
      summary: options.hubDynamic("cardNormalizeSummary"),
      actionLabel: options.bundleNormalize,
      tone: "good",
      onAction: () => {
        void options.runAction("project-normalize");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.comparePath) {
    cards.push({
      id: "diff-bundles",
      title: options.hubDynamic("cardDiffTitle"),
      summary: options.hubDynamic("cardDiffSummary"),
      actionLabel: options.bundleDiff,
      tone: "watch",
      onAction: () => {
        void options.runAction("project-diff");
      },
    });
  }

  cards.push({
    id: "open-guides",
    title: options.hubDynamic("cardGuidesTitle"),
    summary: options.hubDynamic("cardGuidesSummary"),
    actionLabel: options.homeGuidesTitle,
    tone: "watch",
    onAction: () => {
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setProjectBundleOutput(options.hubDynamic("focusedGuidesPage"));
    },
  });

  cards.push({
    id: "open-workbench",
    title: options.hubDynamic("cardWorkbenchTitle"),
    summary: options.hubDynamic("cardWorkbenchSummary"),
    actionLabel: options.shellOpenWorkbench,
    tone: "good",
    onAction: () => {
      void options.runAction("open-workbench");
    },
  });

  return cards.slice(0, 5);
}

export function renderHubAssistantLocalCards(options) {
  if (!options.assistantLocalCards) {
    return;
  }

  const cards = buildHubAssistantLocalCards(options);
  options.assistantLocalCards.innerHTML = "";
  if (!cards.length) {
    options.renderEmptyHistoryState(options.assistantLocalCards, options.hubDynamic("assistantNoUrgent"));
    return;
  }

  cards.forEach((card) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    options.appendAssistantCardHeader(
      article,
      card.title,
      card.tone,
      `desktop-shell-state desktop-shell-state--${
        card.tone === "risk" ? "danger" : card.tone === "watch" ? "warning" : "healthy"
      }`,
    );
    options.appendTextElement(article, "p", card.summary, "desktop-shell-note");
    const buttonRow = document.createElement("div");
    buttonRow.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = card.actionLabel;
    button.addEventListener("click", card.onAction);
    buttonRow.appendChild(button);
    article.appendChild(buttonRow);
    options.assistantLocalCards.appendChild(article);
  });
}

export function buildLocalGuideContext(options) {
  const snapshot = options.currentAssistantSnapshot();
  return {
    section: snapshot.activeSection,
    runtimeReady: /ready|healthy/i.test(snapshot.runtimeStatus),
    hasBundle: Boolean(snapshot.bundlePath),
    hasCompare: Boolean(snapshot.comparePath),
    hasOutput: Boolean(snapshot.outputPath),
    bundlePath: snapshot.bundlePath,
  };
}

export function buildLocalGuideResponse(query, options) {
  const normalized = String(query || "").trim().toLowerCase();
  const context = buildLocalGuideContext(options);

  if (!normalized) {
    return options.hubDynamic("assistantPromptEmpty");
  }
  if (/first|start|begin|fresh|what should i do/.test(normalized)) {
    if (!context.runtimeReady) {
      return options.hubDynamic("guideFirstNoRuntime");
    }
    if (!context.hasBundle) {
      return options.hubDynamic("guideFirstNoBundle");
    }
    return options.hubDynamic("guideFirstReady");
  }
  if (/inspect|bundle|validate|normalize|diff|pack|unpack/.test(normalized)) {
    if (!context.hasBundle) {
      return options.hubDynamic("guideBundleNoPath");
    }
    if (/normalize/.test(normalized) && !context.hasOutput) {
      return options.hubDynamic("guideNormalizeNoOutput");
    }
    if (/diff/.test(normalized) && !context.hasCompare) {
      return options.hubDynamic("guideDiffNoCompare");
    }
    return options.hubDynamic("guideBundleGeneral");
  }
  if (/workbench|analysis|open/.test(normalized)) {
    return options.hubDynamic("guideWorkbench");
  }
  if (/docs|guide|read|document|manual|help/.test(normalized)) {
    return options.hubDynamic("guideDocs");
  }
  if (/runtime|stack|agent|hot|observe|log/.test(normalized)) {
    return options.hubDynamic("guideRuntime");
  }
  if (/catalog|library|workload|remote/.test(normalized)) {
    return options.hubDynamic("guideLibrary");
  }
  if (/installer|package|packaging|desktop|dmg|build/.test(normalized)) {
    return options.hubDynamic("guidePackaging");
  }
  if (/partial|failed|error|warning/.test(normalized)) {
    return options.hubDynamic("guideFailure");
  }
  return options.hubDynamic("guideFallback");
}

export function extractAssistantJsonBlock(value) {
  const fenced = value.match(/```json\s*([\s\S]*?)```/i);
  if (fenced?.[1]) {
    return fenced[1].trim();
  }

  const firstBrace = value.indexOf("{");
  const lastBrace = value.lastIndexOf("}");
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    return value.slice(firstBrace, lastBrace + 1);
  }

  return String(value || "").trim();
}
