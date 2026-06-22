import {
  applyDesktopState,
  invokeTauri,
  loadDesktopBrand,
  loadDesktopLanguagePreference,
  normalizeDesktopLanguage,
  saveDesktopLanguagePreference,
  setText,
  syncDesktopStates,
} from "./shared/tauri-bridge.js";
import {
  desktopPlatformContextLabel,
  detectDesktopPlatform,
  normalizeDesktopPlatform,
  populateDesktopPlatformSelect,
} from "./shared/platform.js";
import {
  currentProjectBundleComparePayload as buildCurrentProjectBundleComparePayload,
  currentProjectBundleOutputPayload as buildCurrentProjectBundleOutputPayload,
  currentProjectBundlePayload as buildCurrentProjectBundlePayload,
  runProjectBundleAction as executeProjectBundleAction,
} from "./hub-project-bundles.js";
import { runHubProjectAction } from "./hub-project-actions.js";
import { runHubRuntimeAction } from "./hub-runtime-actions.js";
import { runHubDesktopAction } from "./hub-desktop-actions.js";
import {
  builtInWorkflowSampleInputArtifacts as buildBuiltInWorkflowSampleInputArtifacts,
  describeWorkflowSummary as buildWorkflowSummaryDescription,
  fetchWorkflowCatalog as fetchWorkflowCatalogPanel,
  renderWorkflowCatalog as renderWorkflowCatalogPanel,
  runWorkflowCatalogSample as runWorkflowCatalogSamplePanel,
  waitForWorkflowJob as waitForWorkflowCatalogJob,
} from "./hub-workflow-catalog.js";
import {
  applyAssistantBundlePayload as applyAssistantBundlePayloadEngine,
  assistantHostRequiresTrust as assistantHostRequiresTrustEngine,
  assistantTrustHostOrigin as assistantTrustHostOriginEngine,
  confirmHubAssistantAction as confirmHubAssistantActionEngine,
  ensureAssistantHostTrust as ensureAssistantHostTrustEngine,
  ensureRemoteHostTrust as ensureRemoteHostTrustEngine,
  executeHubAssistantAction as executeHubAssistantActionEngine,
  executeHubAssistantPlan as executeHubAssistantPlanEngine,
  renderHubAssistantPlan as renderHubAssistantPlanEngine,
  requestHubAssistantPlan as requestHubAssistantPlanEngine,
} from "./hub-assistant-engine.js";
import {
  buildHubDesktopActionContext,
  buildHubProjectActionContext,
  buildHubRuntimeActionContext,
  buildHubWorkloadActionContext,
} from "./hub-action-contexts.js";
import {
  buildHubAssistantLocalCards as buildHubAssistantLocalCardsModule,
  buildLocalGuideResponse as buildLocalGuideResponseModule,
  extractAssistantJsonBlock as extractAssistantJsonBlockModule,
  renderHubAssistantLocalCards as renderHubAssistantLocalCardsModule,
} from "./hub-assistant-local.js";
import {
  inferDownloadFilename as deriveDownloadFilename,
  loadHubWorkloadLibrary as loadStoredHubWorkloadLibrary,
  mergeHubWorkloadLibrary as mergeStoredHubWorkloadLibrary,
  normalizeHubWorkloadEntry as normalizeStoredHubWorkloadEntry,
  normalizeRemoteWorkloadCatalogPayload as buildNormalizedRemoteWorkloadCatalogPayload,
  persistHubWorkloadLibrary as persistStoredHubWorkloadLibrary,
  projectSummaryFromInspectPayload as parseProjectSummaryFromInspectPayload,
  validateHubCatalogUrl as validateWorkloadCatalogUrl,
  validateRemoteWorkloadCatalogPayload as validateWorkloadCatalogPayload,
  workloadDomainLabel as resolveWorkloadDomainLabel,
  workloadFamilyLabel as resolveWorkloadFamilyLabel,
  workloadIdentity as buildWorkloadIdentity,
  workloadProvenanceHost as resolveWorkloadProvenanceHost,
  workloadProvenanceLabel as resolveWorkloadProvenanceLabel,
  workloadSourceBadge as resolveWorkloadSourceBadge,
} from "./hub-workload-library.js";
import { runHubWorkloadAction } from "./hub-workload-actions.js";
import {
  copySanitizedRuntimeLogToClipboard,
  inferHotRuntimeState as inferHotRuntimeStateHelper,
  refreshDesktopStatusPanel,
  refreshHotRuntimeStatusPanel,
  refreshRuntimeLogPanel,
  refreshRuntimeStatusPanel,
  sanitizeRuntimeLogForClipboard as sanitizeRuntimeLogForClipboardHelper,
} from "./hub-runtime-helpers.js";
import { applyHubDocsI18n } from "./hub-i18n-docs.js";
import { applyHubGuidesI18n } from "./hub-i18n-guides.js";
import { applyHubLocalizationI18n } from "./hub-i18n-localization.js";
import { applyHubAssistantI18n } from "./hub-i18n-assistant.js";
import { applyHubWorkloadsI18n } from "./hub-i18n-workloads.js";
import { renderAssistantShellCopy } from "./hub-assistant-shell.js";
import { renderHubBundlesCopy } from "./hub-bundles-copy.js";
import { renderHubHomeCopy } from "./hub-home-copy.js";
import { bindHubLibraryControls } from "./hub-library-controls.js";
import { renderHubLibraryCopy } from "./hub-library-copy.js";
import { renderHubPanelCopy } from "./hub-panel-copy.js";
import { bindHubRecentActionControls } from "./hub-recent-actions.js";
import { renderHubWorkloadLibraryList, renderWorkloadFilters as renderWorkloadFiltersModule } from "./hub-workload-list.js";
import {
  attachCurrentBundleToWorkload as attachCurrentBundleToWorkloadModule,
  clearHubWorkloadLibrary as clearHubWorkloadLibraryModule,
  currentWorkloadLibrarySearchQuery as currentWorkloadLibrarySearchQueryModule,
  downloadRemoteWorkloadBundle as downloadRemoteWorkloadBundleModule,
  exportHubWorkloadLibrary as exportHubWorkloadLibraryModule,
  importHubWorkloadLibrary as importHubWorkloadLibraryModule,
  matchesWorkloadFamilyFilter as matchesWorkloadFamilyFilterModule,
  matchesWorkloadFilter as matchesWorkloadFilterModule,
  matchesWorkloadSearchQuery as matchesWorkloadSearchQueryModule,
  openWorkloadInWorkbench as openWorkloadInWorkbenchModule,
  registerCurrentBundleAsWorkload as registerCurrentBundleAsWorkloadModule,
  saveHubWorkloadLibrary as saveHubWorkloadLibraryModule,
  syncLocalControlPlaneWorkloads as syncLocalControlPlaneWorkloadsModule,
  syncRemoteWorkloadCatalog as syncRemoteWorkloadCatalogModule,
} from "./hub-workload-runtime.js";
import { resolveHubCopy } from "./hub-copy-registry.js";
import {
  renderDirectMeshRegressionLoadError,
  renderDirectMeshRegressionSnapshot,
  renderGuidesPanelCopy,
  renderRegressionGateReport,
} from "./hub-guides-panel.js";
import {
  bindHubLocalizationPanel,
} from "./hub-localization-panel.js";

const HUB_I18N = {
  en: {
    nav: {
      projects: "Home",
      runtimes: "Runtimes",
      deploy: "Deploy",
      observe: "Observe",
      tools: "Tools",
    },
    sections: {
      projects: {
        title: "Home",
        copy: "Start with one clear path: bring work in, inspect it once, then move into Workbench.",
      },
      runtimes: {
        title: "Runtimes",
        copy: "Start the right loop, check runtime health, and keep logs close.",
      },
      deploy: {
        title: "Deploy",
        copy: "Choose the target posture, validate the workstation, and prepare release paths.",
      },
      observe: {
        title: "Observe",
        copy: "Scan health, tails, and recent risk signals without leaving the desktop shell.",
      },
      tools: {
        title: "Tools",
        copy: "Run diagnostics, packaging, and verification from one operator surface.",
      },
    },
    shell: {
      language: "Language",
      actionStatus: "Action status",
      idle: "idle",
      openWorkbench: "Open workbench",
      startLocal: "Start local stack",
      validateEnv: "Validate env",
      focus: "runtime orchestration",
    },
    signals: {
      intakeLabel: "Workload intake",
      intakeTitle: "local + remote",
      intakeCopy: "Register bundles, sync catalogs, and keep one shelf in view.",
      domainsLabel: "Analysis domains",
      domainsTitle: "mechanical / thermal / thermo",
      domainsCopy: "The same study language now flows through Hub, CLI, and Workbench.",
      firstMoveLabel: "Recommended first move",
      firstMoveTitle: "sync, inspect, open",
      firstMoveCopy: "Sync first, inspect once, then open Workbench.",
    },
    home: {
      tabs: {
        start: "Start here",
        library: "Library",
        bundles: "Bundle tools",
        guides: "Guides",
      },
      steps: {
        step1Label: "Step 1",
        step1Title: "Bring work in",
        step1Copy: "Register the current bundle, sync the local control plane, or pull a remote catalog into one shelf.",
        step2Label: "Step 2",
        step2Title: "Inspect once",
        step2Copy: "Validate the bundle shape and analysis intent before you go deeper.",
        step3Label: "Step 3",
        step3Title: "Open Workbench",
        step3Copy: "Move into analysis only after the active bundle and runtime path look safe.",
      },
      path: {
        label: "Recommended path",
        title: "Use Hub as a short runway",
        copy: "If this is a fresh session, follow one short path instead of bouncing across all sections.",
      },
      flow: {
        title1: "Start the local stack if needed",
        copy1: "Bring the local runtime online before you inspect or open anything that depends on it.",
        title2: "Sync or register work",
        copy2: "Pull from the local control plane, a remote catalog, or the current bundle path.",
        title3: "Inspect once, then open",
        copy3: "Run one quick bundle check, then move into Workbench with fewer surprises.",
      },
      quick: {
        label: "Quick orientation",
        title: "Choose the right next move",
        copy: "Open the deeper pages only when you know which job you are doing.",
        libraryTitle: "Open Library",
        libraryCopy: "Curate workloads, sync catalogs, and filter by domain or family.",
        bundlesTitle: "Open Bundle tools",
        bundlesCopy: "Inspect, validate, normalize, pack, unpack, and diff project bundles.",
        guidesTitle: "Open docs & guides",
        guidesCopy: "Go to one clear documentation shelf for current-line, operations, troubleshooting, and accuracy notes.",
        installerTitle: "Open Installer",
        installerCopy: "Bootstrap release layouts, desktop packaging, and workstation setup from one shell.",
        runtimesTitle: "Open Runtimes",
        runtimesCopy: "Check stack health, hot-reload status, and runtime tails.",
      },
      actions: {
        start: "Start local stack",
        sync: "Sync local control plane",
        open: "Open workbench",
      },
    },
    panels: {
      runtimes: {
        tabs: ["Local runtime", "Hot loop", "Remote targets"],
        overview: [
          { label: "Runtime posture", title: "See the local stack first", copy: "Frontend, control plane, and agents stay visible before you dive into logs or hot-reload details." },
          { label: "Hot loop", title: "Develop without leaving Hub", copy: "Start the mode you need, then watch the current tail from one stable control surface." },
          { label: "Targets", title: "Keep expansion visible", copy: "Remote clusters and mesh labs stay in the same runtime map, even when your daily path is local." },
        ],
        local: {
          label: "Managed local loop",
          title: "Local runtime",
          copy: "Keep the workstation path readable: status first, URLs second, diagnostics close by.",
          status: "Status",
          frontend: "Frontend",
          controlPlane: "Control plane",
          agents: "Agents",
        },
        hot: {
          label: "Developer runtime",
          title: "Hot reload loop",
          copy: "Launch the right dev mode, then keep status, follow-state, and tail output in one place.",
          status: "Status",
          mode: "Mode",
          local: "Hot local",
          cloud: "Hot cloud",
          distributed: "Hot distributed",
          refreshStatus: "Refresh hot status",
          stop: "Stop hot loop",
          logs: "Hot logs",
          autoRefresh: "Auto refresh",
          interval: "Interval",
          refreshLog: "Refresh log",
          copyTail: "Copy tail",
          clearView: "Clear view",
          note: "Copy tail exports a sanitized view, not the raw log file.",
        },
        targets: {
          label: "Runtime map",
          title: "Remote targets",
          copy: "Keep the distributed picture visible even if your main release path stays on one workstation.",
        },
      },
      deploy: {
        tabs: ["Modes", "Bootstrap", "Release path"],
        modes: {
          label: "Runtime targeting",
          title: "Deployment modes",
          copy: "Choose the operating posture that matches the current environment, then restart or pivot without leaving Hub.",
          local: "Local workstation",
          cloud: "Cloud control plane",
          distributed: "Distributed control plane",
          restart: "Restart local stack",
        },
        bootstrap: {
          label: "Release prep",
          title: "Bootstrap",
          copy: "Use Hub for validation and readiness checks, then move into Installer for deployment and packaging work.",
          validate: "Validate env",
          stage: "Open Installer",
          doctor: "Run doctor",
        },
        release: {
          label: "Suggested deploy path",
          title: "Mode, validate, hand off",
          copy: "Switch posture first, validate next, then hand off to Installer for packaging, repair, and release work.",
        },
      },
      observe: {
        tabs: ["Health", "Runtime watch", "Stack watch"],
        overview: [
          { label: "Signal quality", title: "Health first, then detail", copy: "Keep watchdog, recent security events, and job failures readable before you move into tail output." },
          { label: "Live watch", title: "Mirror what the runtimes see", copy: "Observe mirrors stay close to the live runtime loop, so watch surfaces stay consistent across sections." },
          { label: "Operator flow", title: "Refresh, copy, move on", copy: "Use Observe for scanning and copying; use Runtimes when you need to change the underlying loop." },
        ],
        health: {
          label: "Stability",
          title: "Health and watchdog",
          copy: "Keep the short health story compact: stable runtime, recent security signal, and whether failures are actually accumulating.",
          watchdog: "Watchdog",
          security: "Security events",
          failures: "Failed jobs",
        },
        runtime: {
          title: "Runtime watch",
          localRuntime: "Local runtime",
          hotLoop: "Hot loop",
          mode: "Mode",
          logSource: "Log source",
          open: "Open runtimes",
          refresh: "Refresh tail",
          copy: "Copy tail",
        },
        stack: {
          title: "Stack watch",
          logs: "Stack logs",
          auto: "Auto watch",
          refresh: "Refresh tail",
          copy: "Copy tail",
          note: "Copy tail exports a sanitized view, not the raw log file.",
        },
      },
      tools: {
        tabs: ["Packages", "Status", "Output"],
        overview: [
          { label: "Diagnostics", title: "Read the platform first", copy: "Desktop readiness, benchmarks, and bundle validation stay visible before you hand off heavier packaging work." },
          { label: "Installer handoff", title: "Readiness here, heavy actions there", copy: "Use Hub for visibility and short checks, then open Installer for stage, build, verify, and cleanup flows." },
          { label: "Operational outputs", title: "Keep logs and results nearby", copy: "Status and operation output stay together so handoff decisions stay easy to audit while you iterate." },
        ],
        packages: {
          label: "Platform checks",
          title: "Diagnostics and readiness",
          copy: "Use Hub for local diagnostics, bundle validation, and desktop readiness before opening Installer for heavier packaging work.",
          platform: "Target platform",
          benchmark: "Run benchmark",
          validate: "Validate project bundle",
          export: "Export database",
          status: "Desktop status",
          stage: "Open Installer",
          stop: "Stop local stack",
        },
        status: {
          label: "Readiness",
          title: "Desktop status",
          copy: "Use this output as the short readiness wall before handing packaging, repair, or verification work to Installer.",
        },
        output: {
          label: "Operations",
          title: "Tool output",
          copy: "Keep diagnostics and supporting output close by so Installer handoff stays inspectable.",
        },
      },
    },
    library: {
      catalogUrl: "Catalog URL",
      labelOrNote: "Label or note",
      register: "Register current bundle",
      syncLocal: "Sync local control plane",
      syncRemote: "Sync remote catalog",
      export: "Export library JSON",
      import: "Import library JSON",
      clear: "Clear library",
      all: "All",
      mechanical: "Mechanical",
      thermal: "Thermal",
      thermo: "Thermo-mechanical",
      allFamilies: "All families",
      axial: "Axial & Springs",
      beams: "Beams & Frames",
      trusses: "Trusses",
      planes: "Planes",
      workloadSearchLabel: "Search workloads",
      workloadSearchPlaceholder: "bridge frame axial thermal",
      workloadSearchClear: "Clear search",
      workflowCatalogSearchLabel: "Search workflows",
      workflowCatalogSearchPlaceholder: "bridge thermal export",
      workflowCatalogSearchClear: "Clear search",
      workflowCatalogRefresh: "Refresh workflow catalog",
      workflowCatalogReady: "Workflow catalog is ready.",
      ready: "Workload library is ready.",
    },
    bundles: {
      all: "All",
      failed: "Failed",
      keepFailed: "Keep failed only",
      import: "Import JSON",
      export: "Export JSON",
      clear: "Clear history",
      recent: "Recent",
    },
    guides: {
      overviewTroubleshootingTitle: "Find the shortest path",
      overviewTroubleshootingCopy: "Use the first-line support notes before you dive into deeper runtime or packaging details.",
      operationsCopy: "Use this when you need the runtime, stack, or operator path explained as a coherent workflow.",
      troubleshootingCopy: "Use the shortest failure path before you dig into full logs or packaging output.",
      accuracyTitle: "Read the trust story",
      accuracyCopy: "These are the documents that explain what the current line is trying to verify and why that matters before moxi.",
      accuracyPlanTitle: "Accuracy plan",
      accuracyPlanCopy: "See the long-line plan for verified baselines, benchmark expansion, and solver trust.",
      accuracyBaselinesTitle: "Accuracy baselines",
      accuracyBaselinesCopy: "Read which benchmark families are already locked into regression and which are still next.",
    },
    assistant: {
      close: "Close",
      engine: "Engine",
      localMode: "Local guide",
      llmMode: "Model assist",
      section: "Section",
      runtime: "Runtime",
      bundle: "Bundle",
      quickStart: "Start local stack",
      quickLibrary: "Open library",
      quickBundles: "Inspect bundle",
      quickGuides: "Open guides",
      ask: "Ask",
      docs: "Docs & guides",
      docsOperationsCopy: "Read the runtime and operator workflow.",
      docsTroubleshootingCopy: "Jump to the first-line support notes.",
      baseUrl: "Base URL",
      apiKey: "API key",
      preset: "Preset",
      model: "Model",
      request: "Request",
      generate: "Generate plan",
      approve: "I reviewed this plan and allow execution.",
      execute: "Execute plan",
      ready: "Assistant is ready.",
      audit: "Assistant audit",
    },
    dynamic: {
      focusedBundleField: "focused the bundle path field",
      focusedGuidesPage: "focused the guides page",
      noLogLines: "No log lines yet for {service}.",
      hotStatusRefreshed: "refreshed hot-reload runtime status",
      hotLogRefreshed: "refreshed hot log: {service}",
      hotLogCopied: "copied sanitized hot log tail: {service}",
      runtimeLogRefreshed: "refreshed runtime log: {service}",
      runtimeLogCopied: "copied sanitized runtime log tail: {service}",
      hotLogCleared: "cleared hot log view: {service}",
      packagingRefreshed: "refreshed desktop packaging readiness",
      focusedHomePage: "focused {page} home page",
      openedHomeTarget: "opened {page} from home",
      focusedPanelPage: "focused {page} {group} page",
      assistantPanelOpened: "opened assistant panel",
      assistantPanelClosed: "closed assistant panel",
      runtimeWatchPlaceholder: "Runtime watch will appear here.",
      endpointPolicyAllowed: "Assistant endpoint looks allowed. The API key is sent directly to the configured base URL for plan generation.",
      cardBundlePathTitle: "Start with a bundle path",
      cardBundlePathSummary: "Paste a .kyuubiki path first so the Hub can inspect, validate, or normalize it safely.",
      cardStartLocalTitle: "Bring the local stack online",
      cardStartLocalSummary: "The Hub does not currently see a healthy local runtime, so starting the local stack is the safest next step.",
      cardInspectTitle: "Inspect the selected bundle",
      cardInspectSummary: "Inspecting first gives a quick structural read before we normalize, unpack, or diff anything.",
      cardNormalizeTitle: "Normalize into the target path",
      cardNormalizeSummary: "You already have both the source and output path, so normalization is ready to run.",
      cardDiffTitle: "Compare the current pair",
      cardDiffSummary: "Both bundle inputs are present, so the Hub can run a safe diff without more setup.",
      cardGuidesTitle: "Keep the docs shelf nearby",
      cardGuidesSummary: "If you are still orienting yourself, the Guides page is the cleanest single entry to current-line, operations, troubleshooting, and accuracy notes.",
      cardWorkbenchTitle: "Jump into Workbench",
      cardWorkbenchSummary: "Open the modeling and analysis surface when you are ready to move past bundle-level prep.",
      actionRun: "Run action",
      actionRestore: "Restore",
      actionRerun: "Re-run",
      actionPinned: "Pinned",
      actionPin: "Pin",
      actionLabel: "Label",
      actionCopyCli: "Copy CLI",
      actionCopyPython: "Copy Python",
      recentEntriesEmpty: "No recent entries yet.",
      favoritesEmpty: "No favorite actions yet.",
      recentActionsEmpty: "No recent project actions yet.",
      favoritesFilterEmpty: "No favorites match the {filter} filter.",
      actionsFilterEmpty: "No actions match the {filter} filter.",
      pinnedFavoritesEmpty: "No pinned favorites yet.",
      nonPinnedEmpty: "No non-pinned actions in this view.",
      managedWorkloadsEmpty: "No managed workloads yet. Register a current bundle or sync a remote catalog.",
      managedWorkloadsFilterEmpty: "No workloads match {domain} / {family} for \"{query}\".",
      workflowCatalogEmpty: "No named workflows are available yet.",
      workflowCatalogNoSearchMatches: "No workflows match \"{query}\" right now.",
      workflowCatalogSuggestedBadge: "score {score}",
      workflowCatalogMatchedTerms: "Matched: {terms}",
      workflowCatalogLoading: "Loading workflow catalog…",
      workflowCatalogLoaded: "loaded {count} named workflows into the Hub catalog",
      workflowCatalogRun: "Run reference sample",
      workflowCatalogUnsupported: "No Hub reference sample is defined for {workflow} yet.",
      workflowCatalogEntryInputs: "Entry inputs: {inputs}",
      workflowCatalogOutputs: "Outputs: {outputs}",
      workflowCatalogQueued: "queued workflow {workflow} as job {job}",
      workflowCatalogCompleted: "completed workflow {workflow} with {count} nodes · {summary}",
      workflowCatalogPolling: "waiting for workflow job {job}",
      workflowCatalogFailed: "workflow {workflow} finished with status {status}",
      restoredActionContext: "restored {action} context",
      restoredWorkloadContext: "restored workload context for {label}",
      loadedWorkloadContext: "loaded {label} into the bundle path",
      removedWorkload: "removed {label} from the workload library",
      workloadUse: "Use",
      workloadOpenWorkbench: "Open in Workbench",
      workloadInspect: "Inspect",
      workloadValidate: "Validate",
      workloadDownload: "Download",
      workloadReattach: "Reattach bundle",
      workloadAttach: "Attach current bundle",
      workloadRemove: "Remove",
      noRationale: "No rationale supplied.",
      modelPlanTitle: "Model plan",
    },
  },
  zh: {
    nav: {
      projects: "首页",
      runtimes: "运行时",
      deploy: "部署",
      observe: "观察",
      tools: "工具",
    },
    sections: {
      projects: {
        title: "首页",
        copy: "先走一条清晰路径：把工作带进来，检查一次，再进入 Workbench。",
      },
      runtimes: {
        title: "运行时",
        copy: "启动正确的 loop，确认运行时健康，并把日志放在手边。",
      },
      deploy: {
        title: "部署",
        copy: "选择目标姿态，验证工作站，并准备发布路径。",
      },
      observe: {
        title: "观察",
        copy: "不用离开桌面壳，就能浏览健康状态、日志尾部和最近风险信号。",
      },
      tools: {
        title: "工具",
        copy: "从一个操作面完成诊断、打包和验证。",
      },
    },
    shell: {
      language: "语言",
      actionStatus: "动作状态",
      idle: "空闲",
      openWorkbench: "打开 Workbench",
      startLocal: "启动本地栈",
      validateEnv: "验证环境",
      focus: "运行时编排",
    },
    signals: {
      intakeLabel: "工作入口",
      intakeTitle: "本地 + 远端",
      intakeCopy: "注册 bundle、同步 catalog，并把工作统一放在一个架子上。",
      domainsLabel: "分析域",
      domainsTitle: "力学 / 热 / 力热",
      domainsCopy: "同一套 study 语言现在贯穿 Hub、CLI 和 Workbench。",
      firstMoveLabel: "推荐起手",
      firstMoveTitle: "同步、检查、打开",
      firstMoveCopy: "先同步，再检查一次，然后再打开 Workbench。",
    },
    home: {
      tabs: {
        start: "从这里开始",
        library: "库",
        bundles: "Bundle 工具",
        guides: "文档",
      },
      steps: {
        step1Label: "步骤 1",
        step1Title: "先把工作带进来",
        step1Copy: "注册当前 bundle、同步本地 control plane，或者把远端 catalog 拉进同一个工作架。",
        step2Label: "步骤 2",
        step2Title: "先检查一次",
        step2Copy: "在继续深入前，先验证 bundle 结构和分析意图。",
        step3Label: "步骤 3",
        step3Title: "打开 Workbench",
        step3Copy: "只有在当前 bundle 和运行时路径看起来安全时，再进入分析界面。",
      },
      path: {
        label: "推荐路径",
        title: "把 Hub 当成短跑道",
        copy: "如果这是一次新会话，先走一条短路径，而不是在所有 section 之间来回跳。",
      },
      flow: {
        title1: "需要时先启动本地栈",
        copy1: "在检查或打开依赖本地运行时的内容前，先把本地 loop 带起来。",
        title2: "同步或注册工作",
        copy2: "从本地 control plane、远端 catalog，或当前 bundle path 拉工作进来。",
        title3: "检查一次，再打开",
        copy3: "先做一次快速 bundle 检查，再进入 Workbench，减少意外。",
      },
      quick: {
        label: "快速导览",
        title: "选对下一步",
        copy: "只有在你知道当前要处理哪类工作时，再打开更深的页面。",
        libraryTitle: "打开库",
        libraryCopy: "整理工作负载、同步 catalog，并按域或 family 过滤。",
        bundlesTitle: "打开 Bundle 工具",
        bundlesCopy: "检查、验证、规范化、打包、解包并 diff 项目 bundle。",
        guidesTitle: "打开文档与指南",
        guidesCopy: "进入一个清晰的文档架，查看 current-line、operations、troubleshooting 和 accuracy 说明。",
        installerTitle: "打开 Installer",
        installerCopy: "从一个壳里完成发布布局、桌面打包和工作站 setup。",
        runtimesTitle: "打开运行时",
        runtimesCopy: "检查栈健康、热重载状态和运行时 tail。",
      },
      actions: {
        start: "启动本地栈",
        sync: "同步本地 control plane",
        open: "打开 Workbench",
      },
    },
    panels: {
      runtimes: {
        tabs: ["本地运行时", "热重载", "远端目标"],
        overview: [
          { label: "运行时姿态", title: "先看本地栈", copy: "在深入日志或热重载细节前，先把 frontend、control plane 和 agents 放在眼前。" },
          { label: "热重载", title: "不离开 Hub 做开发", copy: "启动需要的模式，然后在同一个稳定控制面查看当前日志尾部。" },
          { label: "目标", title: "保持扩展视野", copy: "即使日常路径是单机，远端集群和 mesh lab 也仍然留在同一张 runtime 地图里。" },
        ],
        local: {
          label: "本地托管 loop",
          title: "本地运行时",
          copy: "让工作站路径保持可读：先看状态，再看 URL，诊断信息放在附近。",
          status: "状态",
          frontend: "前端",
          controlPlane: "控制面",
          agents: "代理",
        },
        hot: {
          label: "开发运行时",
          title: "热重载 loop",
          copy: "启动需要的开发模式，然后把状态、跟随状态和 tail 输出放在同一个地方。",
          status: "状态",
          mode: "模式",
          local: "本地热重载",
          cloud: "云端热重载",
          distributed: "分布式热重载",
          refreshStatus: "刷新热重载状态",
          stop: "停止热重载 loop",
          logs: "热重载日志",
          autoRefresh: "自动刷新",
          interval: "间隔",
          refreshLog: "刷新日志",
          copyTail: "复制 tail",
          clearView: "清空视图",
          note: "复制 tail 会导出净化后的视图，而不是原始日志文件。",
        },
        targets: {
          label: "运行时地图",
          title: "远端目标",
          copy: "即使主要发布路径仍在单机上，也要把分布式全景保持可见。",
        },
      },
      deploy: {
        tabs: ["模式", "预备", "发布路径"],
        modes: {
          label: "运行时定位",
          title: "部署模式",
          copy: "选择与当前环境匹配的运行姿态，然后无需离开 Hub 就能重启或切换。",
          local: "本地工作站",
          cloud: "云 control plane",
          distributed: "分布式 control plane",
          restart: "重启本地栈",
        },
        bootstrap: {
          label: "发布预备",
          title: "预备",
          copy: "在 Hub 里做验证和就绪检查，然后进入 Installer 处理部署和打包工作。",
          validate: "验证环境",
          stage: "打开 Installer",
          doctor: "运行 doctor",
        },
        release: {
          label: "推荐部署路径",
          title: "模式、验证、交接",
          copy: "先切运行姿态，再验证，然后交给 Installer 处理打包、修复和发布工作。",
        },
      },
      observe: {
        tabs: ["健康", "运行时观察", "栈观察"],
        overview: [
          { label: "信号质量", title: "先看健康，再看细节", copy: "在进入 tail 输出前，先让 watchdog、最近安全事件和失败任务保持可读。" },
          { label: "实时观察", title: "镜像运行时所见", copy: "Observe 镜像会贴着 live runtime loop，这样各个 section 的观察面就能保持一致。" },
          { label: "操作员路径", title: "刷新、复制、继续前进", copy: "Observe 用来浏览和复制；真正要改 loop 时，再去 Runtimes。" },
        ],
        health: {
          label: "稳定性",
          title: "健康与 watchdog",
          copy: "把短健康故事保持紧凑：运行时是否稳定、最近安全信号、失败是否真的在累积。",
          watchdog: "Watchdog",
          security: "安全事件",
          failures: "失败任务",
        },
        runtime: {
          title: "运行时观察",
          localRuntime: "本地运行时",
          hotLoop: "热重载 loop",
          mode: "模式",
          logSource: "日志来源",
          open: "打开运行时",
          refresh: "刷新 tail",
          copy: "复制 tail",
        },
        stack: {
          title: "栈观察",
          logs: "栈日志",
          auto: "自动观察",
          refresh: "刷新 tail",
          copy: "复制 tail",
          note: "复制 tail 会导出净化后的视图，而不是原始日志文件。",
        },
      },
      tools: {
        tabs: ["打包", "状态", "输出"],
        overview: [
          { label: "诊断", title: "先读平台状态", copy: "桌面就绪度、benchmark 和 bundle 验证会先显示出来，再决定是否把更重的打包工作交给 Installer。" },
          { label: "交给 Installer", title: "这里看就绪，那边做重动作", copy: "Hub 负责可见性和短检查，Installer 负责 stage、build、verify 和 cleanup 流程。" },
          { label: "操作输出", title: "让日志和结果留在手边", copy: "状态和操作输出放在一起，让交接判断时更容易审阅。" },
        ],
        packages: {
          label: "平台检查",
          title: "诊断与就绪度",
          copy: "Hub 负责本地诊断、bundle 验证和桌面就绪度；更重的打包工作交给 Installer。",
          platform: "目标平台",
          benchmark: "运行 benchmark",
          validate: "验证项目 bundle",
          export: "导出数据库",
          status: "桌面状态",
          stage: "打开 Installer",
          stop: "停止本地栈",
        },
        status: {
          label: "就绪度",
          title: "桌面状态",
          copy: "把这份输出当成短就绪墙，再决定是否把打包、修复或验证工作交给 Installer。",
        },
        output: {
          label: "操作输出",
          title: "工具输出",
          copy: "让诊断和辅助输出留在手边，这样交给 Installer 前后都容易检查。",
        },
      },
    },
    library: {
      introLabel: "统一入口",
      introTitle: "工作负载库",
      introCopy: "把下载的 bundle、单独导入项以及未来服务端分发的工作负载统一放进 Hub 管理的库里。",
      catalogUrl: "Catalog 地址",
      labelOrNote: "标签或备注",
      register: "注册当前 bundle",
      syncLocal: "同步本地 control plane",
      syncRemote: "同步远端 catalog",
      export: "导出库 JSON",
      import: "导入库 JSON",
    clear: "清空库",
    all: "全部",
    mechanical: "力学",
    thermal: "热",
    thermo: "力热",
    allFamilies: "全部 family",
    axial: "轴向与弹簧",
    beams: "梁与刚架",
    trusses: "桁架",
    planes: "平面",
    workloadSearchLabel: "搜索工作负载",
    workloadSearchPlaceholder: "bridge frame axial thermal",
    workloadSearchClear: "清空搜索",
    workflowCatalogSearchLabel: "搜索工作流",
    workflowCatalogSearchPlaceholder: "bridge thermal export",
    workflowCatalogSearchClear: "清空搜索",
    workflowCatalogRefresh: "刷新工作流目录",
    workflowCatalogReady: "工作流目录已就绪。",
    ready: "工作负载库已就绪。",
  },
    bundles: {
      all: "全部",
      failed: "失败",
      keepFailed: "只保留失败项",
      import: "导入 JSON",
      export: "导出 JSON",
      clear: "清空历史",
      recent: "最近",
    },
    guides: {
      overviewTroubleshootingTitle: "找到最短路径",
      overviewTroubleshootingCopy: "先走第一线支持说明，再决定是否深入 runtime 或打包细节。",
      operationsCopy: "当你需要把 runtime、stack 或 operator 路径看成一条完整工作流时，用这份文档。",
      troubleshootingCopy: "先走最短的失败路径，再决定是否深挖完整日志或打包输出。",
      accuracyTitle: "阅读信任路径",
      accuracyCopy: "这些文档解释当前版本线正在验证什么，以及为什么这在 moxi 之前很重要。",
      accuracyPlanTitle: "精度计划",
      accuracyPlanCopy: "看 verified baselines、benchmark 扩展和 solver 信任链的长期计划。",
      accuracyBaselinesTitle: "精度基线",
      accuracyBaselinesCopy: "查看哪些 benchmark family 已经锁进回归，哪些还在下一步。",
    },
    assistant: {
      close: "关闭",
      engine: "引擎",
      localMode: "本地向导",
      llmMode: "模型助手",
      section: "页面",
      runtime: "运行时",
      bundle: "Bundle",
      quickStart: "启动本地栈",
      quickLibrary: "打开库",
      quickBundles: "检查 bundle",
      quickGuides: "打开指南",
      ask: "提问",
      docs: "文档与指南",
      docsOperationsCopy: "查看运行时和 operator 工作流。",
      docsTroubleshootingCopy: "跳到第一线支持说明。",
      baseUrl: "Base URL",
      apiKey: "API key",
      preset: "预设",
      model: "模型",
      request: "请求",
      generate: "生成计划",
      approve: "我已阅读此计划并允许执行。",
      execute: "执行计划",
      ready: "助手已就绪。",
      audit: "助手审计",
    },
    dynamic: {
      focusedBundleField: "已聚焦到 bundle 路径输入框",
      focusedGuidesPage: "已打开指南页面",
      noLogLines: "{service} 目前还没有日志。",
      hotStatusRefreshed: "已刷新热重载运行时状态",
      hotLogRefreshed: "已刷新热日志：{service}",
      hotLogCopied: "已复制热日志尾部（脱敏）：{service}",
      runtimeLogRefreshed: "已刷新运行时日志：{service}",
      runtimeLogCopied: "已复制运行时日志尾部（脱敏）：{service}",
      hotLogCleared: "已清空热日志视图：{service}",
      packagingRefreshed: "已刷新桌面打包就绪状态",
      focusedHomePage: "已聚焦到首页子页：{page}",
      openedHomeTarget: "已从首页打开：{page}",
      focusedPanelPage: "已聚焦到 {group} / {page} 页面",
      assistantPanelOpened: "已打开助手面板",
      assistantPanelClosed: "已关闭助手面板",
      runtimeWatchPlaceholder: "运行时监视会显示在这里。",
      endpointPolicyAllowed: "当前助手端点看起来可用。API key 会直接发送到配置的 Base URL 以生成计划。",
      cardBundlePathTitle: "先填入 bundle 路径",
      cardBundlePathSummary: "先粘贴一个 .kyuubiki 路径，这样 Hub 才能安全地检查、验证或规范化它。",
      cardStartLocalTitle: "先让本地栈上线",
      cardStartLocalSummary: "Hub 当前还没有看到健康的本地运行时，所以启动本地栈是最安全的下一步。",
      cardInspectTitle: "检查当前 bundle",
      cardInspectSummary: "先做一次检查，可以在规范化、解包或对比之前拿到一个快速结构读取。",
      cardNormalizeTitle: "规范化到目标路径",
      cardNormalizeSummary: "你现在已经同时有源路径和输出路径了，所以可以直接做规范化。",
      cardDiffTitle: "对比当前这一对 bundle",
      cardDiffSummary: "两个 bundle 输入都已经具备，所以 Hub 可以直接执行一次安全 diff。",
      cardGuidesTitle: "把文档架放在手边",
      cardGuidesSummary: "如果你还在找方向，Guides 是 current-line、operations、troubleshooting 和 accuracy notes 最清晰的单一入口。",
      cardWorkbenchTitle: "进入 Workbench",
      cardWorkbenchSummary: "当你准备从 bundle 层面的准备进入建模和分析时，就打开 Workbench。",
      actionRun: "执行动作",
      actionRestore: "恢复",
      actionRerun: "重新运行",
      actionPinned: "已置顶",
      actionPin: "置顶",
      actionLabel: "命名",
      actionCopyCli: "复制 CLI",
      actionCopyPython: "复制 Python",
      recentEntriesEmpty: "还没有最近记录。",
      favoritesEmpty: "还没有收藏动作。",
      recentActionsEmpty: "还没有最近的项目动作。",
      favoritesFilterEmpty: "没有收藏动作符合 {filter} 筛选。",
      actionsFilterEmpty: "没有动作符合 {filter} 筛选。",
      pinnedFavoritesEmpty: "还没有置顶收藏。",
      nonPinnedEmpty: "当前视图里没有未置顶动作。",
      managedWorkloadsEmpty: "还没有已管理工作负载。先注册当前 bundle，或同步远端 catalog。",
      managedWorkloadsFilterEmpty: "没有工作负载在 {domain} / {family} 下匹配“{query}”。",
      workflowCatalogEmpty: "当前还没有可用的命名工作流。",
      workflowCatalogNoSearchMatches: "当前没有工作流匹配“{query}”。",
      workflowCatalogSuggestedBadge: "评分 {score}",
      workflowCatalogMatchedTerms: "命中词：{terms}",
      workflowCatalogLoading: "正在加载工作流目录……",
      workflowCatalogLoaded: "已将 {count} 条命名工作流载入 Hub 目录",
      workflowCatalogRun: "运行参考样板",
      workflowCatalogUnsupported: "Hub 里还没有为 {workflow} 定义参考样板。",
      workflowCatalogEntryInputs: "入口输入：{inputs}",
      workflowCatalogOutputs: "输出产物：{outputs}",
      workflowCatalogQueued: "已把工作流 {workflow} 排入队列，job 为 {job}",
      workflowCatalogCompleted: "工作流 {workflow} 已完成，共跑完 {count} 个节点 · {summary}",
      workflowCatalogPolling: "正在等待 workflow job {job}",
      workflowCatalogFailed: "工作流 {workflow} 结束状态为 {status}",
      restoredActionContext: "已恢复 {action} 上下文",
      restoredWorkloadContext: "已恢复 {label} 的工作负载上下文",
      loadedWorkloadContext: "已把 {label} 载入 bundle 路径",
      removedWorkload: "已从工作负载库移除 {label}",
      workloadUse: "使用",
      workloadOpenWorkbench: "在 Workbench 中打开",
      workloadInspect: "检查",
      workloadValidate: "验证",
      workloadDownload: "下载",
      workloadReattach: "重新关联 bundle",
      workloadAttach: "关联当前 bundle",
      workloadRemove: "移除",
      noRationale: "没有附带说明。",
      modelPlanTitle: "模型计划",
    },
  },
  ja: {
    nav: {
      projects: "ホーム",
      runtimes: "ランタイム",
      deploy: "デプロイ",
      observe: "観察",
      tools: "ツール",
    },
    sections: {
      projects: {
        title: "ホーム",
        copy: "まずは一本のわかりやすい流れで進みます。作業を取り込み、一度確認してから Workbench に入ります。",
      },
      runtimes: {
        title: "ランタイム",
        copy: "適切な loop を起動し、状態を確認し、ログを近くに置きます。",
      },
      deploy: {
        title: "デプロイ",
        copy: "対象の姿勢を選び、ワークステーションを検証し、リリース経路を整えます。",
      },
      observe: {
        title: "観察",
        copy: "デスクトップシェルから離れずに、ヘルス、ログの末尾、最近のリスク信号を確認します。",
      },
      tools: {
        title: "ツール",
        copy: "診断、パッケージング、検証を一つの操作面から行います。",
      },
    },
    shell: {
      language: "言語",
      actionStatus: "動作状態",
      idle: "待機",
      openWorkbench: "Workbench を開く",
      startLocal: "ローカルスタックを起動",
      validateEnv: "環境を確認",
      focus: "ランタイム運用",
    },
    signals: {
      intakeLabel: "作業の取り込み",
      intakeTitle: "ローカル + リモート",
      intakeCopy: "bundle を登録し、catalog を同期し、作業を一つの棚にまとめます。",
      domainsLabel: "解析ドメイン",
      domainsTitle: "mechanical / thermal / thermo",
      domainsCopy: "同じ study の言語が Hub、CLI、Workbench を通して流れます。",
      firstMoveLabel: "推奨の最初の一手",
      firstMoveTitle: "同期、確認、起動",
      firstMoveCopy: "まず同期し、一度確認してから Workbench を開きます。",
    },
    home: {
      tabs: {
        start: "ここから開始",
        library: "ライブラリ",
        bundles: "Bundle ツール",
        guides: "ガイド",
      },
      steps: {
        step1Label: "ステップ 1",
        step1Title: "作業を取り込む",
        step1Copy: "現在の bundle を登録し、ローカル control plane を同期するか、リモート catalog を同じ棚に取り込みます。",
        step2Label: "ステップ 2",
        step2Title: "一度確認する",
        step2Copy: "深く進む前に、bundle の形と分析意図を確認します。",
        step3Label: "ステップ 3",
        step3Title: "Workbench を開く",
        step3Copy: "現在の bundle と runtime が安全に見えてから解析に入ります。",
      },
      path: {
        label: "推奨パス",
        title: "Hub を短い滑走路として使う",
        copy: "新しいセッションなら、すべての section を行き来する前に一本の短い流れで進みます。",
      },
      flow: {
        title1: "必要なら先にローカルスタックを起動",
        copy1: "ローカル runtime に依存するものを確認したり開いたりする前に、先に loop を起こします。",
        title2: "作業を同期または登録",
        copy2: "ローカル control plane、リモート catalog、または現在の bundle path から取り込みます。",
        title3: "一度確認してから開く",
        copy3: "まず短い bundle チェックを行い、その後 Workbench に入って surprises を減らします。",
      },
      quick: {
        label: "クイック案内",
        title: "次の一手を選ぶ",
        copy: "今どの仕事をしているかが見えたときだけ、より深いページを開きます。",
        libraryTitle: "ライブラリを開く",
        libraryCopy: "workload を整理し、catalog を同期し、ドメインや family で絞り込みます。",
        bundlesTitle: "Bundle ツールを開く",
        bundlesCopy: "project bundle の inspect、validate、normalize、pack、unpack、diff を行います。",
        guidesTitle: "ドキュメントとガイドを開く",
        guidesCopy: "current-line、operations、troubleshooting、accuracy の文書棚へ進みます。",
        installerTitle: "Installer を開く",
        installerCopy: "一つの shell から release layout、desktop packaging、workstation setup を進めます。",
        runtimesTitle: "Runtimes を開く",
        runtimesCopy: "stack の健康、hot-reload 状態、runtime tail を確認します。",
      },
      actions: {
        start: "ローカルスタックを起動",
        sync: "ローカル control plane を同期",
        open: "Workbench を開く",
      },
    },
    panels: {
      runtimes: {
        tabs: ["ローカル runtime", "ホット loop", "リモート targets"],
        overview: [
          { label: "ランタイム姿勢", title: "まずローカルスタックを見る", copy: "ログや hot-reload の細部に入る前に、frontend、control plane、agents を見える状態にします。" },
          { label: "ホット loop", title: "Hub を離れずに開発", copy: "必要なモードを起動し、同じ安定した操作面から現在の tail を確認します。" },
          { label: "ターゲット", title: "拡張先を見失わない", copy: "日常の経路がローカルでも、リモート cluster と mesh lab は同じ runtime map に残ります。" },
        ],
        local: {
          label: "ローカル管理 loop",
          title: "ローカル runtime",
          copy: "ワークステーションの経路を読みやすく保ちます。まず状態、その次に URL、診断は近くに。",
          status: "状態",
          frontend: "フロントエンド",
          controlPlane: "コントロールプレーン",
          agents: "エージェント",
        },
        hot: {
          label: "開発 runtime",
          title: "ホットリロード loop",
          copy: "必要な開発モードを起動し、状態、追従状態、tail 出力を一か所にまとめます。",
          status: "状態",
          mode: "モード",
          local: "ローカル hot",
          cloud: "クラウド hot",
          distributed: "分散 hot",
          refreshStatus: "hot 状態を更新",
          stop: "hot loop を停止",
          logs: "hot ログ",
          autoRefresh: "自動更新",
          interval: "間隔",
          refreshLog: "ログ更新",
          copyTail: "tail をコピー",
          clearView: "表示をクリア",
          note: "Copy tail は生ログではなく、整えたビューを書き出します。",
        },
        targets: {
          label: "ランタイムマップ",
          title: "リモート targets",
          copy: "主要なリリース経路が一台のワークステーションでも、分散の全体像を見失わないようにします。",
        },
      },
      deploy: {
        tabs: ["モード", "ブートストラップ", "リリース経路"],
        modes: {
          label: "ランタイム選定",
          title: "デプロイモード",
          copy: "現在の環境に合う運用姿勢を選び、Hub を離れずに再起動や切り替えを行います。",
          local: "ローカル workstation",
          cloud: "クラウド control plane",
          distributed: "分散 control plane",
          restart: "ローカル stack を再起動",
        },
        bootstrap: {
          label: "リリース準備",
          title: "ブートストラップ",
          copy: "Hub で検証と readiness を確認し、その後 Installer で deployment と packaging を進めます。",
          validate: "環境を確認",
          stage: "Installer を開く",
          doctor: "doctor を実行",
        },
        release: {
          label: "推奨デプロイ経路",
          title: "モード、検証、引き継ぎ",
          copy: "まず姿勢を切り替え、次に確認し、その後 Installer に packaging・repair・release 作業を引き渡します。",
        },
      },
      observe: {
        tabs: ["ヘルス", "ランタイム監視", "スタック監視"],
        overview: [
          { label: "信号品質", title: "まずヘルス、次に詳細", copy: "tail 出力に入る前に、watchdog、最近の security event、失敗ジョブを読みやすく保ちます。" },
          { label: "ライブ監視", title: "ランタイムが見ているものを映す", copy: "Observe のミラーは live runtime loop に寄り添い、各 section の監視面を揃えます。" },
          { label: "オペレータの流れ", title: "更新して、コピーして、次へ進む", copy: "Observe は確認とコピー用、loop を変えるときは Runtimes に戻ります。" },
        ],
        health: {
          label: "安定性",
          title: "ヘルスと watchdog",
          copy: "短いヘルスの物語を保ちます。runtime の安定性、最近の security signal、失敗の蓄積が一目でわかるようにします。",
          watchdog: "Watchdog",
          security: "セキュリティイベント",
          failures: "失敗ジョブ",
        },
        runtime: {
          title: "ランタイム監視",
          localRuntime: "ローカル runtime",
          hotLoop: "ホット loop",
          mode: "モード",
          logSource: "ログソース",
          open: "Runtimes を開く",
          refresh: "tail を更新",
          copy: "tail をコピー",
        },
        stack: {
          title: "スタック監視",
          logs: "スタックログ",
          auto: "自動監視",
          refresh: "tail を更新",
          copy: "tail をコピー",
          note: "Copy tail は生ログではなく、整えたビューを書き出します。",
        },
      },
      tools: {
        tabs: ["パッケージ", "状態", "出力"],
        overview: [
          { label: "診断", title: "まずプラットフォームを読む", copy: "desktop readiness、benchmark、bundle validation を先に見て、重い packaging 作業を Installer に渡す前提を保ちます。" },
          { label: "Installer への引き継ぎ", title: "ここで readiness、向こうで重い作業", copy: "Hub は可視化と短いチェック、Installer は stage・build・verify・cleanup を担当します。" },
          { label: "操作出力", title: "ログと結果を近くに置く", copy: "status と operation output を近くに置き、引き継ぎ判断を監査しやすくします。" },
        ],
        packages: {
          label: "プラットフォーム確認",
          title: "診断と readiness",
          copy: "Hub ではローカル診断、bundle validation、desktop readiness を扱い、重い packaging 作業は Installer に渡します。",
          platform: "対象プラットフォーム",
          benchmark: "benchmark を実行",
          validate: "project bundle を検証",
          export: "データベースを出力",
          status: "desktop status",
          stage: "Installer を開く",
          stop: "ローカル stack を停止",
        },
        status: {
          label: "準備状況",
          title: "Desktop status",
          copy: "packaging・repair・verification を Installer に渡す前に、この出力を短い readiness wall として使います。",
        },
        output: {
          label: "操作出力",
          title: "Tool output",
          copy: "diagnostics と補助出力を近くに置き、Installer への引き継ぎ判断を確認しやすくします。",
        },
      },
    },
    library: {
      introLabel: "取り込みの管理",
      introTitle: "ワークロードライブラリ",
      introCopy: "ダウンロード済み bundle、単独インポート、将来のサーバ配信 workload を Hub 管理の一つのライブラリにまとめます。",
      catalogUrl: "Catalog URL",
      labelOrNote: "ラベルまたはメモ",
      register: "現在の bundle を登録",
      syncLocal: "ローカル control plane を同期",
      syncRemote: "リモート catalog を同期",
      export: "ライブラリ JSON を出力",
      import: "ライブラリ JSON を読み込み",
      clear: "ライブラリをクリア",
      all: "すべて",
      mechanical: "力学",
      thermal: "熱",
      thermo: "熱・構造",
      allFamilies: "全 family",
      axial: "軸・ばね",
      beams: "梁・フレーム",
      trusses: "トラス",
      planes: "平面",
      workloadSearchLabel: "workload を検索",
      workloadSearchPlaceholder: "bridge frame axial thermal",
      workloadSearchClear: "検索をクリア",
      workflowCatalogSearchLabel: "workflow を検索",
      workflowCatalogSearchPlaceholder: "bridge thermal export",
      workflowCatalogSearchClear: "検索をクリア",
      workflowCatalogRefresh: "workflow catalog を更新",
      workflowCatalogReady: "workflow catalog の準備ができました。",
      ready: "ワークロードライブラリの準備ができました。",
    },
    bundles: {
      all: "すべて",
      failed: "失敗",
      keepFailed: "失敗だけ残す",
      import: "JSON を読み込み",
      export: "JSON を出力",
      clear: "履歴をクリア",
      recent: "最近",
    },
    guides: {
      overviewTroubleshootingTitle: "最短経路を見つける",
      overviewTroubleshootingCopy: "より深い runtime や packaging の詳細に入る前に、第一線のサポートノートを使います。",
      operationsCopy: "runtime、stack、operator path を一つの流れとして理解したいときに使います。",
      troubleshootingCopy: "完全なログや packaging 出力へ潜る前に、最短の失敗経路を使います。",
      accuracyTitle: "信頼の筋道を読む",
      accuracyCopy: "これらの文書は、現在のバージョンラインが何を検証しようとしているのか、そしてそれが moxi の前に重要である理由を説明します。",
      accuracyPlanTitle: "Accuracy plan",
      accuracyPlanCopy: "verified baseline、benchmark 拡張、solver trust の長期計画を確認します。",
      accuracyBaselinesTitle: "Accuracy baselines",
      accuracyBaselinesCopy: "どの benchmark family が回帰に固定され、どれが次なのかを確認します。",
    },
    assistant: {
      close: "閉じる",
      engine: "エンジン",
      localMode: "ローカルガイド",
      llmMode: "モデル補助",
      section: "セクション",
      runtime: "ランタイム",
      bundle: "Bundle",
      quickStart: "ローカルスタックを起動",
      quickLibrary: "ライブラリを開く",
      quickBundles: "bundle を確認",
      quickGuides: "ガイドを開く",
      ask: "質問",
      docs: "Docs とガイド",
      docsOperationsCopy: "runtime と operator workflow を読みます。",
      docsTroubleshootingCopy: "第一線のサポートノートへ進みます。",
      baseUrl: "Base URL",
      apiKey: "API key",
      preset: "プリセット",
      model: "モデル",
      request: "リクエスト",
      generate: "計画を生成",
      approve: "この計画を確認し、実行を許可します。",
      execute: "計画を実行",
      ready: "アシスタントは準備完了です。",
      audit: "アシスタント監査",
    },
    dynamic: {
      focusedBundleField: "bundle パス入力欄に移動しました",
      focusedGuidesPage: "ガイドページを開きました",
      noLogLines: "{service} にはまだログ行がありません。",
      hotStatusRefreshed: "ホットリロードのランタイム状態を更新しました",
      hotLogRefreshed: "ホットログを更新しました: {service}",
      hotLogCopied: "サニタイズ済みホットログ末尾をコピーしました: {service}",
      runtimeLogRefreshed: "ランタイムログを更新しました: {service}",
      runtimeLogCopied: "サニタイズ済みランタイムログ末尾をコピーしました: {service}",
      hotLogCleared: "ホットログ表示をクリアしました: {service}",
      packagingRefreshed: "デスクトップ packaging readiness を更新しました",
      focusedHomePage: "ホームの {page} ページに移動しました",
      openedHomeTarget: "ホームから {page} を開きました",
      focusedPanelPage: "{group} / {page} ページに移動しました",
      assistantPanelOpened: "アシスタントパネルを開きました",
      assistantPanelClosed: "アシスタントパネルを閉じました",
      runtimeWatchPlaceholder: "ランタイム監視はここに表示されます。",
      endpointPolicyAllowed: "現在のアシスタント endpoint は許可された形に見えます。API key は計画生成のために設定された Base URL に直接送信されます。",
      cardBundlePathTitle: "まず bundle パスを入れる",
      cardBundlePathSummary: "まず .kyuubiki パスを貼り付けると、Hub が安全に inspect・validate・normalize できます。",
      cardStartLocalTitle: "ローカルスタックを先に起動",
      cardStartLocalSummary: "Hub はまだ健全なローカルランタイムを見ていないため、ローカルスタックの起動が最も安全な次の一手です。",
      cardInspectTitle: "選択中の bundle を確認",
      cardInspectSummary: "まず inspect すると、normalize・unpack・diff の前に短い構造読みができます。",
      cardNormalizeTitle: "目標パスへ normalize",
      cardNormalizeSummary: "入力パスと出力パスの両方がそろっているので、そのまま normalize を実行できます。",
      cardDiffTitle: "現在のペアを比較",
      cardDiffSummary: "bundle 入力が両方そろっているため、Hub は追加設定なしで安全な diff を実行できます。",
      cardGuidesTitle: "ドキュメント棚を近くに置く",
      cardGuidesSummary: "まだ方向付けの途中なら、Guides が current-line、operations、troubleshooting、accuracy notes への最も読みやすい単一入口です。",
      cardWorkbenchTitle: "Workbench に入る",
      cardWorkbenchSummary: "bundle レベルの準備から先へ進む準備ができたら、モデリングと解析の面へ移動します。",
      actionRun: "実行",
      noRationale: "理由は添えられていません。",
      modelPlanTitle: "モデル計画",
    },
  },
};

HUB_I18N.es = {
  ...HUB_I18N.en,
  nav: {
    projects: "Inicio",
    runtimes: "Runtimes",
    deploy: "Despliegue",
    observe: "Observación",
    tools: "Herramientas",
  },
  sections: {
    projects: {
      title: "Inicio",
      copy: "Sigue una sola ruta clara: trae trabajo, revísalo una vez y luego entra en Workbench.",
    },
    runtimes: {
      title: "Runtimes",
      copy: "Inicia el bucle correcto, revisa la salud del runtime y mantén los registros cerca.",
    },
    deploy: {
      title: "Despliegue",
      copy: "Elige la postura objetivo, valida la estación y prepara la ruta de publicación.",
    },
    observe: {
      title: "Observación",
      copy: "Revisa salud, colas y señales recientes sin salir del shell de escritorio.",
    },
    tools: {
      title: "Herramientas",
      copy: "Ejecuta diagnósticos, empaquetado y verificación desde una sola superficie.",
    },
  },
  shell: {
    ...HUB_I18N.en.shell,
    language: "Idioma",
    actionStatus: "Estado de acción",
    idle: "inactivo",
    openWorkbench: "Abrir Workbench",
    startLocal: "Iniciar pila local",
    validateEnv: "Validar entorno",
    focus: "orquestación del runtime",
  },
  signals: {
    intakeLabel: "Entrada de cargas",
    intakeTitle: "local + remota",
    intakeCopy: "Registra bundles, sincroniza catálogos y mantén una sola biblioteca a la vista.",
    domainsLabel: "Dominios de análisis",
    domainsTitle: "mecánico / térmico / termo",
    domainsCopy: "El mismo lenguaje de estudios ahora fluye por Hub, CLI y Workbench.",
    firstMoveLabel: "Primer movimiento recomendado",
    firstMoveTitle: "sincronizar, revisar, abrir",
    firstMoveCopy: "Sincroniza primero, revisa una vez y luego abre Workbench.",
  },
  home: {
    ...HUB_I18N.en.home,
    tabs: {
      start: "Empezar aquí",
      library: "Biblioteca",
      bundles: "Herramientas de bundle",
      guides: "Guías",
    },
    steps: {
      step1Label: "PASO 1",
      step1Title: "Traer trabajo",
      step1Copy: "Registra el bundle actual, sincroniza el plano de control local o importa un catálogo remoto.",
      step2Label: "PASO 2",
      step2Title: "Revisar una vez",
      step2Copy: "Valida la forma del bundle y la intención del análisis antes de profundizar.",
      step3Label: "PASO 3",
      step3Title: "Abrir Workbench",
      step3Copy: "Entra al análisis solo cuando el bundle activo y la ruta de runtime se vean seguros.",
    },
    path: {
      label: "Ruta recomendada",
      title: "Usa Hub como pista corta",
      copy: "Si esta es una sesión nueva, sigue una ruta corta en vez de saltar entre secciones.",
    },
    flow: {
      title1: "Inicia la pila local si hace falta",
      copy1: "Pon en línea el runtime local antes de revisar o abrir algo que dependa de él.",
      title2: "Sincroniza o registra trabajo",
      copy2: "Tira desde el plano de control local, un catálogo remoto o la ruta del bundle actual.",
      title3: "Revisa y luego abre",
      copy3: "Haz una revisión rápida del bundle y entra en Workbench con menos sorpresas.",
    },
    quick: {
      label: "Orientación rápida",
      title: "Elige el siguiente paso correcto",
      copy: "Abre páginas más profundas solo cuando sepas qué trabajo estás haciendo.",
      libraryTitle: "Abrir Biblioteca",
      libraryCopy: "Organiza cargas, sincroniza catálogos y filtra por dominio o familia.",
      bundlesTitle: "Abrir herramientas de bundle",
      bundlesCopy: "Inspecciona, valida, normaliza, empaqueta, desempaqueta y compara bundles de proyecto.",
      guidesTitle: "Abrir docs y guías",
      guidesCopy: "Ve a una sola estantería de documentación para línea actual, operaciones, troubleshooting y exactitud.",
      installerTitle: "Abrir Installer",
      installerCopy: "Prepara layouts de release, empaquetado de escritorio y setup de estación desde un solo shell.",
      runtimesTitle: "Abrir Runtimes",
      runtimesCopy: "Revisa la salud de la pila, el estado hot-reload y los tails del runtime.",
    },
    actions: {
      start: "Iniciar pila local",
      sync: "Sincronizar plano de control local",
      open: "Abrir Workbench",
    },
  },
  library: {
    ...HUB_I18N.en.library,
    introLabel: "Entrada gestionada",
    introTitle: "Biblioteca de cargas",
    introCopy: "Mantén bundles descargados, importaciones sueltas y cargas futuras del servidor en una sola biblioteca gestionada por Hub.",
    catalogUrl: "URL del catálogo",
    labelOrNote: "Etiqueta o nota",
    register: "Registrar bundle actual",
    syncLocal: "Sincronizar control local",
    syncRemote: "Sincronizar catálogo remoto",
    export: "Exportar biblioteca JSON",
    import: "Importar biblioteca JSON",
    clear: "Vaciar biblioteca",
    workloadSearchLabel: "Buscar workloads",
    workloadSearchPlaceholder: "bridge frame axial thermal",
    workloadSearchClear: "Limpiar búsqueda",
    workflowCatalogSearchLabel: "Buscar workflows",
    workflowCatalogSearchPlaceholder: "bridge thermal export",
    workflowCatalogSearchClear: "Limpiar búsqueda",
    workflowCatalogRefresh: "Actualizar catálogo de workflows",
    workflowCatalogReady: "El catálogo de workflows está listo.",
  },
  bundles: {
    ...HUB_I18N.en.bundles,
  },
  guides: {
    ...HUB_I18N.en.guides,
  },
  assistant: {
    ...HUB_I18N.en.assistant,
    close: "Cerrar",
    engine: "Motor",
    title: "Asistente",
    localGuide: "Guía local",
    modelAssist: "Asistencia con modelo",
    section: "Sección",
    runtime: "Runtime",
    bundle: "Bundle",
    quickStart: "Iniciar pila local",
    quickLibrary: "Abrir biblioteca",
    quickBundles: "Inspeccionar bundle",
    quickGuides: "Abrir guías",
    ask: "Preguntar",
    askLocal: "Preguntar a la guía local",
    docsGuides: "Docs y guías",
    docsOperationsCopy: "Lee el flujo de runtime y operador.",
    docsTroubleshootingCopy: "Salta a las notas de soporte de primera línea.",
    baseUrl: "URL base",
    apiKey: "Clave API",
    preset: "Preajuste",
    model: "Modelo",
    request: "Solicitud",
    generate: "Generar plan",
    approve: "He revisado este plan y permito su ejecución.",
    execute: "Ejecutar plan",
    ready: "El asistente está listo.",
    audit: "Auditoría del asistente",
  },
  dynamic: {
    ...HUB_I18N.en.dynamic,
    focusedBundleField: "se enfocó el campo de ruta del bundle",
    focusedGuidesPage: "se enfocó la página de guías",
    noLogLines: "Todavía no hay líneas de log para {service}.",
    hotStatusRefreshed: "se actualizó el estado del runtime hot-reload",
    hotLogRefreshed: "se actualizó el log hot: {service}",
    hotLogCopied: "se copió la cola saneada del log hot: {service}",
    runtimeLogRefreshed: "se actualizó el log del runtime: {service}",
    runtimeLogCopied: "se copió la cola saneada del log del runtime: {service}",
    hotLogCleared: "se limpió la vista del log hot: {service}",
    packagingRefreshed: "se actualizó la disponibilidad del empaquetado de escritorio",
    focusedHomePage: "se enfocó la página de inicio {page}",
    openedHomeTarget: "se abrió {page} desde inicio",
    focusedPanelPage: "se enfocó la página {page} de {group}",
    assistantPanelOpened: "se abrió el panel del asistente",
    assistantPanelClosed: "se cerró el panel del asistente",
    runtimeWatchPlaceholder: "La observación del runtime aparecerá aquí.",
    endpointPolicyAllowed: "El endpoint del asistente parece permitido. La API key se envía directamente a la URL base configurada para generar el plan.",
    cardBundlePathTitle: "Empieza con una ruta de bundle",
    cardBundlePathSummary: "Pega primero una ruta .kyuubiki para que Hub pueda inspeccionarla, validarla o normalizarla con seguridad.",
    cardStartLocalTitle: "Poner en línea la pila local",
    cardStartLocalSummary: "Hub no ve ahora mismo un runtime local sano, así que iniciar la pila local es el siguiente paso más seguro.",
    cardInspectTitle: "Inspeccionar el bundle seleccionado",
    cardInspectSummary: "Inspeccionar primero da una lectura estructural rápida antes de normalizar, desempaquetar o comparar nada.",
    cardNormalizeTitle: "Normalizar hacia la ruta de destino",
    cardNormalizeSummary: "Ya tienes tanto la ruta de origen como la de salida, así que la normalización está lista para ejecutarse.",
    cardDiffTitle: "Comparar el par actual",
    cardDiffSummary: "Ambas entradas de bundle están presentes, así que Hub puede ejecutar un diff seguro sin más preparación.",
    cardGuidesTitle: "Mantén cerca la estantería de docs",
    cardGuidesSummary: "Si todavía te estás orientando, la página Guides es la entrada única más limpia para línea actual, operaciones, troubleshooting y notas de exactitud.",
    cardWorkbenchTitle: "Entrar en Workbench",
    cardWorkbenchSummary: "Abre la superficie de modelado y análisis cuando ya estés listo para dejar atrás la preparación a nivel de bundle.",
    actionRun: "Ejecutar acción",
  },
};

applyHubDocsI18n(HUB_I18N);
applyHubGuidesI18n(HUB_I18N);
applyHubLocalizationI18n(HUB_I18N);
applyHubAssistantI18n(HUB_I18N);
applyHubWorkloadsI18n(HUB_I18N);

const HUB_RECENTS_KEY = "kyuubiki.hub.recents.v1";
const HUB_WORKLOAD_LIBRARY_KEY = "kyuubiki.hub.workloads.v1";
const HUB_ASSISTANT_SETTINGS_KEY = "kyuubiki.hub.assistant.settings.v1";
const HUB_ASSISTANT_LEGACY_SECRETS_KEY = "kyuubiki.hub.assistant.secrets.v1";
const HUB_ASSISTANT_AUDIT_KEY = "kyuubiki.hub.assistant.audit.v1";
const HUB_ASSISTANT_TRUSTED_HOSTS_KEY = "kyuubiki.hub.assistant.trusted-hosts.v1";
const HUB_REMOTE_TRUSTED_HOSTS_KEY = "kyuubiki.hub.remote-trusted-hosts.v1";
const HUB_HOT_LOG_SETTINGS_KEY = "kyuubiki.hub.hot-log-settings.v1";
const HUB_RUNTIME_LOG_SETTINGS_KEY = "kyuubiki.hub.runtime-log-settings.v1";
const HUB_DENSITY_SETTINGS_KEY = "kyuubiki.hub.density-settings.v1";
const HUB_RECENTS_LIMIT = 6;
const HUB_ACTION_HISTORY_LIMIT = 8;
const HUB_ASSISTANT_AUDIT_LIMIT = 16;
const HUB_WORKLOAD_LIBRARY_LIMIT = 32;
const HUB_HOT_LOG_POLL_MS = 4000;
const HUB_ASSISTANT_MODEL_PRESETS = ["gpt-5", "gpt-5-mini", "gpt-4.1", "custom"];
const HUB_ASSISTANT_ACTION_RISK = {
  "hub/focusSection": "low",
  "hub/openWorkbench": "low",
  "hub/openInstaller": "sensitive",
  "hub/openDocsIndex": "low",
  "hub/openCurrentLineDoc": "low",
  "hub/openOperationsDoc": "low",
  "hub/openTroubleshootingDoc": "low",
  "hub/startLocal": "sensitive",
  "hub/validateEnv": "low",
  "hub/desktopStage": "sensitive",
  "hub/desktopBuildHost": "high",
  "hub/desktopVerify": "sensitive",
  "hub/setBundleContext": "low",
  "hub/projectInspect": "low",
  "hub/projectValidate": "low",
  "hub/projectNormalize": "sensitive",
  "hub/projectUnpack": "sensitive",
  "hub/projectPack": "high",
  "hub/projectDiff": "low",
};
const PROJECT_ACTION_LABELS = {
  "project inspect": "project-inspect",
  "project validate": "project-validate",
  "project normalize": "project-normalize",
  "project unpack": "project-unpack",
  "project pack": "project-pack",
  "project diff": "project-diff",
};
const HUB_DIRECT_ACTION_RISK = {
  "open-installer": "sensitive",
  "workload-clear-library": "sensitive",
  "start-local": "sensitive",
  "start-cloud": "sensitive",
  "start-distributed": "sensitive",
  "restart-local": "sensitive",
  "stop-stack": "sensitive",
  "hot-start-local": "sensitive",
  "hot-start-cloud": "sensitive",
  "hot-start-distributed": "sensitive",
  "hot-stop": "sensitive",
  "project-normalize": "sensitive",
  "project-unpack": "sensitive",
  "project-pack": "high",
};
const HUB_ASSISTANT_ACTIONS = [
  { id: "hub/focusSection", summary: "Focus a Hub section.", payloadExample: { section: "projects" } },
  { id: "hub/openWorkbench", summary: "Open the Workbench desktop shell.", payloadExample: {} },
  { id: "hub/openInstaller", summary: "Open the Installer desktop shell.", payloadExample: {} },
  { id: "hub/openDocsIndex", summary: "Open the Hub documentation index.", payloadExample: {} },
  { id: "hub/openCurrentLineDoc", summary: "Open the current-line document.", payloadExample: {} },
  { id: "hub/openOperationsDoc", summary: "Open the operations guide.", payloadExample: {} },
  { id: "hub/openTroubleshootingDoc", summary: "Open the troubleshooting guide.", payloadExample: {} },
  { id: "hub/startLocal", summary: "Start the local stack.", payloadExample: {} },
  { id: "hub/validateEnv", summary: "Validate the desktop environment.", payloadExample: {} },
  { id: "hub/desktopStage", summary: "Open Installer for desktop staging work.", payloadExample: {} },
  { id: "hub/desktopBuildHost", summary: "Open Installer for host-bundle build work.", payloadExample: {} },
  { id: "hub/desktopVerify", summary: "Open Installer for desktop verification work.", payloadExample: {} },
  { id: "hub/setBundleContext", summary: "Fill Hub bundle path inputs.", payloadExample: { path: "", comparePath: "", out: "" } },
  { id: "hub/projectInspect", summary: "Inspect the selected project bundle.", payloadExample: { path: "" } },
  { id: "hub/projectValidate", summary: "Validate the selected project bundle.", payloadExample: { path: "" } },
  { id: "hub/projectNormalize", summary: "Normalize the selected project bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectUnpack", summary: "Unpack the selected project bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectPack", summary: "Pack a project directory into a bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectDiff", summary: "Diff two project bundles.", payloadExample: { leftPath: "", rightPath: "" } },
];
const HUB_DENSITY_DEFAULTS = {
  "projects-workflow": false,
  "runtimes-remote-targets": false,
  "deploy-suggested-flow": false,
  "tools-output": false,
  "side-current-mode": false,
};

const state = {
  hostPlatform: detectDesktopPlatform(),
  activeSection: "projects",
  projectsPage: "start",
  panelPages: {
    runtimes: "local",
    deploy: "modes",
    observe: "health",
    tools: "packages",
  },
  assistantOpen: false,
  isBusy: false,
  historyFilter: "all",
  workloadFilter: "all",
  workloadFamilyFilter: "all",
  workflowCatalog: [],
  workflowCatalogBusy: false,
  assistantMode: "local",
  assistantPlan: null,
  hotLogRefreshInFlight: false,
  runtimeLogRefreshInFlight: false,
  density: { ...HUB_DENSITY_DEFAULTS },
  releaseVersion: "",
  releaseCodename: "",
  language: "en",
  directMeshRegressionSnapshot: null,
  regressionGateReport: null,
  assistantApiKey: "",
  assistantTrustedHosts: loadHubAssistantTrustedHosts(),
  remoteTrustedHosts: loadHubTrustedHosts(HUB_REMOTE_TRUSTED_HOSTS_KEY),
};

let hotRuntimeLogPollHandle = null;
let observeRuntimeLogPollHandle = null;

function hubCopy() {
  return resolveHubCopy(HUB_I18N, state.language);
}

function renderToolsPlatformLabel() {
  if (!elements.toolsPackagesPlatformLabel) {
    return;
  }

  const baseLabel = hubCopy().panels.tools.packages.platform;
  const selectedPlatform = elements.releasePlatform?.value || state.hostPlatform;
  elements.toolsPackagesPlatformLabel.textContent = desktopPlatformContextLabel(
    baseLabel,
    normalizeDesktopPlatform(selectedPlatform, state.hostPlatform, {
      allowAll: true,
    }),
    "",
  );
}

async function loadRegressionGateReport() {
  try {
    state.regressionGateReport = await invokeTauri("hub_regression_gate_report");
    renderRegressionGateReport({
      elements,
      report: state.regressionGateReport,
      applyDesktopState,
    });
    renderDirectMeshRegressionSnapshot({
      elements,
      snapshot: state.directMeshRegressionSnapshot,
      copy: hubCopy(),
      regressionGateReport: state.regressionGateReport,
      applyDesktopState,
    });
  } catch {
    state.regressionGateReport = null;
    renderRegressionGateReport({
      elements,
      report: null,
      applyDesktopState,
    });
  }
}

function hubMessage(template, replacements = {}) {
  return Object.entries(replacements).reduce(
    (value, [key, replacement]) => value.replaceAll(`{${key}}`, String(replacement)),
    String(template ?? ""),
  );
}

function localizedHistoryFilterLabel(filter) {
  const copy = hubCopy();
  switch (filter) {
    case "all":
      return copy.bundles.all;
    case "failed":
      return copy.bundles.failed;
    case "inspect":
      return copy.bundles.inspect;
    case "normalize":
      return copy.bundles.normalize;
    case "diff":
      return copy.bundles.diff;
    default:
      return filter;
  }
}

function localizedWorkloadFilterLabel(filter) {
  const copy = hubCopy();
  switch (filter) {
    case "all":
      return copy.library.all;
    case "mechanical":
      return copy.library.mechanical;
    case "thermal":
      return copy.library.thermal;
    case "thermo_mechanical":
      return copy.library.thermo;
    default:
      return filter;
  }
}

function localizedWorkloadFamilyFilterLabel(filter) {
  const copy = hubCopy();
  switch (filter) {
    case "all":
      return copy.library.allFamilies;
    case "axial_and_springs":
      return copy.library.axial;
    case "beams_and_frames":
      return copy.library.beams;
    case "trusses":
      return copy.library.trusses;
    case "planes":
      return copy.library.planes;
    default:
      return filter;
  }
}

function localizedWorkflowCatalogLabel(key) {
  const copy = hubCopy();
  return copy.dynamic?.[key] || HUB_I18N.en.dynamic?.[key] || key;
}

const elements = {
  title: document.getElementById("section-title"),
  copy: document.getElementById("section-copy"),
  languageLabel: document.getElementById("shell-language-label"),
  languageSelect: document.getElementById("shell-language-select"),
  actionStatusLabel: document.getElementById("shell-action-status-label"),
  navProjects: document.getElementById("nav-projects"),
  navRuntimes: document.getElementById("nav-runtimes"),
  navDeploy: document.getElementById("nav-deploy"),
  navObserve: document.getElementById("nav-observe"),
  navTools: document.getElementById("nav-tools"),
  navItems: Array.from(document.querySelectorAll(".hub-nav__item")),
  panels: Array.from(document.querySelectorAll(".hub-panel")),
  heroOpenWorkbench: document.getElementById("hero-open-workbench"),
  heroStartLocal: document.getElementById("hero-start-local"),
  heroValidateEnv: document.getElementById("hero-validate-env"),
  signalIntakeLabel: document.getElementById("signal-intake-label"),
  signalIntakeTitle: document.getElementById("signal-intake-title"),
  signalIntakeCopy: document.getElementById("signal-intake-copy"),
  signalDomainsLabel: document.getElementById("signal-domains-label"),
  signalDomainsTitle: document.getElementById("signal-domains-title"),
  signalDomainsCopy: document.getElementById("signal-domains-copy"),
  signalFirstMoveLabel: document.getElementById("signal-firstmove-label"),
  signalFirstMoveTitle: document.getElementById("signal-firstmove-title"),
  signalFirstMoveCopy: document.getElementById("signal-firstmove-copy"),
  projectsPageButtons: Array.from(document.querySelectorAll("[data-projects-page]")),
  projectsTabStart: document.getElementById("projects-tab-start"),
  projectsTabLibrary: document.getElementById("projects-tab-library"),
  projectsTabBundles: document.getElementById("projects-tab-bundles"),
  projectsTabGuides: document.getElementById("projects-tab-guides"),
  projectsTargetButtons: Array.from(document.querySelectorAll("[data-projects-target]")),
  projectsPanes: Array.from(document.querySelectorAll("[data-projects-pane]")),
  homeStep1Label: document.getElementById("home-step1-label"),
  homeStep1Title: document.getElementById("home-step1-title"),
  homeStep1Copy: document.getElementById("home-step1-copy"),
  homeStep2Label: document.getElementById("home-step2-label"),
  homeStep2Title: document.getElementById("home-step2-title"),
  homeStep2Copy: document.getElementById("home-step2-copy"),
  homeStep3Label: document.getElementById("home-step3-label"),
  homeStep3Title: document.getElementById("home-step3-title"),
  homeStep3Copy: document.getElementById("home-step3-copy"),
  homePathLabel: document.getElementById("home-path-label"),
  homePathTitle: document.getElementById("home-path-title"),
  homePathCopy: document.getElementById("home-path-copy"),
  homeFlow1Title: document.getElementById("home-flow1-title"),
  homeFlow1Copy: document.getElementById("home-flow1-copy"),
  homeFlow2Title: document.getElementById("home-flow2-title"),
  homeFlow2Copy: document.getElementById("home-flow2-copy"),
  homeFlow3Title: document.getElementById("home-flow3-title"),
  homeFlow3Copy: document.getElementById("home-flow3-copy"),
  homeQuickLabel: document.getElementById("home-quick-label"),
  homeQuickTitle: document.getElementById("home-quick-title"),
  homeQuickCopy: document.getElementById("home-quick-copy"),
  homeClusterLibraryTitle: document.getElementById("home-cluster-library-title"),
  homeClusterLibraryCopy: document.getElementById("home-cluster-library-copy"),
  homeClusterBundlesTitle: document.getElementById("home-cluster-bundles-title"),
  homeClusterBundlesCopy: document.getElementById("home-cluster-bundles-copy"),
  homeClusterGuidesTitle: document.getElementById("home-cluster-guides-title"),
  homeClusterGuidesCopy: document.getElementById("home-cluster-guides-copy"),
  homeClusterInstallerTitle: document.getElementById("home-cluster-installer-title"),
  homeClusterInstallerCopy: document.getElementById("home-cluster-installer-copy"),
  homeClusterRuntimesTitle: document.getElementById("home-cluster-runtimes-title"),
  homeClusterRuntimesCopy: document.getElementById("home-cluster-runtimes-copy"),
  homeActionStart: document.getElementById("home-action-start"),
  homeActionSync: document.getElementById("home-action-sync"),
  homeActionOpen: document.getElementById("home-action-open"),
  libraryIntroLabel: document.getElementById("library-intro-label"),
  libraryIntroTitle: document.getElementById("library-intro-title"),
  libraryIntroCopy: document.getElementById("library-intro-copy"),
  libraryCatalogUrlLabel: document.getElementById("library-catalog-url-label"),
  libraryLabelNoteLabel: document.getElementById("library-label-note-label"),
  libraryActionRegister: document.getElementById("library-action-register"),
  libraryActionSyncLocal: document.getElementById("library-action-sync-local"),
  libraryActionSyncRemote: document.getElementById("library-action-sync-remote"),
  libraryActionExport: document.getElementById("library-action-export"),
  libraryActionImport: document.getElementById("library-action-import"),
  libraryActionClear: document.getElementById("library-action-clear"),
  libraryManagedWorkloadsLabel: document.getElementById("library-managed-workloads-label"),
  librarySearchLabel: document.getElementById("library-search-label"),
  workloadLibrarySearch: document.getElementById("workload-library-search"),
  librarySearchClear: document.getElementById("library-search-clear"),
  libraryFilterAll: document.getElementById("library-filter-all"),
  libraryFilterMechanical: document.getElementById("library-filter-mechanical"),
  libraryFilterThermal: document.getElementById("library-filter-thermal"),
  libraryFilterThermo: document.getElementById("library-filter-thermo"),
  libraryFamilyAll: document.getElementById("library-family-all"),
  libraryFamilyAxial: document.getElementById("library-family-axial"),
  libraryFamilyBeams: document.getElementById("library-family-beams"),
  libraryFamilyTrusses: document.getElementById("library-family-trusses"),
  libraryFamilyPlanes: document.getElementById("library-family-planes"),
  workflowCatalogLabel: document.getElementById("workflow-catalog-label"),
  workflowCatalogTitle: document.getElementById("workflow-catalog-title"),
  workflowCatalogCopy: document.getElementById("workflow-catalog-copy"),
  workflowCatalogSearchLabel: document.getElementById("workflow-catalog-search-label"),
  workflowCatalogSearch: document.getElementById("workflow-catalog-search"),
  workflowCatalogSearchClear: document.getElementById("workflow-catalog-search-clear"),
  workflowCatalogRefresh: document.getElementById("workflow-catalog-refresh"),
  workflowCatalogList: document.getElementById("workflow-catalog-list"),
  workflowCatalogOutput: document.getElementById("workflow-catalog-output"),
  bundlesIntroLabel: document.getElementById("bundles-intro-label"),
  bundlesIntroTitle: document.getElementById("bundles-intro-title"),
  bundlesIntroCopy: document.getElementById("bundles-intro-copy"),
  bundlesBundlePathLabel: document.getElementById("bundles-bundle-path-label"),
  bundlesComparePathLabel: document.getElementById("bundles-compare-path-label"),
  bundlesOutputPathLabel: document.getElementById("bundles-output-path-label"),
  bundlesActionInspect: document.getElementById("bundles-action-inspect"),
  bundlesActionValidate: document.getElementById("bundles-action-validate"),
  bundlesActionNormalize: document.getElementById("bundles-action-normalize"),
  bundlesActionUnpack: document.getElementById("bundles-action-unpack"),
  bundlesActionPack: document.getElementById("bundles-action-pack"),
  bundlesActionDiff: document.getElementById("bundles-action-diff"),
  bundlesActionOpenWorkbench: document.getElementById("bundles-action-open-workbench"),
  bundlesActionDesktopTools: document.getElementById("bundles-action-desktop-tools"),
  bundlesRecentBundlesLabel: document.getElementById("bundles-recent-bundles-label"),
  bundlesRecentCompareLabel: document.getElementById("bundles-recent-compare-label"),
  bundlesRecentOutputsLabel: document.getElementById("bundles-recent-outputs-label"),
  bundlesRecentActionsLabel: document.getElementById("bundles-recent-actions-label"),
  bundlesHistoryAll: document.getElementById("bundles-history-all"),
  bundlesHistoryFailed: document.getElementById("bundles-history-failed"),
  bundlesHistoryInspect: document.getElementById("bundles-history-inspect"),
  bundlesHistoryNormalize: document.getElementById("bundles-history-normalize"),
  bundlesHistoryDiff: document.getElementById("bundles-history-diff"),
  bundlesHistoryKeepFailed: document.getElementById("bundles-history-keep-failed"),
  bundlesHistoryImport: document.getElementById("bundles-history-import"),
  bundlesHistoryExport: document.getElementById("bundles-history-export"),
  bundlesHistoryClear: document.getElementById("bundles-history-clear"),
  bundlesFavoritesLabel: document.getElementById("bundles-favorites-label"),
  bundlesRecentLabel: document.getElementById("bundles-recent-label"),
  guidesPrimaryLabel: document.getElementById("guides-primary-label"),
  guidesPrimaryTitle: document.getElementById("guides-primary-title"),
  guidesPrimaryCopy: document.getElementById("guides-primary-copy"),
  guidesDocsTitle: document.getElementById("guides-docs-title"),
  guidesDocsCopy: document.getElementById("guides-docs-copy"),
  guidesCurrentTitle: document.getElementById("guides-current-title"),
  guidesCurrentCopy: document.getElementById("guides-current-copy"),
  guidesOverviewDocsLabel: document.getElementById("guides-overview-docs-label"),
  guidesOverviewDocsTitle: document.getElementById("guides-overview-docs-title"),
  guidesOverviewDocsCopy: document.getElementById("guides-overview-docs-copy"),
  guidesOverviewCurrentLabel: document.getElementById("guides-overview-current-label"),
  guidesOverviewCurrentTitle: document.getElementById("guides-overview-current-title"),
  guidesOverviewCurrentCopy: document.getElementById("guides-overview-current-copy"),
  guidesOverviewTroubleshootingLabel: document.getElementById("guides-overview-troubleshooting-label"),
  guidesOverviewTroubleshootingTitle: document.getElementById("guides-overview-troubleshooting-title"),
  guidesOverviewTroubleshootingCopy: document.getElementById("guides-overview-troubleshooting-copy"),
  guidesOperationsTitle: document.getElementById("guides-operations-title"),
  guidesOperationsCopy: document.getElementById("guides-operations-copy"),
  guidesTroubleshootingTitle: document.getElementById("guides-troubleshooting-title"),
  guidesTroubleshootingCopy: document.getElementById("guides-troubleshooting-copy"),
  guidesAccuracyLabel: document.getElementById("guides-accuracy-label"),
  guidesAccuracyTitle: document.getElementById("guides-accuracy-title"),
  guidesAccuracyCopy: document.getElementById("guides-accuracy-copy"),
  guidesAccuracyPlanTitle: document.getElementById("guides-accuracy-plan-title"),
  guidesAccuracyPlanCopy: document.getElementById("guides-accuracy-plan-copy"),
  guidesAccuracyBaselinesTitle: document.getElementById("guides-accuracy-baselines-title"),
  guidesAccuracyBaselinesCopy: document.getElementById("guides-accuracy-baselines-copy"),
  guidesDirectMeshTitle: document.getElementById("guides-direct-mesh-title"),
  guidesDirectMeshCopy: document.getElementById("guides-direct-mesh-copy"),
  guidesRegressionLabel: document.getElementById("guides-regression-label"),
  guidesRegressionTitle: document.getElementById("guides-regression-title"),
  guidesRegressionCopy: document.getElementById("guides-regression-copy"),
  guidesRegressionElapsedLabel: document.getElementById("guides-regression-elapsed-label"),
  guidesRegressionRssLabel: document.getElementById("guides-regression-rss-label"),
  guidesRegressionRepeatLabel: document.getElementById("guides-regression-repeat-label"),
  guidesRegressionNetworkLabel: document.getElementById("guides-regression-network-label"),
  guidesRegressionLatestLabel: document.getElementById("guides-regression-latest-label"),
  guidesRegressionStatusLabel: document.getElementById("guides-regression-status-label"),
  guidesRegressionBaselinePathLabel: document.getElementById("guides-regression-baseline-path-label"),
  guidesRegressionOutputPathLabel: document.getElementById("guides-regression-output-path-label"),
  guidesRegressionBaselineTitle: document.getElementById("guides-regression-baseline-title"),
  guidesRegressionBaselineCopy: document.getElementById("guides-regression-baseline-copy"),
  guidesRegressionOutputTitle: document.getElementById("guides-regression-output-title"),
  guidesRegressionOutputCopy: document.getElementById("guides-regression-output-copy"),
  guidesRegressionLaneTitle: document.getElementById("guides-regression-lane-title"),
  guidesRegressionLaneCopy: document.getElementById("guides-regression-lane-copy"),
  guidesRegressionElapsedValue: document.getElementById("guides-regression-elapsed-value"),
  guidesRegressionRssValue: document.getElementById("guides-regression-rss-value"),
  guidesRegressionRepeatValue: document.getElementById("guides-regression-repeat-value"),
  guidesRegressionNetworkValue: document.getElementById("guides-regression-network-value"),
  guidesRegressionLatestValue: document.getElementById("guides-regression-latest-value"),
  guidesRegressionStatusValue: document.getElementById("guides-regression-status-value"),
  guidesRegressionBaselinePath: document.getElementById("guides-regression-baseline-path"),
  guidesRegressionOutputPath: document.getElementById("guides-regression-output-path"),
  guidesRegressionNote: document.getElementById("guides-regression-note"),
  guidesGateTitle: document.getElementById("guides-gate-title"),
  guidesGateCopy: document.getElementById("guides-gate-copy"),
  guidesGateStatusValue: document.getElementById("guides-gate-status-value"),
  guidesGateWarningCount: document.getElementById("guides-gate-warning-count"),
  guidesGateFailingCount: document.getElementById("guides-gate-failing-count"),
  guidesGateLaneCount: document.getElementById("guides-gate-lane-count"),
  guidesGateCatalogPath: document.getElementById("guides-gate-catalog-path"),
  guidesGateNote: document.getElementById("guides-gate-note"),
  guidesGateReasons: document.getElementById("guides-gate-reasons"),
  guidesLocalizationLabel: document.getElementById("guides-localization-label"),
  guidesLocalizationTitle: document.getElementById("guides-localization-title"),
  guidesLocalizationCopy: document.getElementById("guides-localization-copy"),
  guidesLocalizationActiveLanguageLabel: document.getElementById("guides-localization-active-language-label"),
  guidesLocalizationActiveLanguageValue: document.getElementById("guides-localization-active-language-value"),
  guidesLocalizationInstalledLanguagesLabel: document.getElementById("guides-localization-installed-languages-label"),
  guidesLocalizationInstalledLanguagesValue: document.getElementById("guides-localization-installed-languages-value"),
  guidesLocalizationDefaultLayerLabel: document.getElementById("guides-localization-default-layer-label"),
  guidesLocalizationDefaultLayerValue: document.getElementById("guides-localization-default-layer-value"),
  guidesLocalizationImportModeLabel: document.getElementById("guides-localization-import-mode-label"),
  guidesLocalizationImportModeValue: document.getElementById("guides-localization-import-mode-value"),
  guidesLocalizationLatestAssetLabel: document.getElementById("guides-localization-latest-asset-label"),
  guidesLocalizationLatestAssetValue: document.getElementById("guides-localization-latest-asset-value"),
  guidesLocalizationStorageKeyLabel: document.getElementById("guides-localization-storage-key-label"),
  guidesLocalizationStorageKeyValue: document.getElementById("guides-localization-storage-key-value"),
  guidesLocalizationImport: document.getElementById("guides-localization-import"),
  guidesLocalizationExport: document.getElementById("guides-localization-export"),
  guidesLocalizationClear: document.getElementById("guides-localization-clear"),
  guidesLocalizationImportInput: document.getElementById("guides-localization-import-input"),
  guidesLocalizationOutput: document.getElementById("guides-localization-output"),
  guidesLocalizationHint: document.getElementById("guides-localization-hint"),
  runtimeLocalLabel: document.getElementById("runtime-local-label"),
  runtimeLocalTitle: document.getElementById("runtime-local-title"),
  runtimeLocalCopy: document.getElementById("runtime-local-copy"),
  runtimeLocalStatusLabel: document.getElementById("runtime-local-status-label"),
  runtimeLocalFrontendLabel: document.getElementById("runtime-local-frontend-label"),
  runtimeLocalControlLabel: document.getElementById("runtime-local-control-label"),
  runtimeLocalAgentsLabel: document.getElementById("runtime-local-agents-label"),
  runtimeHotLabel: document.getElementById("runtime-hot-label"),
  runtimeHotTitle: document.getElementById("runtime-hot-title"),
  runtimeHotCopy: document.getElementById("runtime-hot-copy"),
  runtimeHotStatusLabel: document.getElementById("runtime-hot-status-label"),
  runtimeHotModeLabel: document.getElementById("runtime-hot-mode-label"),
  runtimeHotActionLocal: document.getElementById("runtime-hot-action-local"),
  runtimeHotActionCloud: document.getElementById("runtime-hot-action-cloud"),
  runtimeHotActionDistributed: document.getElementById("runtime-hot-action-distributed"),
  runtimeHotActionRefresh: document.getElementById("runtime-hot-action-refresh"),
  runtimeHotActionStop: document.getElementById("runtime-hot-action-stop"),
  runtimeHotLogsLabel: document.getElementById("runtime-hot-logs-label"),
  runtimeHotAutoLabel: document.getElementById("runtime-hot-auto-label"),
  runtimeHotIntervalLabel: document.getElementById("runtime-hot-interval-label"),
  runtimeHotRefreshLog: document.getElementById("runtime-hot-refresh-log"),
  runtimeHotCopyTail: document.getElementById("runtime-hot-copy-tail"),
  runtimeHotClearView: document.getElementById("runtime-hot-clear-view"),
  runtimeHotNote: document.getElementById("runtime-hot-note"),
  runtimeTargetsLabel: document.getElementById("runtime-targets-label"),
  runtimeTargetsTitle: document.getElementById("runtime-targets-title"),
  runtimeTargetsCopy: document.getElementById("runtime-targets-copy"),
  deployModesLabel: document.getElementById("deploy-modes-label"),
  deployModesTitle: document.getElementById("deploy-modes-title"),
  deployModesCopy: document.getElementById("deploy-modes-copy"),
  deployActionLocal: document.getElementById("deploy-action-local"),
  deployActionCloud: document.getElementById("deploy-action-cloud"),
  deployActionDistributed: document.getElementById("deploy-action-distributed"),
  deployActionRestart: document.getElementById("deploy-action-restart"),
  deployBootstrapLabel: document.getElementById("deploy-bootstrap-label"),
  deployBootstrapTitle: document.getElementById("deploy-bootstrap-title"),
  deployBootstrapCopy: document.getElementById("deploy-bootstrap-copy"),
  deployBootstrapValidate: document.getElementById("deploy-bootstrap-validate"),
  deployBootstrapStage: document.getElementById("deploy-bootstrap-stage"),
  deployBootstrapDoctor: document.getElementById("deploy-bootstrap-doctor"),
  deployReleaseLabel: document.getElementById("deploy-release-label"),
  deployReleaseTitle: document.getElementById("deploy-release-title"),
  deployReleaseCopy: document.getElementById("deploy-release-copy"),
  observeHealthLabel: document.getElementById("observe-health-label"),
  observeHealthTitle: document.getElementById("observe-health-title"),
  observeHealthCopy: document.getElementById("observe-health-copy"),
  observeHealthWatchdogLabel: document.getElementById("observe-health-watchdog-label"),
  observeHealthSecurityLabel: document.getElementById("observe-health-security-label"),
  observeHealthFailuresLabel: document.getElementById("observe-health-failures-label"),
  observeRuntimeTitle: document.getElementById("observe-runtime-title"),
  observeRuntimeStatusLabel: document.getElementById("observe-runtime-status-label"),
  observeRuntimeHotLabel: document.getElementById("observe-runtime-hot-label"),
  observeRuntimeModeLabel: document.getElementById("observe-runtime-mode-label"),
  observeRuntimeSourceLabel: document.getElementById("observe-runtime-source-label"),
  observeRuntimeOpen: document.getElementById("observe-runtime-open"),
  observeRuntimeRefresh: document.getElementById("observe-runtime-refresh"),
  observeRuntimeCopy: document.getElementById("observe-runtime-copy"),
  observeStackTitle: document.getElementById("observe-stack-title"),
  observeStackLogsLabel: document.getElementById("observe-stack-logs-label"),
  observeStackAutoLabel: document.getElementById("observe-stack-auto-label"),
  observeStackRefresh: document.getElementById("observe-stack-refresh"),
  observeStackCopy: document.getElementById("observe-stack-copy"),
  observeStackNote: document.getElementById("observe-stack-note"),
  toolsPackagesLabel: document.getElementById("tools-packages-label"),
  toolsPackagesTitle: document.getElementById("tools-packages-title"),
  toolsPackagesCopy: document.getElementById("tools-packages-copy"),
  toolsPackagesPlatformLabel: document.getElementById("tools-packages-platform-label"),
  toolsPackagesBenchmark: document.getElementById("tools-packages-benchmark"),
  toolsPackagesValidate: document.getElementById("tools-packages-validate"),
  toolsPackagesExport: document.getElementById("tools-packages-export"),
  toolsPackagesStatus: document.getElementById("tools-packages-status"),
  toolsPackagesStage: document.getElementById("tools-packages-stage"),
  toolsPackagesBuild: document.getElementById("tools-packages-build"),
  toolsPackagesVerify: document.getElementById("tools-packages-verify"),
  toolsPackagesStop: document.getElementById("tools-packages-stop"),
  toolsStatusLabel: document.getElementById("tools-status-label"),
  toolsStatusTitle: document.getElementById("tools-status-title"),
  toolsStatusCopy: document.getElementById("tools-status-copy"),
  toolsOutputLabel: document.getElementById("tools-output-label"),
  toolsOutputTitle: document.getElementById("tools-output-title"),
  toolsOutputCopy: document.getElementById("tools-output-copy"),
  panelPageButtons: Array.from(document.querySelectorAll("[data-panel-page-group][data-panel-page]")),
  panelPanes: Array.from(document.querySelectorAll("[data-panel-pane-group][data-panel-pane]")),
  assistantFab: document.getElementById("hub-assistant-fab"),
  assistantClose: document.getElementById("hub-assistant-close"),
  assistantPanel: document.getElementById("hub-assistant-panel"),
  releasePlatform: document.getElementById("release-platform"),
  projectBundlePath: document.getElementById("project-bundle-path"),
  projectBundleComparePath: document.getElementById("project-bundle-compare-path"),
  projectBundleOutPath: document.getElementById("project-bundle-out-path"),
  projectBundleOutput: document.getElementById("project-bundle-output"),
  workloadCatalogUrl: document.getElementById("workload-catalog-url"),
  workloadLabel: document.getElementById("workload-label"),
  workloadImportInput: document.getElementById("workload-import-input"),
  workloadLibraryList: document.getElementById("workload-library-list"),
  workloadLibraryOutput: document.getElementById("workload-library-output"),
  workloadFilterButtons: Array.from(document.querySelectorAll("[data-workload-filter]")),
  workloadFamilyFilterButtons: Array.from(document.querySelectorAll("[data-workload-family-filter]")),
  historyImportInput: document.getElementById("history-import-input"),
  recentBundleList: document.getElementById("recent-bundle-list"),
  recentCompareList: document.getElementById("recent-compare-list"),
  recentOutputList: document.getElementById("recent-output-list"),
  favoriteActionList: document.getElementById("favorite-action-list"),
  recentActionList: document.getElementById("recent-action-list"),
  operationOutput: document.getElementById("hub-operation-output"),
  runtimeStatusPlane: document.getElementById("runtime-status-plane"),
  runtimeStatusOutput: document.getElementById("runtime-status-output"),
  localRuntimeStatus: document.getElementById("local-runtime-status"),
  observeRuntimeStatusOutput: document.getElementById("observe-runtime-status-output"),
  observeRuntimeStatus: document.getElementById("observe-runtime-status"),
  hotRuntimeStatusOutput: document.getElementById("hot-runtime-status-output"),
  hotRuntimeStatus: document.getElementById("hot-runtime-status"),
  hotRuntimeMode: document.getElementById("hot-runtime-mode"),
  hotRuntimeLogService: document.getElementById("hot-runtime-log-service"),
  hotRuntimeLogAuto: document.getElementById("hot-runtime-log-auto"),
  hotRuntimeLogInterval: document.getElementById("hot-runtime-log-interval"),
  hotRuntimeLogFollowState: document.getElementById("hot-runtime-log-follow-state"),
  hotRuntimeLogOutput: document.getElementById("hot-runtime-log-output"),
  observeHotStatus: document.getElementById("observe-hot-status"),
  observeHotMode: document.getElementById("observe-hot-mode"),
  observeHotFollowState: document.getElementById("observe-hot-follow-state"),
  observeHotLogService: document.getElementById("observe-hot-log-service"),
  observeHotLogOutput: document.getElementById("observe-hot-log-output"),
  observeRuntimeLogService: document.getElementById("observe-runtime-log-service"),
  observeRuntimeLogAuto: document.getElementById("observe-runtime-log-auto"),
  observeRuntimeLogFollowState: document.getElementById("observe-runtime-log-follow-state"),
  observeRuntimeLogOutput: document.getElementById("observe-runtime-log-output"),
  workbenchUrl: document.getElementById("local-workbench-url"),
  orchestratorUrl: document.getElementById("local-orchestrator-url"),
  currentRuntimeMode: document.getElementById("current-runtime-mode"),
  currentProfile: document.getElementById("current-profile"),
  actionState: document.getElementById("hub-action-state"),
  desktopStatusOutput: document.getElementById("hub-desktop-status-output"),
  actionButtons: Array.from(document.querySelectorAll("[data-action]")),
  sectionJumpButtons: Array.from(document.querySelectorAll("[data-target-section]")),
  historyFilterButtons: Array.from(document.querySelectorAll("[data-history-filter]")),
  historyManageButtons: Array.from(document.querySelectorAll("[data-history-manage]")),
  assistantModeButtons: Array.from(document.querySelectorAll("[data-assistant-mode]")),
  assistantIntroLabel: document.getElementById("assistant-intro-label"),
  assistantIntroTitle: document.getElementById("assistant-intro-title"),
  assistantIntroCopy: document.getElementById("assistant-intro-copy"),
  assistantEngineLabel: document.getElementById("assistant-engine-label"),
  assistantContextSectionLabel: document.getElementById("assistant-context-section-label"),
  assistantContextRuntimeLabel: document.getElementById("assistant-context-runtime-label"),
  assistantContextBundleLabel: document.getElementById("assistant-context-bundle-label"),
  assistantLocalActionsLabel: document.getElementById("assistant-local-actions-label"),
  assistantLocalActionStart: document.getElementById("assistant-local-action-start"),
  assistantLocalActionLibrary: document.getElementById("assistant-local-action-library"),
  assistantLocalActionBundles: document.getElementById("assistant-local-action-bundles"),
  assistantLocalActionGuides: document.getElementById("assistant-local-action-guides"),
  assistantLocalAskLabel: document.getElementById("assistant-local-ask-label"),
  assistantLocalPromptLabel: document.getElementById("assistant-local-prompt-label"),
  assistantEngineState: document.getElementById("assistant-engine-state"),
  assistantContextSection: document.getElementById("assistant-context-section"),
  assistantContextRuntime: document.getElementById("assistant-context-runtime"),
  assistantContextBundle: document.getElementById("assistant-context-bundle"),
  assistantLocalPanel: document.getElementById("assistant-local-panel"),
  assistantLocalCards: document.getElementById("assistant-local-cards"),
  assistantLocalPrompt: document.getElementById("assistant-local-prompt"),
  assistantLocalAsk: document.getElementById("assistant-local-ask"),
  assistantLocalOutput: document.getElementById("assistant-local-output"),
  assistantDocsLabel: document.getElementById("assistant-docs-label"),
  assistantDocsIndexTitle: document.getElementById("assistant-docs-index-title"),
  assistantDocsIndexCopy: document.getElementById("assistant-docs-index-copy"),
  assistantDocsCurrentTitle: document.getElementById("assistant-docs-current-title"),
  assistantDocsCurrentCopy: document.getElementById("assistant-docs-current-copy"),
  assistantDocsOperationsTitle: document.getElementById("assistant-docs-operations-title"),
  assistantDocsOperationsCopy: document.getElementById("assistant-docs-operations-copy"),
  assistantDocsTroubleshootingTitle: document.getElementById("assistant-docs-troubleshooting-title"),
  assistantDocsTroubleshootingCopy: document.getElementById("assistant-docs-troubleshooting-copy"),
  assistantSuggestedLabel: document.getElementById("assistant-suggested-label"),
  assistantLlmPanel: document.getElementById("assistant-llm-panel"),
  assistantLlmIntroCopy: document.getElementById("assistant-llm-intro-copy"),
  assistantBaseUrlLabel: document.getElementById("assistant-base-url-label"),
  assistantApiKeyLabel: document.getElementById("assistant-api-key-label"),
  assistantPresetLabel: document.getElementById("assistant-preset-label"),
  assistantModelLabel: document.getElementById("assistant-model-label"),
  assistantRequestLabel: document.getElementById("assistant-request-label"),
  assistantBaseUrl: document.getElementById("assistant-base-url"),
  assistantApiKey: document.getElementById("assistant-api-key"),
  assistantModelPreset: document.getElementById("assistant-model-preset"),
  assistantModelName: document.getElementById("assistant-model-name"),
  assistantPrompt: document.getElementById("assistant-prompt"),
  assistantEndpointPolicy: document.getElementById("assistant-endpoint-policy"),
  assistantRequestPlan: document.getElementById("assistant-request-plan"),
  assistantApprovePlan: document.getElementById("assistant-approve-plan"),
  assistantApproveLabel: document.getElementById("assistant-approve-label"),
  assistantExecutePlan: document.getElementById("assistant-execute-plan"),
  assistantPlanActions: document.getElementById("assistant-plan-actions"),
  assistantOutput: document.getElementById("assistant-output"),
  assistantAuditLabel: document.getElementById("assistant-audit-label"),
  assistantAuditList: document.getElementById("assistant-audit-list"),
  densityToggleButtons: Array.from(document.querySelectorAll("[data-density-toggle]")),
  densityPanels: Array.from(document.querySelectorAll("[data-density-panel]")),
};

function renderDesktopLanguagePreference() {
  const copy = hubCopy();
  document.documentElement.lang = state.language;
  if (elements.languageLabel) {
    elements.languageLabel.textContent = copy.shell.language;
  }
  if (elements.languageSelect) {
    elements.languageSelect.value = state.language;
  }
  if (elements.actionStatusLabel) {
    elements.actionStatusLabel.textContent = copy.shell.actionStatus;
  }
  if (elements.navProjects) {
    elements.navProjects.textContent = copy.nav.projects;
  }
  if (elements.navRuntimes) {
    elements.navRuntimes.textContent = copy.nav.runtimes;
  }
  if (elements.navDeploy) {
    elements.navDeploy.textContent = copy.nav.deploy;
  }
  if (elements.navObserve) {
    elements.navObserve.textContent = copy.nav.observe;
  }
  if (elements.navTools) {
    elements.navTools.textContent = copy.nav.tools;
  }
  setText("brand-hub-focus", copy.shell.focus);
  if (elements.heroOpenWorkbench) {
    elements.heroOpenWorkbench.textContent = copy.shell.openWorkbench;
  }
  if (elements.heroStartLocal) {
    elements.heroStartLocal.textContent = copy.shell.startLocal;
  }
  if (elements.heroValidateEnv) {
    elements.heroValidateEnv.textContent = copy.shell.validateEnv;
  }
  setText(elements.signalIntakeLabel, copy.signals.intakeLabel);
  setText(elements.signalIntakeTitle, copy.signals.intakeTitle);
  setText(elements.signalIntakeCopy, copy.signals.intakeCopy);
  setText(elements.signalDomainsLabel, copy.signals.domainsLabel);
  setText(elements.signalDomainsTitle, copy.signals.domainsTitle);
  setText(elements.signalDomainsCopy, copy.signals.domainsCopy);
  setText(elements.signalFirstMoveLabel, copy.signals.firstMoveLabel);
  setText(elements.signalFirstMoveTitle, copy.signals.firstMoveTitle);
  setText(elements.signalFirstMoveCopy, copy.signals.firstMoveCopy);
  if (!state.isBusy && elements.actionState) {
    elements.actionState.textContent = copy.shell.idle;
  }
  renderHubHomeCopy({
    elements,
    copy,
    setText,
  });
  renderHubLibraryCopy({
    elements,
    copy,
    isBusy: state.isBusy,
    workflowCatalogBusy: state.workflowCatalogBusy,
    setText,
  });
  renderHubBundlesCopy({
    elements,
    copy,
    isBusy: state.isBusy,
    setText,
  });
  renderGuidesPanelCopy({
    elements,
    copy,
    activeLanguage: state.language,
    setText,
  });
  renderDirectMeshRegressionSnapshot({
    elements,
    snapshot: state.directMeshRegressionSnapshot,
    copy,
    regressionGateReport: state.regressionGateReport,
    applyDesktopState,
  });
  renderAssistantShellCopy({
    elements,
    copy,
    isBusy: state.isBusy,
    setText,
  });
  renderPanelLanguage(copy);

  const activeSectionCopy = copy.sections[state.activeSection];
  if (activeSectionCopy) {
    elements.title.textContent = activeSectionCopy.title;
    elements.copy.textContent = activeSectionCopy.copy;
  }
}

function rerenderLocalizedHubShell() {
  renderDesktopLanguagePreference();
  renderHubRecents();
  renderWorkflowCatalog();
  renderHubAssistantAudit();
  renderAssistantContext();
  renderHubAssistantLocalCards();
  renderAssistantPanel();
}

function renderPanelLanguage(copy) {
  renderHubPanelCopy({
    elements,
    copy,
    setText,
    renderToolsPlatformLabel,
  });
}

function loadHubRecents() {
  try {
    const raw = window.localStorage.getItem(HUB_RECENTS_KEY);
    if (!raw) {
      return { bundles: [], compares: [], outputs: [] };
    }

    const parsed = JSON.parse(raw);
    return {
      bundles: Array.isArray(parsed?.bundles) ? parsed.bundles : [],
      compares: Array.isArray(parsed?.compares) ? parsed.compares : [],
      outputs: Array.isArray(parsed?.outputs) ? parsed.outputs : [],
      actions: Array.isArray(parsed?.actions) ? parsed.actions : [],
    };
  } catch {
    return { bundles: [], compares: [], outputs: [], actions: [] };
  }
}

function persistHubRecents(recents) {
  window.localStorage.setItem(HUB_RECENTS_KEY, JSON.stringify(recents));
}

function loadHubWorkloadLibrary() {
  return loadStoredHubWorkloadLibrary(HUB_WORKLOAD_LIBRARY_KEY);
}

function persistHubWorkloadLibrary(entries) {
  persistStoredHubWorkloadLibrary(HUB_WORKLOAD_LIBRARY_KEY, entries, HUB_WORKLOAD_LIBRARY_LIMIT);
}

function appendTextElement(parent, tagName, text, className) {
  const element = document.createElement(tagName);
  if (className) {
    element.className = className;
  }
  element.textContent = text;
  parent.appendChild(element);
  return element;
}

function appendAssistantCardHeader(parent, title, badgeText, badgeClassName) {
  const header = document.createElement("div");
  header.className = "desktop-shell-section-header";
  appendTextElement(header, "strong", title);
  appendTextElement(header, "span", badgeText, badgeClassName);
  parent.appendChild(header);
  return header;
}

function workloadIdentity(entry) {
  return buildWorkloadIdentity(entry);
}

function normalizeHubWorkloadEntry(entry) {
  return normalizeStoredHubWorkloadEntry(entry);
}

function mergeHubWorkloadLibrary(existingEntries, incomingEntries) {
  return mergeStoredHubWorkloadLibrary(
    existingEntries,
    incomingEntries,
    HUB_WORKLOAD_LIBRARY_LIMIT,
  );
}

function loadHubAssistantSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_ASSISTANT_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return {
      mode: parsed?.mode === "llm" ? "llm" : "local",
      baseUrl: String(parsed?.baseUrl || ""),
      modelPreset: HUB_ASSISTANT_MODEL_PRESETS.includes(String(parsed?.modelPreset || "")) ? parsed.modelPreset : "gpt-5",
      model: String(parsed?.model || "gpt-5"),
    };
  } catch {
    return { mode: "local", baseUrl: "", modelPreset: "gpt-5", model: "gpt-5" };
  }
}

function persistHubAssistantSettings(settings) {
  window.localStorage.setItem(HUB_ASSISTANT_SETTINGS_KEY, JSON.stringify(settings));
}

function loadHubAssistantTrustedHosts() {
  return loadHubTrustedHosts(HUB_ASSISTANT_TRUSTED_HOSTS_KEY);
}

function loadHubTrustedHosts(storageKey) {
  try {
    const raw = window.localStorage.getItem(storageKey);
    const parsed = raw ? JSON.parse(raw) : [];
    return new Set(Array.isArray(parsed) ? parsed.filter((item) => typeof item === "string") : []);
  } catch {
    return new Set();
  }
}

function persistHubAssistantTrustedHosts(hosts) {
  persistHubTrustedHosts(HUB_ASSISTANT_TRUSTED_HOSTS_KEY, hosts);
}

function persistHubTrustedHosts(storageKey, hosts) {
  window.localStorage.setItem(storageKey, JSON.stringify(Array.from(hosts)));
}

function clearLegacyHubAssistantSecrets() {
  try {
    window.sessionStorage.removeItem(HUB_ASSISTANT_LEGACY_SECRETS_KEY);
  } catch {
    // Ignore cleanup failures for best-effort legacy secret removal.
  }
}

function loadHubAssistantAudit() {
  try {
    const raw = window.sessionStorage.getItem(HUB_ASSISTANT_AUDIT_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persistHubAssistantAudit(entries) {
  window.sessionStorage.setItem(HUB_ASSISTANT_AUDIT_KEY, JSON.stringify(entries.slice(0, HUB_ASSISTANT_AUDIT_LIMIT)));
}

function loadHubHotLogSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_HOT_LOG_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const interval = String(parsed?.interval || "4000");
    return {
      service: String(parsed?.service || "hot-stack"),
      autoRefresh: parsed?.autoRefresh !== false,
      interval: ["2000", "4000", "8000"].includes(interval) ? interval : "4000",
    };
  } catch {
    return { service: "hot-stack", autoRefresh: true, interval: "4000" };
  }
}

function persistHubHotLogSettings(settings) {
  window.localStorage.setItem(HUB_HOT_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

function loadHubRuntimeLogSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_RUNTIME_LOG_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const service = String(parsed?.service || "frontend");
    return {
      service: ["frontend", "orchestrator", "agent-5001", "agent-5002"].includes(service) ? service : "frontend",
      autoRefresh: parsed?.autoRefresh !== false,
    };
  } catch {
    return { service: "frontend", autoRefresh: true };
  }
}

function persistHubRuntimeLogSettings(settings) {
  window.localStorage.setItem(HUB_RUNTIME_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

function loadHubDensitySettings() {
  try {
    const raw = window.localStorage.getItem(HUB_DENSITY_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return Object.fromEntries(
      Object.entries(HUB_DENSITY_DEFAULTS).map(([key, defaultExpanded]) => [
        key,
        typeof parsed?.[key] === "boolean" ? parsed[key] : defaultExpanded,
      ]),
    );
  } catch {
    return { ...HUB_DENSITY_DEFAULTS };
  }
}

function persistHubDensitySettings() {
  window.localStorage.setItem(HUB_DENSITY_SETTINGS_KEY, JSON.stringify(state.density));
}

function assistantRiskLevel(action) {
  return HUB_ASSISTANT_ACTION_RISK[action] || "low";
}

function assistantRiskStateClass(risk) {
  switch (risk) {
    case "high":
      return "desktop-shell-state desktop-shell-state--danger";
    case "sensitive":
      return "desktop-shell-state desktop-shell-state--warning";
    default:
      return "desktop-shell-state desktop-shell-state--healthy";
  }
}

function assistantStatusStateClass(status) {
  switch (status) {
    case "failed":
    case "cancelled":
      return "desktop-shell-state desktop-shell-state--danger";
    case "prompted":
    case "confirmed":
      return "desktop-shell-state desktop-shell-state--warning";
    case "completed":
      return "desktop-shell-state desktop-shell-state--healthy";
    default:
      return "desktop-shell-state desktop-shell-state--idle";
  }
}

function assistantDeliveryStateClass(delivery) {
  switch (delivery) {
    case "synced":
      return "desktop-shell-state desktop-shell-state--healthy";
    case "sync_failed":
      return "desktop-shell-state desktop-shell-state--danger";
    default:
      return "desktop-shell-state desktop-shell-state--idle";
  }
}

function formatAssistantAuditTime(value) {
  const timestamp = new Date(String(value || "").trim());
  if (Number.isNaN(timestamp.getTime())) {
    return String(value || "").trim();
  }

  return timestamp.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function renderHubAssistantAudit(entries = loadHubAssistantAudit()) {
  if (!elements.assistantAuditList) {
    return;
  }

  elements.assistantAuditList.innerHTML = "";
  if (!entries.length) {
    renderEmptyHistoryState(elements.assistantAuditList, "No assistant actions recorded in this session.");
    return;
  }

  entries.forEach((entry) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    const header = document.createElement("div");
    header.className = "desktop-shell-section-header";
    appendTextElement(header, "strong", entry.action);
    const badges = document.createElement("div");
    badges.className = "desktop-shell-action-row";
    appendTextElement(badges, "span", entry.risk, assistantRiskStateClass(entry.risk));
    appendTextElement(badges, "span", entry.status, assistantStatusStateClass(entry.status));
    appendTextElement(badges, "span", entry.delivery || "local", assistantDeliveryStateClass(entry.delivery || "local"));
    header.appendChild(badges);
    article.appendChild(header);
    appendTextElement(
      article,
      "p",
      `${formatAssistantAuditTime(entry.createdAt)} · ${entry.source}${entry.note ? ` · ${entry.note}` : ""}`,
      "desktop-shell-note",
    );
    elements.assistantAuditList.appendChild(article);
  });
}

function rememberHubAssistantAudit(entry) {
  const normalized = {
    auditId: String(entry?.auditId || `hub-assistant-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    action: String(entry?.action || "").trim(),
    risk: String(entry?.risk || "low").trim(),
    status: String(entry?.status || "idle").trim(),
    source: String(entry?.source || "assistant").trim(),
    note: String(entry?.note || "").trim(),
    createdAt: new Date().toISOString(),
    delivery: String(entry?.delivery || "local").trim(),
  };

  if (!normalized.action) {
    return loadHubAssistantAudit();
  }

  const next = [normalized, ...loadHubAssistantAudit()].slice(0, HUB_ASSISTANT_AUDIT_LIMIT);
  persistHubAssistantAudit(next);
  renderHubAssistantAudit(next);
  if (entry?.sync !== false) {
    void mirrorHubAssistantAuditToSecurityEvents(normalized);
  }
  return next;
}

function currentOrchestratorBaseUrl() {
  const text = String(elements.orchestratorUrl?.textContent || "").trim();
  return text || "http://127.0.0.1:4000";
}

function currentLocalWorkloadCatalogUrl() {
  return `${currentOrchestratorBaseUrl().replace(/\/+$/u, "")}/api/v1/workloads/catalog`;
}

function currentWorkflowCatalogUrl() {
  return `${currentOrchestratorBaseUrl().replace(/\/+$/u, "")}/api/v1/workflows/catalog`;
}

function ensureDefaultWorkloadCatalogUrl(force = false) {
  if (!elements.workloadCatalogUrl) {
    return "";
  }

  if (!force && String(elements.workloadCatalogUrl.value || "").trim()) {
    return String(elements.workloadCatalogUrl.value || "").trim();
  }

  const next = currentLocalWorkloadCatalogUrl();
  elements.workloadCatalogUrl.value = next;
  return next;
}

function currentAssistantAuditContext() {
  return {
    section: state.activeSection,
    runtime: String(elements.currentRuntimeMode?.textContent || "").trim(),
    profile: String(elements.currentProfile?.textContent || "").trim(),
    bundle_path: String(elements.projectBundlePath?.value || "").trim(),
    compare_path: String(elements.projectBundleComparePath?.value || "").trim(),
    output_path: String(elements.projectBundleOutPath?.value || "").trim(),
  };
}

function updateHubAssistantAuditDelivery(auditId, delivery, noteSuffix = "") {
  const entries = loadHubAssistantAudit();
  const next = entries.map((entry) => {
    if (entry.auditId !== auditId) {
      return entry;
    }
    return {
      ...entry,
      delivery,
      note: noteSuffix ? `${entry.note}${entry.note ? " · " : ""}${noteSuffix}` : entry.note,
    };
  });
  persistHubAssistantAudit(next);
  renderHubAssistantAudit(next);
}

async function mirrorHubAssistantAuditToSecurityEvents(entry) {
  const baseUrl = currentOrchestratorBaseUrl().replace(/\/+$/, "");
  try {
    const response = await fetch(`${baseUrl}/api/v1/security-events`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        event_id: entry.auditId,
        event_type: "hub.assistant.action",
        source: "hub-assistant",
        action: entry.action,
        risk: entry.risk,
        status: entry.status,
        note: entry.note || null,
        context: {
          ...currentAssistantAuditContext(),
          assistant_source: entry.source,
          delivery: "hub-session",
        },
        occurred_at: entry.createdAt,
      }),
    });

    if (!response.ok) {
      throw new Error(`control-plane sync failed (${response.status})`);
    }

    updateHubAssistantAuditDelivery(entry.auditId, "synced");
  } catch (error) {
    updateHubAssistantAuditDelivery(
      entry.auditId,
      "sync_failed",
      error instanceof Error ? error.message : String(error),
    );
  }
}

function saveHubRecents(recents) {
  persistHubRecents(recents);
  renderHubRecents(recents);
}

function setWorkloadLibraryOutput(value) {
  if (elements.workloadLibraryOutput) {
    elements.workloadLibraryOutput.textContent = value;
  }
}

function rawErrorMessage(error) {
  return error instanceof Error ? error.message : String(error || "");
}

function formatHubOperatorError(error, options = {}) {
  const raw = rawErrorMessage(error).trim();
  const actionLabel = String(options?.actionLabel || "This action").trim();
  const service = String(options?.service || "").trim();
  const context = String(options?.context || "").trim();

  if (/request timed out:/i.test(raw)) {
    return `${actionLabel} timed out. Check runtime health and agent availability, then try again.`;
  }

  if (context === "log-read") {
    return `Couldn't read the ${service || "selected"} log right now. Check whether the runtime is running, then refresh the log again.`;
  }

  if (context === "desktop-status") {
    return "Couldn't refresh desktop packaging status right now. Check the local runtime tools and try again.";
  }

  if (/operation not permitted|permission denied|access denied|denied|eperm/i.test(raw)) {
    return `${actionLabel} needs additional local access. Check desktop permissions and try again.`;
  }

  if (/invalid analysis_domains|invalid analysis_families|invalid thermal_intents|missing label/i.test(raw)) {
    return `The workload catalog format is not valid for ${actionLabel.toLowerCase()}. Check the catalog entry and try again.`;
  }

  if (!raw) {
    return `${actionLabel} didn't complete. Try again after checking runtime state and inputs.`;
  }

  return `${actionLabel} didn't complete: ${raw}`;
}

function inferDownloadFilename(url, fallback = "kyuubiki-workload.kyuubiki") {
  return deriveDownloadFilename(url, fallback);
}

function downloadHubBlob(filename, blob) {
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}

function workloadSourceBadge(entry) {
  return resolveWorkloadSourceBadge(entry);
}

function workloadProvenanceLabel(entry) {
  return resolveWorkloadProvenanceLabel(entry);
}

function workloadProvenanceHost(value) {
  return resolveWorkloadProvenanceHost(value);
}

function workloadDomainLabel(domain) {
  return resolveWorkloadDomainLabel(domain);
}

function workloadFamilyLabel(family) {
  return resolveWorkloadFamilyLabel(family);
}

function workloadRuntimeContext() {
  return {
    downloadHubBlob,
    downloadHubJson,
    elements,
    ensureDefaultWorkloadCatalogUrl,
    ensureRemoteHostTrust,
    inferDownloadFilename,
    invokeTauri,
    loadHubWorkloadLibrary,
    mergeHubWorkloadLibrary,
    normalizeHubWorkloadEntry,
    persistHubWorkloadLibrary,
    projectSummaryFromInspectPayload,
    renderAssistantContext,
    renderHubAssistantLocalCards,
    renderHubWorkloadLibrary,
    runAction,
    saveHubWorkloadLibrary,
    setWorkloadLibraryOutput,
    state,
    validateHubCatalogUrl,
    validateRemoteWorkloadCatalogPayload,
    normalizeRemoteWorkloadCatalogPayload,
    workloadIdentity,
  };
}

async function downloadRemoteWorkloadBundle(entry) {
  return downloadRemoteWorkloadBundleModule(workloadRuntimeContext(), entry);
}

async function openWorkloadInWorkbench(entry) {
  return openWorkloadInWorkbenchModule(workloadRuntimeContext(), entry);
}

async function attachCurrentBundleToWorkload(entry) {
  return attachCurrentBundleToWorkloadModule(workloadRuntimeContext(), entry);
}

function saveHubWorkloadLibrary(entries) {
  return saveHubWorkloadLibraryModule(workloadRuntimeContext(), entries);
}

function matchesWorkloadFilter(entry) {
  return matchesWorkloadFilterModule(workloadRuntimeContext(), entry);
}

function matchesWorkloadFamilyFilter(entry) {
  return matchesWorkloadFamilyFilterModule(workloadRuntimeContext(), entry);
}

function currentWorkloadLibrarySearchQuery() {
  return currentWorkloadLibrarySearchQueryModule(workloadRuntimeContext());
}

function matchesWorkloadSearchQuery(entry) {
  return matchesWorkloadSearchQueryModule(workloadRuntimeContext(), entry);
}

function renderWorkloadFilters() {
  return renderWorkloadFiltersModule({ elements, state });
}

function renderHubWorkloadLibrary(entries = loadHubWorkloadLibrary()) {
  const domainLabel = localizedWorkloadFilterLabel(state.workloadFilter);
  const familyLabel = localizedWorkloadFamilyFilterLabel(state.workloadFamilyFilter);
  const query = currentWorkloadLibrarySearchQuery();
  return renderHubWorkloadLibraryList({
    elements,
    entries,
    state,
    renderWorkloadFilters,
    renderEmptyHistoryState,
    emptyMessage:
      hubCopy().dynamic?.managedWorkloadsEmpty
      || HUB_I18N.en.dynamic?.managedWorkloadsEmpty
      || "No managed workloads yet. Register a current bundle or sync a remote catalog.",
    filterEmptyMessage: hubMessage(
      hubCopy().dynamic?.managedWorkloadsFilterEmpty
        || HUB_I18N.en.dynamic?.managedWorkloadsFilterEmpty
        || "No workloads match {domain} / {family} for \"{query}\".",
      {
        domain: domainLabel,
        family: familyLabel,
        query: query || "--",
      },
    ),
    matchesWorkloadFilter,
    matchesWorkloadSearchQuery,
    workloadSourceBadge,
    formatProjectActionTime,
    appendTextElement,
    workloadDomainLabel,
    workloadFamilyLabel,
    workloadProvenanceLabel,
    currentSearchQuery: query,
    setWorkloadLibraryOutput,
    hubMessage,
    restoredWorkloadContextMessage:
      hubCopy().dynamic?.restoredWorkloadContext
      || HUB_I18N.en.dynamic?.restoredWorkloadContext
      || "restored workload context for {label}",
    loadedWorkloadContextMessage:
      hubCopy().dynamic?.loadedWorkloadContext
      || HUB_I18N.en.dynamic?.loadedWorkloadContext
      || "loaded {label} into the bundle path",
    removedWorkloadMessage:
      hubCopy().dynamic?.removedWorkload
      || HUB_I18N.en.dynamic?.removedWorkload
      || "removed {label} from the workload library",
    renderAssistantContext,
    renderHubAssistantLocalCards,
    openWorkloadInWorkbench,
    formatHubOperatorError,
    runAction,
    downloadRemoteWorkloadBundle,
    attachCurrentBundleToWorkload,
    loadHubWorkloadLibrary,
    saveHubWorkloadLibrary,
    workloadIdentity,
    projectBundlePath: elements.projectBundlePath,
    workloadCatalogUrl: elements.workloadCatalogUrl,
    hubCopy,
    hubI18nEn: HUB_I18N.en,
  });
}

function projectSummaryFromInspectPayload(raw) {
  return parseProjectSummaryFromInspectPayload(raw);
}

async function registerCurrentBundleAsWorkload() {
  return registerCurrentBundleAsWorkloadModule(workloadRuntimeContext());
}

function validateHubCatalogUrl(value) {
  return validateWorkloadCatalogUrl(value);
}

function validateRemoteWorkloadCatalogPayload(payload) {
  return validateWorkloadCatalogPayload(payload);
}

function normalizeRemoteWorkloadCatalogPayload(payload, catalogUrl) {
  return buildNormalizedRemoteWorkloadCatalogPayload(payload, catalogUrl);
}

async function syncRemoteWorkloadCatalog(urlOverride = "") {
  return syncRemoteWorkloadCatalogModule(workloadRuntimeContext(), urlOverride);
}

async function syncLocalControlPlaneWorkloads() {
  return syncLocalControlPlaneWorkloadsModule(workloadRuntimeContext());
}

function exportHubWorkloadLibrary() {
  return exportHubWorkloadLibraryModule(workloadRuntimeContext());
}

async function importHubWorkloadLibrary(file) {
  return importHubWorkloadLibraryModule(workloadRuntimeContext(), file);
}

function clearHubWorkloadLibrary() {
  return clearHubWorkloadLibraryModule(workloadRuntimeContext());
}

function pushRecentValue(values, value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return values.slice(0, HUB_RECENTS_LIMIT);
  }

  return [normalized, ...values.filter((entry) => entry !== normalized)].slice(0, HUB_RECENTS_LIMIT);
}

function summarizeProjectActionResult(value) {
  const normalized = String(value || "").replace(/\s+/g, " ").trim();
  if (!normalized) {
    return "";
  }

  return normalized.length > 140 ? `${normalized.slice(0, 137)}...` : normalized;
}

function formatProjectActionTime(value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return "";
  }

  const timestamp = new Date(normalized);
  if (Number.isNaN(timestamp.getTime())) {
    return normalized;
  }

  return timestamp.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function projectActionStateClass(status) {
  switch (String(status || "").trim()) {
    case "ok":
      return "desktop-shell-state desktop-shell-state--healthy";
    case "failed":
      return "desktop-shell-state desktop-shell-state--danger";
    default:
      return "desktop-shell-state desktop-shell-state--idle";
  }
}

function rememberProjectBundleAction(
  action,
  { bundlePath = "", comparePath = "", outputPath = "", status = "idle", note = "", executedAt = "" } = {},
) {
  const normalizedAction = String(action || "").trim();
  if (!normalizedAction) {
    return [];
  }

  const recents = loadHubRecents();
  const existingEntry = (recents.actions ?? []).find((entry) => {
    return (
      entry.action === normalizedAction &&
      String(entry.bundlePath || "").trim() === String(bundlePath || "").trim() &&
      String(entry.comparePath || "").trim() === String(comparePath || "").trim() &&
      String(entry.outputPath || "").trim() === String(outputPath || "").trim()
    );
  });
  const nextEntry = {
    action: normalizedAction,
    bundlePath: String(bundlePath || "").trim(),
    comparePath: String(comparePath || "").trim(),
    outputPath: String(outputPath || "").trim(),
    status: String(status || "idle").trim() || "idle",
    note: summarizeProjectActionResult(note),
    executedAt: String(executedAt || "").trim() || new Date().toISOString(),
    pinned: Boolean(existingEntry?.pinned),
    favoriteLabel: String(existingEntry?.favoriteLabel || "").trim(),
  };

  return [
    nextEntry,
    ...(recents.actions ?? []).filter((entry) => {
      return !(
        entry.action === nextEntry.action &&
        entry.bundlePath === nextEntry.bundlePath &&
        entry.comparePath === nextEntry.comparePath &&
        entry.outputPath === nextEntry.outputPath
      );
    }),
  ].slice(0, HUB_ACTION_HISTORY_LIMIT);
}

function normalizeImportedProjectAction(entry) {
  const normalizedAction = String(entry?.action || "").trim();
  if (!normalizedAction) {
    return null;
  }

  return {
    action: normalizedAction,
    bundlePath: String(entry?.bundlePath || "").trim(),
    comparePath: String(entry?.comparePath || "").trim(),
    outputPath: String(entry?.outputPath || "").trim(),
    status: String(entry?.status || "idle").trim() || "idle",
    note: summarizeProjectActionResult(entry?.note || ""),
    executedAt: String(entry?.executedAt || "").trim() || new Date().toISOString(),
    pinned: Boolean(entry?.pinned),
    favoriteLabel: String(entry?.favoriteLabel || "").trim(),
  };
}

function mergeProjectActionHistory(existingActions, importedActions) {
  const merged = [];

  for (const entry of [...importedActions, ...existingActions]) {
    const normalized = normalizeImportedProjectAction(entry);
    if (!normalized) {
      continue;
    }

    const duplicateIndex = merged.findIndex((candidate) => {
      return (
        candidate.action === normalized.action &&
        candidate.bundlePath === normalized.bundlePath &&
        candidate.comparePath === normalized.comparePath &&
        candidate.outputPath === normalized.outputPath
      );
    });

    if (duplicateIndex >= 0) {
      continue;
    }

    merged.push(normalized);
    if (merged.length >= HUB_ACTION_HISTORY_LIMIT) {
      break;
    }
  }

  return merged;
}

function actionIdentity(entry) {
  return [
    String(entry?.action || "").trim(),
    String(entry?.bundlePath || "").trim(),
    String(entry?.comparePath || "").trim(),
    String(entry?.outputPath || "").trim(),
  ].join("::");
}

function shellQuote(value) {
  const normalized = String(value || "");
  if (!normalized) {
    return "''";
  }

  return `'${normalized.replace(/'/g, `'\\''`)}'`;
}

function buildProjectCliCommand(entry) {
  const action = String(entry?.action || "").trim();
  const bundlePath = String(entry?.bundlePath || "").trim();
  const comparePath = String(entry?.comparePath || "").trim();
  const outputPath = String(entry?.outputPath || "").trim();

  switch (action) {
    case "project inspect":
      return `kyuubiki project inspect ${shellQuote(bundlePath)} --json`;
    case "project validate":
      return `kyuubiki project validate ${shellQuote(bundlePath)} --json`;
    case "project normalize":
      return `kyuubiki project normalize ${shellQuote(bundlePath)} --out ${shellQuote(outputPath)}`;
    case "project unpack":
      return `kyuubiki project unpack ${shellQuote(bundlePath)} --out ${shellQuote(outputPath)}`;
    case "project pack":
      return `kyuubiki project pack ${shellQuote(bundlePath)} --out ${shellQuote(outputPath)}`;
    case "project diff":
      return `kyuubiki project diff ${shellQuote(bundlePath)} ${shellQuote(comparePath)} --json`;
    default:
      return "";
  }
}

function buildPythonMacroStub(entry) {
  const action = String(entry?.action || "").trim();
  const bundlePath = JSON.stringify(String(entry?.bundlePath || "").trim());
  const comparePath = JSON.stringify(String(entry?.comparePath || "").trim());
  const outputPath = JSON.stringify(String(entry?.outputPath || "").trim());
  const label = JSON.stringify(String(entry?.favoriteLabel || entry?.action || "favorite-workflow").trim());

  switch (action) {
    case "project inspect":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectInspect", {"path": ${bundlePath}})\n`;
    case "project validate":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectValidate", {"path": ${bundlePath}})\n`;
    case "project normalize":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectNormalize", {"path": ${bundlePath}, "out": ${outputPath}})\n`;
    case "project unpack":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectUnpack", {"path": ${bundlePath}, "out": ${outputPath}})\n`;
    case "project pack":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectPack", {"path": ${bundlePath}, "out": ${outputPath}})\n`;
    case "project diff":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectDiff", {"leftPath": ${bundlePath}, "rightPath": ${comparePath}})\n`;
    default:
      return "";
  }
}

async function copyProjectCliCommand(entry) {
  const command = buildProjectCliCommand(entry);
  if (!command) {
    setProjectBundleOutput(`cannot build CLI command for ${entry.action}`);
    return;
  }

  await navigator.clipboard.writeText(command);
  setProjectBundleOutput(`copied CLI command for ${entry.favoriteLabel || entry.action}`);
}

async function copyPythonMacroStub(entry) {
  const snippet = buildPythonMacroStub(entry);
  if (!snippet) {
    setProjectBundleOutput(`cannot build Python stub for ${entry.action}`);
    return;
  }

  await navigator.clipboard.writeText(snippet);
  setProjectBundleOutput(`copied Python stub for ${entry.favoriteLabel || entry.action}`);
}

function sortProjectActionHistory(actions) {
  return [...actions].sort((left, right) => {
    if (Boolean(left?.pinned) !== Boolean(right?.pinned)) {
      return left?.pinned ? -1 : 1;
    }

    const leftTime = new Date(String(left?.executedAt || "")).getTime();
    const rightTime = new Date(String(right?.executedAt || "")).getTime();
    return (Number.isNaN(rightTime) ? 0 : rightTime) - (Number.isNaN(leftTime) ? 0 : leftTime);
  });
}

function saveProjectBundleRecents({
  action = "",
  bundlePath = "",
  comparePath = "",
  outputPath = "",
  status = "idle",
  note = "",
  executedAt = "",
} = {}) {
  const next = loadHubRecents();
  next.bundles = pushRecentValue(next.bundles, bundlePath);
  next.compares = pushRecentValue(next.compares, comparePath);
  next.outputs = pushRecentValue(next.outputs, outputPath);
  next.actions = rememberProjectBundleAction(action, { bundlePath, comparePath, outputPath, status, note, executedAt });
  saveHubRecents(next);
}

function renderRecentPathList(container, values, input) {
  if (!container) {
    return;
  }

  container.innerHTML = "";
  if (!values.length) {
    const empty = document.createElement("div");
    empty.className = "hub-recent-empty";
    empty.textContent = hubCopy().dynamic?.recentEntriesEmpty || HUB_I18N.en.dynamic?.recentEntriesEmpty || "No recent entries yet.";
    container.appendChild(empty);
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

function renderHubRecents(recents = loadHubRecents()) {
  renderRecentPathList(elements.recentBundleList, recents.bundles, elements.projectBundlePath);
  renderRecentPathList(elements.recentCompareList, recents.compares, elements.projectBundleComparePath);
  renderRecentPathList(elements.recentOutputList, recents.outputs, elements.projectBundleOutPath);
  renderHistoryFilters();
  renderRecentActionHistory(sortProjectActionHistory(recents.actions ?? []));
  renderHubWorkloadLibrary();
  renderAssistantContext();
  renderHubAssistantLocalCards();
}

function renderRecentActionHistory(actions) {
  if (!elements.recentActionList || !elements.favoriteActionList) {
    return;
  }

  const filteredActions = actions.filter((entry) => matchesHistoryFilter(entry));
  const favoriteActions = filteredActions.filter((entry) => entry.pinned);
  const recentActions = filteredActions.filter((entry) => !entry.pinned);
  elements.favoriteActionList.innerHTML = "";
  elements.recentActionList.innerHTML = "";
  if (!actions.length) {
    renderEmptyHistoryState(
      elements.favoriteActionList,
      hubCopy().dynamic?.favoritesEmpty || HUB_I18N.en.dynamic?.favoritesEmpty || "No favorite actions yet.",
    );
    renderEmptyHistoryState(
      elements.recentActionList,
      hubCopy().dynamic?.recentActionsEmpty || HUB_I18N.en.dynamic?.recentActionsEmpty || "No recent project actions yet.",
    );
    return;
  }

  if (!filteredActions.length) {
    renderEmptyHistoryState(
      elements.favoriteActionList,
      hubMessage(
        hubCopy().dynamic?.favoritesFilterEmpty || HUB_I18N.en.dynamic?.favoritesFilterEmpty || "No favorites match the {filter} filter.",
        { filter: localizedHistoryFilterLabel(state.historyFilter) },
      ),
    );
    renderEmptyHistoryState(
      elements.recentActionList,
      hubMessage(
        hubCopy().dynamic?.actionsFilterEmpty || HUB_I18N.en.dynamic?.actionsFilterEmpty || "No actions match the {filter} filter.",
        { filter: localizedHistoryFilterLabel(state.historyFilter) },
      ),
    );
    return;
  }

  if (!favoriteActions.length) {
    renderEmptyHistoryState(
      elements.favoriteActionList,
      hubCopy().dynamic?.pinnedFavoritesEmpty || HUB_I18N.en.dynamic?.pinnedFavoritesEmpty || "No pinned favorites yet.",
    );
  } else {
    renderProjectActionEntries(elements.favoriteActionList, favoriteActions);
  }

  if (!recentActions.length) {
    renderEmptyHistoryState(
      elements.recentActionList,
      hubCopy().dynamic?.nonPinnedEmpty || HUB_I18N.en.dynamic?.nonPinnedEmpty || "No non-pinned actions in this view.",
    );
  } else {
    renderProjectActionEntries(elements.recentActionList, recentActions);
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

    const button = document.createElement("button");
    button.type = "button";
    button.className = "hub-history-item__summary desktop-shell-button-ghost";
    const paths = [entry.bundlePath, entry.comparePath, entry.outputPath].filter(Boolean).join("  •  ");
    const badge = `<span class="${projectActionStateClass(entry.status)}">${entry.status || "idle"}</span>`;
    const time = formatProjectActionTime(entry.executedAt);
    const meta = [badge, time ? `<span>${time}</span>` : ""].filter(Boolean).join("");
    const details = summarizeProjectActionResult(entry.note) || paths || "No stored paths";
    const title = entry.pinned && entry.favoriteLabel ? entry.favoriteLabel : entry.action;
    button.innerHTML = `
      <div class="hub-history-item__heading">
        <strong>${title}</strong>
        <div class="hub-history-item__meta">${meta}</div>
      </div>
      ${entry.pinned && entry.favoriteLabel ? `<span class="hub-history-item__alias">${entry.action}</span>` : ""}
      <span>${details}</span>
    `;
    button.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      setProjectBundleOutput(
        hubMessage(
          hubCopy().dynamic?.restoredActionContext || HUB_I18N.en.dynamic?.restoredActionContext || "restored {action} context",
          { action: entry.action },
        ),
      );
    });

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";

    const restoreButton = document.createElement("button");
    restoreButton.type = "button";
    restoreButton.className = "desktop-shell-button-ghost";
    restoreButton.textContent = hubCopy().dynamic?.actionRestore || HUB_I18N.en.dynamic?.actionRestore || "Restore";
    restoreButton.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      setProjectBundleOutput(
        hubMessage(
          hubCopy().dynamic?.restoredActionContext || HUB_I18N.en.dynamic?.restoredActionContext || "restored {action} context",
          { action: entry.action },
        ),
      );
    });

    const rerunButton = document.createElement("button");
    rerunButton.type = "button";
    rerunButton.className = "desktop-shell-button-primary";
    rerunButton.textContent = hubCopy().dynamic?.actionRerun || HUB_I18N.en.dynamic?.actionRerun || "Re-run";
    rerunButton.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      void rerunProjectActionEntry(entry);
    });

    const pinButton = document.createElement("button");
    pinButton.type = "button";
    pinButton.className = entry.pinned ? "desktop-shell-button-primary" : "desktop-shell-button-ghost";
    pinButton.textContent = entry.pinned
      ? hubCopy().dynamic?.actionPinned || HUB_I18N.en.dynamic?.actionPinned || "Pinned"
      : hubCopy().dynamic?.actionPin || HUB_I18N.en.dynamic?.actionPin || "Pin";
    pinButton.addEventListener("click", () => {
      togglePinnedProjectAction(entry);
    });

    controls.append(restoreButton);

    if (entry.pinned) {
      const renameButton = document.createElement("button");
      renameButton.type = "button";
      renameButton.className = "desktop-shell-button-ghost";
      renameButton.textContent = hubCopy().dynamic?.actionLabel || HUB_I18N.en.dynamic?.actionLabel || "Label";
      renameButton.addEventListener("click", () => {
        renamePinnedProjectAction(entry);
      });
      controls.append(renameButton);

      const copyButton = document.createElement("button");
      copyButton.type = "button";
      copyButton.className = "desktop-shell-button-ghost";
      copyButton.textContent = hubCopy().dynamic?.actionCopyCli || HUB_I18N.en.dynamic?.actionCopyCli || "Copy CLI";
      copyButton.addEventListener("click", () => {
        void copyProjectCliCommand(entry);
      });
      controls.append(copyButton);

      const pythonButton = document.createElement("button");
      pythonButton.type = "button";
      pythonButton.className = "desktop-shell-button-ghost";
      pythonButton.textContent = hubCopy().dynamic?.actionCopyPython || HUB_I18N.en.dynamic?.actionCopyPython || "Copy Python";
      pythonButton.addEventListener("click", () => {
        void copyPythonMacroStub(entry);
      });
      controls.append(pythonButton);
    }

    controls.append(pinButton, rerunButton);
    shell.append(button, controls);
    container.appendChild(shell);
  });
}

function renderHistoryFilters() {
  elements.historyFilterButtons.forEach((button) => {
    const isActive = button.dataset.historyFilter === state.historyFilter;
    button.classList.toggle("desktop-shell-button-primary", isActive);
    button.classList.toggle("desktop-shell-button-ghost", !isActive);
    button.setAttribute("aria-pressed", String(isActive));
  });
}

function matchesHistoryFilter(entry) {
  switch (state.historyFilter) {
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
  const recents = loadHubRecents();
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
  saveHubRecents(recents);
  setProjectBundleOutput(`${entry.pinned ? "unpinned" : "pinned"} ${entry.action}`);
}

function renamePinnedProjectAction(entry) {
  const currentLabel = String(entry.favoriteLabel || "");
  const nextLabel = window.prompt("Favorite label", currentLabel || entry.action);
  if (nextLabel === null) {
    return;
  }

  const recents = loadHubRecents();
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
  saveHubRecents(recents);
  setProjectBundleOutput(`updated label for ${entry.action}`);
}

function restoreProjectActionContext(entry) {
  elements.projectBundlePath.value = entry.bundlePath || "";
  elements.projectBundleComparePath.value = entry.comparePath || "";
  elements.projectBundleOutPath.value = entry.outputPath || "";
}

async function rerunProjectActionEntry(entry) {
  const action = PROJECT_ACTION_LABELS[entry.action];
  if (!action) {
    setProjectBundleOutput(`cannot re-run unknown action: ${entry.action}`);
    return;
  }

  await runAction(action);
}

async function runProjectBundleAction({ action, command, payload, outputTarget, successOutput }) {
  return executeProjectBundleAction({
    action,
    command,
    payload,
    invokeTauri,
    saveProjectBundleRecents,
    outputTarget,
    elements,
    setBusy,
    projectActionLabels: PROJECT_ACTION_LABELS,
    successOutput,
  });
}

async function invokeGuardedMutation(action, payload = {}) {
  return invokeTauri("guarded_mutation_action", {
    payload: {
      action,
      ...payload,
    },
  });
}

async function applyBrand() {
  const brand = await loadDesktopBrand();
  if (!brand) {
    return;
  }

  const releaseVersion = String(brand.releaseVersion || "").replace(/^v/u, "");
  const releaseCodename = String(brand.releaseCodename || "").trim();
  const releaseTag = [releaseCodename, releaseVersion].filter(Boolean).join(" ");

  if (brand.hubName) {
    state.releaseVersion = releaseVersion;
    state.releaseCodename = releaseCodename;
    document.title = releaseTag ? `${brand.hubName} · ${releaseTag}` : brand.hubName;
  }

  setText("brand-hub-title", brand.hubShortName || "Hub");
  setText("brand-hub-role", brand.shellRoleLabel);
  setText("brand-hub-role-chip", brand.shellRoleLabel);
  setText("brand-hub-focus", brand.shellFocusLabel);
  if (releaseTag) {
    setText("brand-hub-version", releaseTag);
  }
}

function releaseLabel() {
  const releaseTag = [state.releaseCodename, state.releaseVersion].filter(Boolean).join(" ");
  return releaseTag ? `Kyuubiki Hub · ${releaseTag}` : "Kyuubiki Hub";
}

function formatRuntimeReport(value) {
  const body = String(value || "").trim();
  return body ? `${releaseLabel()}\n\n${body}` : releaseLabel();
}

function setSection(section) {
  const next = hubCopy().sections[section];
  if (!next) return;

  state.activeSection = section;
  elements.title.textContent = next.title;
  elements.copy.textContent = next.copy;

  elements.navItems.forEach((item) => {
    const active = item.dataset.target === section;
    item.classList.toggle("hub-nav__item--active", active);
    item.setAttribute("aria-current", active ? "page" : "false");
  });

  elements.panels.forEach((panel) => {
    const hidden = panel.id !== `${section}-panel`;
    panel.classList.toggle("hidden", hidden);
    panel.setAttribute("aria-hidden", String(hidden));
  });

  const defaultProjectsPanel = document.getElementById("projects-panel");
  if (defaultProjectsPanel) {
    defaultProjectsPanel.classList.toggle("hidden", section !== "projects");
  }
  if (section === "projects") {
    renderProjectsPages();
  } else if (section in state.panelPages) {
    renderPanelPages(section);
  }

  renderAssistantContext();
  renderHubAssistantLocalCards();
  syncHotRuntimeLogPolling();
  syncObserveRuntimeLogPolling();
  if (section === "runtimes") {
    void refreshHotRuntimeLog({ silent: true });
  }
  if (section === "observe") {
    void refreshObserveRuntimeLog({ silent: true });
  }
}

function enhanceHubAccessibility() {
  elements.title?.setAttribute("tabindex", "-1");

  elements.navItems.forEach((item) => {
    const target = item.dataset.target || "";
    item.setAttribute("aria-controls", `${target}-panel`);
  });

  elements.sectionJumpButtons.forEach((button) => {
    const target = button.dataset.targetSection || "";
    button.setAttribute("aria-controls", `${target}-panel`);
  });

  elements.projectsPageButtons.forEach((button) => {
    const target = button.dataset.projectsPage || "";
    const pane = elements.projectsPanes.find((candidate) => candidate.dataset.projectsPane === target);
    if (!pane) {
      return;
    }

    if (!pane.id) {
      pane.id = `projects-pane-${target}`;
    }
    button.setAttribute("aria-controls", pane.id);
  });

  elements.panelPageButtons.forEach((button) => {
    const group = button.dataset.panelPageGroup || "";
    const target = button.dataset.panelPage || "";
    const pane = elements.panelPanes.find(
      (candidate) => candidate.dataset.panelPaneGroup === group && candidate.dataset.panelPane === target,
    );
    if (!pane) {
      return;
    }

    if (!pane.id) {
      pane.id = `panel-pane-${group}-${target}`;
    }
    button.setAttribute("aria-controls", pane.id);
  });

  if (elements.assistantPanel && !elements.assistantPanel.id) {
    elements.assistantPanel.id = "hub-assistant-panel";
  }
  if (elements.assistantFab && elements.assistantPanel) {
    elements.assistantFab.setAttribute("aria-controls", elements.assistantPanel.id);
  }

  elements.densityToggleButtons.forEach((button) => {
    const densityId = button.dataset.densityToggle || "";
    const panel = elements.densityPanels.find((candidate) => candidate.dataset.densityPanel === densityId);
    if (!panel) {
      return;
    }

    if (!panel.id) {
      panel.id = `density-panel-${densityId}`;
    }
    button.setAttribute("aria-controls", panel.id);
  });
}

function setOperationOutput(value) {
  elements.operationOutput.textContent = value;
}

function setDesktopStatusOutput(value) {
  if (elements.desktopStatusOutput) {
    elements.desktopStatusOutput.textContent = formatRuntimeReport(value);
  }
}

function setRuntimeStatusOutput(value) {
  elements.runtimeStatusOutput.textContent = formatRuntimeReport(value);
  if (elements.observeRuntimeStatusOutput) {
    elements.observeRuntimeStatusOutput.textContent = formatRuntimeReport(value);
  }
}

function setHotRuntimeStatusOutput(value) {
  if (elements.hotRuntimeStatusOutput) {
    elements.hotRuntimeStatusOutput.textContent = formatRuntimeReport(value);
  }
  if (elements.observeRuntimeStatusOutput) {
    elements.observeRuntimeStatusOutput.textContent = formatRuntimeReport(value);
  }
}

function setHotRuntimeLogOutput(value) {
  if (elements.hotRuntimeLogOutput) {
    elements.hotRuntimeLogOutput.textContent = value;
  }
  if (elements.observeHotLogOutput) {
    elements.observeHotLogOutput.textContent = value;
  }
}

function setObserveRuntimeLogOutput(value) {
  if (elements.observeRuntimeLogOutput) {
    elements.observeRuntimeLogOutput.textContent = value;
  }
}

function clearHotRuntimeLogView() {
  setHotRuntimeLogOutput(`Cleared local log view for ${currentHotRuntimeLogService()}. Background tail and log files are unchanged.`);
}

function sanitizeRuntimeLogForClipboard(text) {
  return sanitizeRuntimeLogForClipboardHelper(text);
}

async function copyHotRuntimeLogView() {
  await copySanitizedRuntimeLogToClipboard(
    String(elements.hotRuntimeLogOutput?.textContent || "").trim(),
  );
}

function renderHotRuntimeLogFollowState() {
  const label = shouldPollHotRuntimeLog() ? "following" : "frozen";
  applyDesktopState(elements.hotRuntimeLogFollowState, label, { kind: "activity" });
  applyDesktopState(elements.observeHotFollowState, label, { kind: "activity" });
}

function renderObserveRuntimeLogFollowState() {
  const label = shouldPollObserveRuntimeLog() ? "following" : "frozen";
  applyDesktopState(elements.observeRuntimeLogFollowState, label, { kind: "activity" });
}

function inferHotRuntimeState(rendered) {
  return inferHotRuntimeStateHelper(rendered, elements.hotRuntimeMode?.textContent?.trim() || "local");
}

function currentHotRuntimeStatus() {
  return String(elements.hotRuntimeStatus?.textContent || "").trim().toLowerCase();
}

function currentHotRuntimeLogService() {
  return elements.hotRuntimeLogService?.value || "hot-stack";
}

function currentObserveRuntimeLogService() {
  return elements.observeRuntimeLogService?.value || "frontend";
}

function renderHotRuntimeLogServiceLabel() {
  const label = currentHotRuntimeLogService();
  if (elements.observeHotLogService) {
    elements.observeHotLogService.textContent = label;
  }
}

function currentHotRuntimeLogAutoRefresh() {
  return elements.hotRuntimeLogAuto?.checked !== false;
}

function currentObserveRuntimeLogAutoRefresh() {
  return elements.observeRuntimeLogAuto?.checked !== false;
}

function currentHotRuntimeLogInterval() {
  const value = String(elements.hotRuntimeLogInterval?.value || "4000");
  return ["2000", "4000", "8000"].includes(value) ? Number(value) : 4000;
}

function persistCurrentHotLogSettings() {
  persistHubHotLogSettings({
    service: currentHotRuntimeLogService(),
    autoRefresh: currentHotRuntimeLogAutoRefresh(),
    interval: String(currentHotRuntimeLogInterval()),
  });
}

function persistCurrentObserveRuntimeLogSettings() {
  persistHubRuntimeLogSettings({
    service: currentObserveRuntimeLogService(),
    autoRefresh: currentObserveRuntimeLogAutoRefresh(),
  });
}

function shouldPollHotRuntimeLog() {
  return state.activeSection === "runtimes"
    && currentHotRuntimeStatus() === "running"
    && currentHotRuntimeLogAutoRefresh();
}

function shouldPollObserveRuntimeLog() {
  return state.activeSection === "observe" && currentObserveRuntimeLogAutoRefresh();
}

function stopHotRuntimeLogPolling() {
  if (hotRuntimeLogPollHandle) {
    window.clearInterval(hotRuntimeLogPollHandle);
    hotRuntimeLogPollHandle = null;
  }
  renderHotRuntimeLogFollowState();
}

function stopObserveRuntimeLogPolling() {
  if (observeRuntimeLogPollHandle) {
    window.clearInterval(observeRuntimeLogPollHandle);
    observeRuntimeLogPollHandle = null;
  }
  renderObserveRuntimeLogFollowState();
}

function syncHotRuntimeLogPolling() {
  if (!shouldPollHotRuntimeLog()) {
    stopHotRuntimeLogPolling();
    return;
  }

  if (hotRuntimeLogPollHandle) {
    renderHotRuntimeLogFollowState();
    return;
  }

  hotRuntimeLogPollHandle = window.setInterval(() => {
    void refreshHotRuntimeLog({ silent: true });
  }, currentHotRuntimeLogInterval() || HUB_HOT_LOG_POLL_MS);
  renderHotRuntimeLogFollowState();
}

function syncObserveRuntimeLogPolling() {
  if (!shouldPollObserveRuntimeLog()) {
    stopObserveRuntimeLogPolling();
    return;
  }

  if (observeRuntimeLogPollHandle) {
    renderObserveRuntimeLogFollowState();
    return;
  }

  observeRuntimeLogPollHandle = window.setInterval(() => {
    void refreshObserveRuntimeLog({ silent: true });
  }, HUB_HOT_LOG_POLL_MS);
  renderObserveRuntimeLogFollowState();
}

function setProjectBundleOutput(value) {
  elements.projectBundleOutput.textContent = value;
}

function setAssistantOutput(value) {
  if (elements.assistantOutput) {
    elements.assistantOutput.textContent = value;
  }
}

function setAssistantLocalOutput(value) {
  if (elements.assistantLocalOutput) {
    elements.assistantLocalOutput.textContent = value;
  }
}

function setWorkflowCatalogOutput(value) {
  if (elements.workflowCatalogOutput) {
    elements.workflowCatalogOutput.textContent = value;
  }
}

function renderProjectsPages() {
  elements.projectsPageButtons.forEach((button) => {
    const active = button.dataset.projectsPage === state.projectsPage;
    button.classList.toggle("hub-panel-tab--active", active);
    button.setAttribute("aria-pressed", String(active));
  });

  elements.projectsPanes.forEach((pane) => {
    const active = pane.dataset.projectsPane === state.projectsPage;
    pane.classList.toggle("hidden", !active);
    pane.setAttribute("aria-hidden", String(!active));
  });
}

function setProjectsPage(page) {
  state.projectsPage =
    page === "library" || page === "bundles" || page === "guides" ? page : "start";
  renderProjectsPages();
  if (state.projectsPage === "library" && !state.workflowCatalog.length && !state.workflowCatalogBusy) {
    void fetchWorkflowCatalog({ silent: true });
  }
}

function builtInWorkflowSampleInputArtifacts(workflowId) {
  return buildBuiltInWorkflowSampleInputArtifacts(workflowId);
}

function describeWorkflowSummary(resultPayload) {
  return buildWorkflowSummaryDescription(resultPayload);
}

async function waitForWorkflowJob(jobId, options = {}) {
  return waitForWorkflowCatalogJob(jobId, {
    ...options,
    currentOrchestratorBaseUrl,
    hubMessage,
    localizedWorkflowCatalogLabel,
    setWorkflowCatalogOutput,
  });
}

async function runWorkflowCatalogSample(entry) {
  return runWorkflowCatalogSamplePanel(entry, {
    state,
    applyDesktopState,
    actionState: elements.actionState,
    currentWorkflowCatalogUrl,
    currentOrchestratorBaseUrl,
    hubMessage,
    localizedWorkflowCatalogLabel,
    setWorkflowCatalogOutput,
    setOperationOutput,
    formatHubOperatorError,
    renderWorkflowCatalog,
  });
}

async function fetchWorkflowCatalog(options = {}) {
  return fetchWorkflowCatalogPanel({
    ...options,
    state,
    renderWorkflowCatalog,
    setWorkflowCatalogOutput,
    setOperationOutput,
    applyDesktopState,
    actionState: elements.actionState,
    currentWorkflowCatalogUrl,
    hubMessage,
    localizedWorkflowCatalogLabel,
    formatHubOperatorError,
  });
}

function renderWorkflowCatalog(entries = state.workflowCatalog) {
  return renderWorkflowCatalogPanel(entries, {
    workflowCatalogList: elements.workflowCatalogList,
    workflowCatalogBusy: state.workflowCatalogBusy,
    workflowCatalogQuery: elements.workflowCatalogSearch?.value || "",
    renderEmptyHistoryState,
    localizedWorkflowCatalogLabel,
    appendTextElement,
    hubMessage,
    runWorkflowCatalogSample,
  });
}

function renderPanelPages(group) {
  const activePage = state.panelPages[group];
  elements.panelPageButtons
    .filter((button) => button.dataset.panelPageGroup === group)
    .forEach((button) => {
      const active = button.dataset.panelPage === activePage;
      button.classList.toggle("hub-panel-tab--active", active);
      button.setAttribute("aria-pressed", String(active));
    });

  elements.panelPanes
    .filter((pane) => pane.dataset.panelPaneGroup === group)
    .forEach((pane) => {
      const active = pane.dataset.panelPane === activePage;
      pane.classList.toggle("hidden", !active);
      pane.setAttribute("aria-hidden", String(!active));
    });
}

function setPanelPage(group, page) {
  if (!(group in state.panelPages)) {
    return;
  }
  state.panelPages[group] = page || state.panelPages[group];
  renderPanelPages(group);
}

function renderAssistantPanel() {
  const open = state.assistantOpen === true;
  elements.assistantPanel?.classList.toggle("hidden", !open);
  elements.assistantPanel?.setAttribute("aria-hidden", String(!open));
  elements.assistantFab?.setAttribute("aria-expanded", String(open));
}

function setAssistantPanelOpen(open) {
  state.assistantOpen = open === true;
  renderAssistantPanel();
}

function currentProjectBundlePayload() {
  return buildCurrentProjectBundlePayload(elements);
}

function currentProjectBundleOutputPayload() {
  return buildCurrentProjectBundleOutputPayload(elements);
}

function currentProjectBundleComparePayload() {
  return buildCurrentProjectBundleComparePayload(elements);
}

function currentAssistantSnapshot() {
  return {
    activeSection: state.activeSection,
    runtimeStatus: elements.localRuntimeStatus?.textContent?.trim() || "unknown",
    profile: elements.currentProfile?.textContent?.trim() || "unknown",
    bundlePath: elements.projectBundlePath?.value?.trim() || "",
    comparePath: elements.projectBundleComparePath?.value?.trim() || "",
    outputPath: elements.projectBundleOutPath?.value?.trim() || "",
    favorites: loadHubRecents().actions?.filter((entry) => entry.pinned).length ?? 0,
  };
}

function renderAssistantContext() {
  const snapshot = currentAssistantSnapshot();
  setText(elements.assistantContextSection, snapshot.activeSection);
  setText(elements.assistantContextRuntime, snapshot.runtimeStatus);
  setText(elements.assistantContextBundle, snapshot.bundlePath || "--");
}

function setAssistantMode(mode) {
  state.assistantMode = mode === "llm" ? "llm" : "local";
  elements.assistantModeButtons.forEach((button) => {
    const active = button.dataset.assistantMode === state.assistantMode;
    button.classList.toggle("desktop-shell-button-primary", active);
    button.classList.toggle("desktop-shell-button-ghost", !active);
    button.setAttribute("aria-pressed", String(active));
  });
  elements.assistantLocalPanel?.classList.toggle("hidden", state.assistantMode !== "local");
  elements.assistantLlmPanel?.classList.toggle("hidden", state.assistantMode !== "llm");
  applyDesktopState(elements.assistantEngineState, state.assistantMode === "llm" ? "remote model" : "local guide", {
    kind: "activity",
  });
  persistHubAssistantSettings({
    ...loadHubAssistantSettings(),
    mode: state.assistantMode,
    baseUrl: elements.assistantBaseUrl?.value || "",
    modelPreset: elements.assistantModelPreset?.value || "gpt-5",
    model: elements.assistantModelName?.value || "gpt-5",
  });
}

function renderHubDensityToggles() {
  elements.densityPanels.forEach((panel) => {
    const densityId = panel.dataset.densityPanel || "";
    const expanded = state.density[densityId] !== false;
    panel.classList.toggle("hidden", !expanded);
  });

  elements.densityToggleButtons.forEach((button) => {
    const densityId = button.dataset.densityToggle || "";
    const expanded = state.density[densityId] !== false;
    button.textContent = expanded ? "Collapse" : "Expand";
    button.setAttribute("aria-expanded", String(expanded));
  });
}

function hubDynamic(key, replacements = {}) {
  return hubMessage(hubCopy().dynamic?.[key] || HUB_I18N.en.dynamic?.[key] || "", replacements);
}

function toggleHubDensityPanel(id) {
  if (!(id in HUB_DENSITY_DEFAULTS)) {
    return;
  }

  state.density[id] = !(state.density[id] !== false);
  persistHubDensitySettings();
  renderHubDensityToggles();
}

function buildHubAssistantLocalCards() {
  return buildHubAssistantLocalCardsModule({
    currentAssistantSnapshot,
    hubDynamic,
    homeBundlesTitle: hubCopy().home.quick.bundlesTitle,
    shellStartLocal: hubCopy().shell.startLocal,
    bundleInspect: hubCopy().bundles.inspect,
    bundleNormalize: hubCopy().bundles.normalize,
    bundleDiff: hubCopy().bundles.diff,
    homeGuidesTitle: hubCopy().home.tabs.guides,
    shellOpenWorkbench: hubCopy().shell.openWorkbench,
    setSection,
    setProjectsPage,
    projectBundlePath: elements.projectBundlePath,
    setProjectBundleOutput,
    runAction,
  });
}

function renderHubAssistantLocalCards() {
  return renderHubAssistantLocalCardsModule({
    assistantLocalCards: elements.assistantLocalCards,
    renderEmptyHistoryState,
    hubDynamic,
    appendAssistantCardHeader,
    appendTextElement,
    currentAssistantSnapshot,
    homeBundlesTitle: hubCopy().home.quick.bundlesTitle,
    shellStartLocal: hubCopy().shell.startLocal,
    bundleInspect: hubCopy().bundles.inspect,
    bundleNormalize: hubCopy().bundles.normalize,
    bundleDiff: hubCopy().bundles.diff,
    homeGuidesTitle: hubCopy().home.tabs.guides,
    shellOpenWorkbench: hubCopy().shell.openWorkbench,
    setSection,
    setProjectsPage,
    projectBundlePath: elements.projectBundlePath,
    setProjectBundleOutput,
    runAction,
  });
}

function buildLocalGuideResponse(query) {
  return buildLocalGuideResponseModule(query, {
    currentAssistantSnapshot,
    hubDynamic,
  });
}

function answerWithLocalGuide() {
  const query = elements.assistantLocalPrompt?.value || "";
  setAssistantLocalOutput(buildLocalGuideResponse(query));
}

function extractAssistantJsonBlock(value) {
  return extractAssistantJsonBlockModule(value);
}

function validateAssistantBaseUrl(value) {
  const baseUrl = value.trim();
  if (!baseUrl) {
    return { ok: false, reason: "Fill in the assistant base URL before requesting a plan." };
  }

  let parsed;
  try {
    parsed = new URL(baseUrl);
  } catch {
    return { ok: false, reason: "Assistant base URL must be a valid absolute URL." };
  }

  const protocol = parsed.protocol.toLowerCase();
  const hostname = parsed.hostname.toLowerCase();
  const isLoopback =
    hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";

  if (protocol === "https:") {
    return { ok: true, normalized: baseUrl };
  }

  if (protocol === "http:" && isLoopback) {
    return { ok: true, normalized: baseUrl };
  }

  return {
    ok: false,
    reason: "Assistant base URL must use https, or http only for localhost / 127.0.0.1 / ::1.",
  };
}

function updateAssistantEndpointPolicy() {
  if (!elements.assistantEndpointPolicy || !elements.assistantBaseUrl) {
    return;
  }

  const baseUrl = elements.assistantBaseUrl.value.trim();
  if (!baseUrl) {
    elements.assistantEndpointPolicy.textContent = hubDynamic("endpointPolicyDefault");
    return;
  }

  const validation = validateAssistantBaseUrl(baseUrl);
  if (!validation.ok) {
    elements.assistantEndpointPolicy.textContent = `${validation.reason} The API key is sent directly to the configured base URL.`;
    return;
  }

  elements.assistantEndpointPolicy.textContent = hubDynamic("endpointPolicyAllowed");
}

function assistantTrustHostOrigin(baseUrl) {
  return assistantTrustHostOriginEngine(baseUrl);
}

function assistantHostRequiresTrust(baseUrl) {
  return assistantHostRequiresTrustEngine(baseUrl);
}

function ensureAssistantHostTrust(baseUrl, apiKey) {
  return ensureAssistantHostTrustEngine(baseUrl, apiKey, {
    trustedHosts: state.assistantTrustedHosts,
    persistTrustedHosts: persistHubAssistantTrustedHosts,
    confirm: window.confirm.bind(window),
  });
}

function ensureRemoteHostTrust(baseUrl, label) {
  return ensureRemoteHostTrustEngine(baseUrl, label, {
    trustedHosts: state.remoteTrustedHosts,
    persistTrustedHosts: (trustedHosts) => persistHubTrustedHosts(HUB_REMOTE_TRUSTED_HOSTS_KEY, trustedHosts),
    confirm: window.confirm.bind(window),
  });
}

async function requestHubAssistantPlan() {
  return requestHubAssistantPlanEngine({
    assistantBaseUrl: elements.assistantBaseUrl,
    assistantModelName: elements.assistantModelName,
    assistantPrompt: elements.assistantPrompt,
    assistantApiKey: elements.assistantApiKey,
    validateAssistantBaseUrl,
    assistantTrustedHosts: state.assistantTrustedHosts,
    persistAssistantTrustedHosts: persistHubAssistantTrustedHosts,
    confirm: window.confirm.bind(window),
    currentAssistantSnapshot,
    assistantActions: HUB_ASSISTANT_ACTIONS,
    buildHubAssistantLocalCards,
    extractAssistantJsonBlock,
  });
}

function renderHubAssistantPlan() {
  return renderHubAssistantPlanEngine({
    assistantPlanActions: elements.assistantPlanActions,
    assistantPlan: state.assistantPlan,
    renderEmptyHistoryState,
    hubDynamic,
    appendAssistantCardHeader,
    appendTextElement,
    assistantRiskLevel,
    assistantRiskStateClass,
    executeHubAssistantAction,
  });
}

function confirmHubAssistantAction(action, source = "assistant") {
  return confirmHubAssistantActionEngine(action, source, {
    assistantRiskLevel,
    rememberHubAssistantAudit,
    confirm: window.confirm.bind(window),
  });
}

function applyAssistantBundlePayload(payload) {
  return applyAssistantBundlePayloadEngine(payload, {
    projectBundlePath: elements.projectBundlePath,
    projectBundleComparePath: elements.projectBundleComparePath,
    projectBundleOutPath: elements.projectBundleOutPath,
  });
}

async function executeHubAssistantAction(action, payload = {}, source = "assistant") {
  return executeHubAssistantActionEngine(action, payload, source, {
    assistantRiskLevel,
    setAssistantOutput,
    hubDynamic,
    setSection,
    rememberHubAssistantAudit,
    runActionWithOptions,
    renderAssistantContext,
    projectBundlePath: elements.projectBundlePath,
    projectBundleComparePath: elements.projectBundleComparePath,
    projectBundleOutPath: elements.projectBundleOutPath,
    confirm: window.confirm.bind(window),
  });
}

async function executeHubAssistantPlan() {
  return executeHubAssistantPlanEngine({
    assistantPlan: state.assistantPlan,
    assistantApprovePlan: elements.assistantApprovePlan,
    setAssistantOutput,
    hubDynamic,
    executeHubAssistantAction,
    rememberHubAssistantAudit,
    assistantRiskLevel,
  });
}

function setBusy(isBusy, label = "idle") {
  state.isBusy = isBusy;
  applyDesktopState(elements.actionState, label, { kind: "activity" });
  elements.actionButtons.forEach((button) => {
    button.disabled = isBusy;
    button.classList.toggle("is-busy", isBusy);
  });
}

function syncAssistantSettingsFromInputs() {
  state.assistantApiKey = elements.assistantApiKey?.value || "";
  persistHubAssistantSettings({
    mode: state.assistantMode,
    baseUrl: elements.assistantBaseUrl?.value || "",
    modelPreset: elements.assistantModelPreset?.value || "gpt-5",
    model: elements.assistantModelName?.value || "gpt-5",
  });
}

function applyAssistantSettings() {
  clearLegacyHubAssistantSecrets();
  const settings = loadHubAssistantSettings();
  state.assistantMode = settings.mode;
  if (elements.assistantBaseUrl) {
    elements.assistantBaseUrl.value = settings.baseUrl;
  }
  if (elements.assistantModelPreset) {
    elements.assistantModelPreset.value = settings.modelPreset;
  }
  if (elements.assistantModelName) {
    elements.assistantModelName.value = settings.model;
  }
  if (elements.assistantApiKey) {
    elements.assistantApiKey.value = state.assistantApiKey || "";
  }
  setAssistantMode(settings.mode);
  updateAssistantEndpointPolicy();
  renderAssistantContext();
  renderHubAssistantLocalCards();
  renderHubAssistantPlan();
  renderHubAssistantAudit();
}

async function loadEnvironment() {
  const environment = await invokeTauri("hub_environment");
  state.hostPlatform = normalizeDesktopPlatform(environment.host_platform);

  populateDesktopPlatformSelect(elements.releasePlatform, {
    includeAll: true,
    fallback: state.hostPlatform,
  });

  if (elements.releasePlatform && !elements.releasePlatform.value) {
    elements.releasePlatform.value = state.hostPlatform;
  }
  renderToolsPlatformLabel();

  if (elements.workbenchUrl) {
    elements.workbenchUrl.textContent = environment.workbench_url;
  }

  if (elements.orchestratorUrl) {
    elements.orchestratorUrl.textContent = environment.orchestrator_url;
  }

  ensureDefaultWorkloadCatalogUrl();

  applyDesktopState(elements.currentRuntimeMode, "orchestrated_gui", { kind: "activity" });
  applyDesktopState(elements.currentProfile, environment.deployment_mode, { kind: "activity" });
  renderAssistantContext();
}

async function loadDirectMeshRegressionSnapshot() {
  try {
    state.directMeshRegressionSnapshot = await invokeTauri("hub_direct_mesh_regression_snapshot");
    renderDirectMeshRegressionSnapshot({
      elements,
      snapshot: state.directMeshRegressionSnapshot,
      copy: hubCopy(),
      regressionGateReport: state.regressionGateReport,
      applyDesktopState,
    });
  } catch (error) {
    renderDirectMeshRegressionLoadError({
      elements,
      copy: hubCopy(),
      error,
      applyDesktopState,
      formatHubOperatorError,
    });
  }
}

async function refreshRuntimeStatus() {
  await refreshRuntimeStatusPanel({
    invokeTauri,
    orchestratorBaseUrl: currentOrchestratorBaseUrl(),
    setRuntimeStatusOutput,
    applyDesktopState,
    localRuntimeStatus: elements.localRuntimeStatus,
    observeRuntimeStatus: elements.observeRuntimeStatus,
    runtimeStatusPlane: elements.runtimeStatusPlane,
  });
  renderAssistantContext();
  renderHubAssistantLocalCards();
}

async function refreshHotRuntimeStatus() {
  await refreshHotRuntimeStatusPanel({
    invokeTauri,
    setHotRuntimeStatusOutput,
    applyDesktopState,
    hotRuntimeStatus: elements.hotRuntimeStatus,
    observeHotStatus: elements.observeHotStatus,
    hotRuntimeMode: elements.hotRuntimeMode,
    observeHotMode: elements.observeHotMode,
    syncHotRuntimeLogPolling,
    refreshHotRuntimeLog,
  });
}

async function refreshHotRuntimeLog(options = {}) {
  await refreshRuntimeLogPanel({
    invokeTauri,
    state,
    inFlightKey: "hotLogRefreshInFlight",
    service: currentHotRuntimeLogService(),
    silent: options?.silent === true,
    setOutput: setHotRuntimeLogOutput,
    hubDynamic,
    formatHubOperatorError,
  });
}

async function refreshObserveRuntimeLog(options = {}) {
  await refreshRuntimeLogPanel({
    invokeTauri,
    state,
    inFlightKey: "runtimeLogRefreshInFlight",
    service: currentObserveRuntimeLogService(),
    silent: options?.silent === true,
    setOutput: setObserveRuntimeLogOutput,
    hubDynamic,
    formatHubOperatorError,
  });
}

async function copyObserveRuntimeLogView() {
  await copySanitizedRuntimeLogToClipboard(
    String(elements.observeRuntimeLogOutput?.textContent || "").trim(),
  );
}

async function refreshDesktopStatusOutput() {
  await refreshDesktopStatusPanel({
    invokeTauri,
    platform: elements.releasePlatform?.value || state.hostPlatform,
    setDesktopStatusOutput,
    formatHubOperatorError,
  });
}

async function runAction(action) {
  return runActionWithOptions(action, {});
}

function confirmHubDesktopAction(action) {
  const risk = HUB_DIRECT_ACTION_RISK[action] || "low";
  if (risk === "low") {
    return true;
  }

  const message =
    risk === "high"
      ? `High-risk desktop action: ${action}\n\nThis can build packages or rewrite bundle outputs.\n\nContinue?`
      : `Sensitive desktop action: ${action}\n\nPlease confirm before the Hub continues.\n\nContinue?`;
  return window.confirm(message);
}

async function runActionWithOptions(action, options = {}) {
  if (state.isBusy) {
    return;
  }

  if (!options.skipConfirmation && !confirmHubDesktopAction(action)) {
    setOperationOutput(`cancelled desktop action: ${action}`);
    applyDesktopState(elements.actionState, "cancelled", { kind: "activity" });
    return;
  }

  setBusy(true, "running");

  try {
    if (await runHubProjectAction(action, buildHubProjectActionContext({
      invokeTauri,
      setOperationOutput,
      setSection,
      setProjectsPage,
      setBusy,
      runProjectBundleAction,
      currentProjectBundlePayload,
      currentProjectBundleOutputPayload,
      currentProjectBundleComparePayload,
      setProjectBundleOutput,
    }))) {
      return;
    }

    if (await runHubRuntimeAction(action, buildHubRuntimeActionContext({
      invokeGuardedMutation,
      setOperationOutput,
      refreshRuntimeStatus,
      refreshHotRuntimeStatus,
      refreshHotRuntimeLog,
      refreshObserveRuntimeLog,
      copyHotRuntimeLogView,
      copyObserveRuntimeLogView,
      clearHotRuntimeLogView,
      currentHotRuntimeLogService,
      currentObserveRuntimeLogService,
      hubDynamic,
      setBusy,
    }))) {
      return;
    }

    if (await runHubWorkloadAction(action, buildHubWorkloadActionContext({
      registerCurrentBundleAsWorkload,
      syncLocalControlPlaneWorkloads,
      syncRemoteWorkloadCatalog,
      exportHubWorkloadLibrary,
      clearHubWorkloadLibrary,
      fetchWorkflowCatalog,
      workloadImportInput: elements.workloadImportInput,
      setBusy,
    }))) {
      return;
    }

    if (await runHubDesktopAction(action, buildHubDesktopActionContext({
      invokeTauri,
      setOperationOutput,
      setSection,
      setBusy,
      refreshDesktopStatusOutput,
      hubDynamic,
    }))) {
      return;
    }

    switch (action) {
      default:
        setBusy(false, "idle");
        return;
    }
  } catch (error) {
    setOperationOutput(formatHubOperatorError(error, {
      actionLabel: "This desktop action",
    }));
    setBusy(false, "failed");
  }
}

elements.navItems.forEach((item) => {
  item.addEventListener("click", () => setSection(item.dataset.target));
});

elements.projectsPageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setProjectsPage(button.dataset.projectsPage || "start");
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(hubDynamic("focusedHomePage", { page: button.dataset.projectsPage || "start" }));
  });
});

elements.projectsTargetButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setProjectsPage(button.dataset.projectsTarget || "start");
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(hubDynamic("openedHomeTarget", { page: button.dataset.projectsTarget || "start" }));
  });
});

elements.panelPageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    const group = button.dataset.panelPageGroup || "";
    const page = button.dataset.panelPage || "";
    setPanelPage(group, page);
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(hubDynamic("focusedPanelPage", { page, group }));
  });
});

elements.assistantFab?.addEventListener("click", () => {
  setAssistantPanelOpen(!state.assistantOpen);
  applyDesktopState(elements.actionState, "active", { kind: "activity" });
  setOperationOutput(state.assistantOpen ? hubDynamic("assistantPanelOpened") : hubDynamic("assistantPanelClosed"));
});

elements.assistantClose?.addEventListener("click", () => {
  setAssistantPanelOpen(false);
  applyDesktopState(elements.actionState, "idle", { kind: "activity" });
  setOperationOutput(hubDynamic("assistantPanelClosed"));
});

elements.assistantPanel?.addEventListener("click", (event) => {
  if (event.target !== elements.assistantPanel) {
    return;
  }
  setAssistantPanelOpen(false);
  applyDesktopState(elements.actionState, "idle", { kind: "activity" });
  setOperationOutput(hubDynamic("assistantPanelClosed"));
});

elements.assistantLocalAsk?.addEventListener("click", () => {
  answerWithLocalGuide();
});

elements.assistantLocalPrompt?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" || event.shiftKey) {
    return;
  }
  event.preventDefault();
  answerWithLocalGuide();
});

for (const button of document.querySelectorAll("[data-action]")) {
  button.addEventListener("click", async () => {
    await runAction(button.dataset.action);
  });
}

elements.sectionJumpButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setSection(button.dataset.targetSection);
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`focused ${button.dataset.targetSection} section`);
  });
});

elements.assistantModeButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setAssistantMode(button.dataset.assistantMode || "local");
  });
});

elements.assistantModelPreset?.addEventListener("change", () => {
  const preset = elements.assistantModelPreset.value;
  if (preset !== "custom" && elements.assistantModelName) {
    elements.assistantModelName.value = preset;
  }
  syncAssistantSettingsFromInputs();
});

elements.assistantBaseUrl?.addEventListener("change", () => {
  syncAssistantSettingsFromInputs();
  updateAssistantEndpointPolicy();
});
elements.assistantBaseUrl?.addEventListener("input", updateAssistantEndpointPolicy);
elements.assistantApiKey?.addEventListener("change", syncAssistantSettingsFromInputs);
elements.assistantModelName?.addEventListener("change", syncAssistantSettingsFromInputs);
elements.releasePlatform?.addEventListener("change", () => {
  renderToolsPlatformLabel();
  void refreshDesktopStatusOutput();
});
elements.hotRuntimeLogService?.addEventListener("change", () => {
  persistCurrentHotLogSettings();
  renderHotRuntimeLogServiceLabel();
  void refreshHotRuntimeLog();
});
elements.hotRuntimeLogAuto?.addEventListener("change", () => {
  persistCurrentHotLogSettings();
  syncHotRuntimeLogPolling();
});
elements.hotRuntimeLogInterval?.addEventListener("change", () => {
  persistCurrentHotLogSettings();
  stopHotRuntimeLogPolling();
  syncHotRuntimeLogPolling();
});
elements.observeRuntimeLogService?.addEventListener("change", () => {
  persistCurrentObserveRuntimeLogSettings();
  void refreshObserveRuntimeLog();
});
elements.observeRuntimeLogAuto?.addEventListener("change", () => {
  persistCurrentObserveRuntimeLogSettings();
  syncObserveRuntimeLogPolling();
});
elements.projectBundlePath?.addEventListener("input", () => {
  renderAssistantContext();
  renderHubAssistantLocalCards();
});
elements.projectBundleComparePath?.addEventListener("input", () => {
  renderAssistantContext();
  renderHubAssistantLocalCards();
});
elements.projectBundleOutPath?.addEventListener("input", () => {
  renderAssistantContext();
  renderHubAssistantLocalCards();
});

elements.assistantRequestPlan?.addEventListener("click", async () => {
  try {
    elements.assistantRequestPlan.disabled = true;
    setAssistantOutput("Planning...");
    syncAssistantSettingsFromInputs();
    state.assistantPlan = await requestHubAssistantPlan();
    elements.assistantApprovePlan.checked = false;
    renderHubAssistantPlan();
    setAssistantOutput(state.assistantPlan.summary || "Generated a Hub assistant plan.");
  } catch (error) {
    setAssistantOutput(formatHubOperatorError(error, {
      actionLabel: "The assistant request",
    }));
  } finally {
    elements.assistantRequestPlan.disabled = false;
  }
});

elements.assistantExecutePlan?.addEventListener("click", async () => {
  try {
    elements.assistantExecutePlan.disabled = true;
    await executeHubAssistantPlan();
  } catch (error) {
    setAssistantOutput(formatHubOperatorError(error, {
      actionLabel: "The assistant plan",
    }));
  } finally {
    elements.assistantExecutePlan.disabled = false;
  }
});

bindHubRecentActionControls({
  elements,
  state,
  renderHubRecents,
  setProjectBundleOutput,
  loadHubRecents,
  saveHubRecents,
  mergeProjectActionHistory,
  formatHubOperatorError,
});

bindHubLibraryControls({
  elements,
  state,
  renderHubWorkloadLibrary,
  setWorkloadLibraryOutput,
  renderWorkflowCatalog,
  setWorkflowCatalogOutput,
  localizedWorkflowCatalogLabel,
});

elements.densityToggleButtons.forEach((button) => {
  button.addEventListener("click", () => {
    toggleHubDensityPanel(button.dataset.densityToggle || "");
  });
});

elements.workloadImportInput?.addEventListener("change", async (event) => {
  const input = event.currentTarget;
  const file = input?.files?.[0];

  try {
    await importHubWorkloadLibrary(file);
  } catch (error) {
    setWorkloadLibraryOutput(formatHubOperatorError(error, {
      actionLabel: "Importing the workload library",
    }));
  } finally {
    if (input) {
      input.value = "";
    }
  }
});

bindHubLocalizationPanel({
  elements,
  hubCopy,
  rerenderLocalizedHubShell,
  setOperationOutput,
});

elements.languageSelect?.addEventListener("change", async (event) => {
  state.language = await saveDesktopLanguagePreference(normalizeDesktopLanguage(event.target.value));
  rerenderLocalizedHubShell();
  renderToolsPlatformLabel();
  window.requestAnimationFrame(() => {
    rerenderLocalizedHubShell();
    renderToolsPlatformLabel();
  });
});

state.language = await loadDesktopLanguagePreference();
rerenderLocalizedHubShell();
await applyBrand();
await loadEnvironment();
await loadDirectMeshRegressionSnapshot();
await loadRegressionGateReport();
enhanceHubAccessibility();
state.density = loadHubDensitySettings();
const hotLogSettings = loadHubHotLogSettings();
const runtimeLogSettings = loadHubRuntimeLogSettings();
if (elements.hotRuntimeLogService) {
  elements.hotRuntimeLogService.value = hotLogSettings.service;
}
if (elements.hotRuntimeLogAuto) {
  elements.hotRuntimeLogAuto.checked = hotLogSettings.autoRefresh;
}
if (elements.hotRuntimeLogInterval) {
  elements.hotRuntimeLogInterval.value = hotLogSettings.interval;
}
if (elements.observeRuntimeLogService) {
  elements.observeRuntimeLogService.value = runtimeLogSettings.service;
}
if (elements.observeRuntimeLogAuto) {
  elements.observeRuntimeLogAuto.checked = runtimeLogSettings.autoRefresh;
}
renderHotRuntimeLogServiceLabel();
syncDesktopStates();
renderHubDensityToggles();
renderPanelPages("runtimes");
renderPanelPages("deploy");
renderPanelPages("observe");
renderPanelPages("tools");
renderHubRecents();
applyAssistantSettings();
renderAssistantPanel();
rerenderLocalizedHubShell();
syncDesktopStates();
setSection(state.activeSection);
setBusy(false, "idle");
await refreshRuntimeStatus();
await refreshHotRuntimeStatus();
await refreshDesktopStatusOutput();
await fetchWorkflowCatalog({ silent: true });
rerenderLocalizedHubShell();
