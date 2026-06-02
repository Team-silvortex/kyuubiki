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
          copy: "Use the same panel for validation, doctor passes, and package staging so preflight work is easy to repeat.",
          validate: "Validate env",
          stage: "Prepare manifests",
          doctor: "Run doctor",
        },
        release: {
          label: "Suggested deploy path",
          title: "Mode, verify, package",
          copy: "Switch posture first, validate next, then move into desktop packaging and verification from the Tools section.",
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
          { label: "Diagnostics", title: "Read the platform first", copy: "Desktop readiness, benchmarks, and bundle validation sit next to packaging so the release story stays connected." },
          { label: "Packaging", title: "One platform selector, one path", copy: "Use one target selector for status, staging, host builds, and verification instead of jumping across disconnected surfaces." },
          { label: "Operational outputs", title: "Keep logs and results nearby", copy: "Status and operation output stay together so packaging work is easier to audit while you iterate." },
        ],
        packages: {
          label: "Platform actions",
          title: "Diagnostics and packaging",
          copy: "Use the same tool surface for local diagnostics, bundle validation, and platform-specific desktop packaging.",
          platform: "Target platform",
          benchmark: "Run benchmark",
          validate: "Validate project bundle",
          export: "Export database",
          status: "Desktop status",
          stage: "Stage desktop package",
          build: "Build host bundles",
          verify: "Verify desktop release",
          stop: "Stop local stack",
        },
        status: {
          label: "Readiness",
          title: "Desktop status",
          copy: "Use this output as the short readiness wall before you stage, build, or verify host bundles.",
        },
        output: {
          label: "Operations",
          title: "Tool output",
          copy: "Keep packaging and diagnostics output close by so release work remains inspectable while it runs.",
        },
      },
    },
    library: {
      introLabel: "Managed intake",
      introTitle: "Workload library",
      introCopy: "Keep downloaded bundles, standalone imports, and future server-delivered workloads in one Hub-managed library.",
      catalogUrl: "Catalog URL",
      labelOrNote: "Label or note",
      register: "Register current bundle",
      syncLocal: "Sync local control plane",
      syncRemote: "Sync remote catalog",
      export: "Export library JSON",
      import: "Import library JSON",
      clear: "Clear library",
      managedWorkloads: "Managed workloads",
      all: "All",
      mechanical: "Mechanical",
      thermal: "Thermal",
      thermo: "Thermo-mechanical",
      allFamilies: "All families",
      axial: "Axial & Springs",
      beams: "Beams & Frames",
      trusses: "Trusses",
      planes: "Planes",
      workflowCatalogLabel: "Workflow catalog",
      workflowCatalogTitle: "Named workflow runs",
      workflowCatalogCopy: "Discover built-in multi-operator workflows and run a reference sample without pasting a raw graph.",
      workflowCatalogRefresh: "Refresh workflow catalog",
      workflowCatalogReady: "Workflow catalog is ready.",
      ready: "Workload library is ready.",
    },
    bundles: {
      introLabel: "Bundle operations",
      introTitle: "Project bundle tools",
      introCopy: "Keep the repetitive archive work in one place, then move straight into analysis.",
      bundlePath: "Bundle path",
      comparePath: "Compare path",
      outputPath: "Output path",
      inspect: "Inspect .kyuubiki",
      validate: "Validate .kyuubiki",
      normalize: "Normalize bundle",
      unpack: "Unpack bundle",
      pack: "Pack project",
      diff: "Diff bundles",
      openWorkbench: "Open workbench",
      desktopTools: "Desktop tools",
      recentBundles: "Recent bundles",
      recentCompare: "Recent compare paths",
      recentOutputs: "Recent outputs",
      recentActions: "Recent bundle actions",
      all: "All",
      failed: "Failed",
      keepFailed: "Keep failed only",
      import: "Import JSON",
      export: "Export JSON",
      clear: "Clear history",
      favorites: "Favorites",
      recent: "Recent",
      ready: "Project bundle tools are ready.",
    },
    guides: {
      primaryLabel: "Primary docs",
      primaryTitle: "Open the right guide",
      primaryCopy: "Start with the docs index, then branch into only the guide that matches the job you are doing now.",
      docsTitle: "Docs index",
      docsCopy: "The single entry to current-line, operations, testing, accuracy, and archived release notes.",
      currentTitle: "Current line",
      currentCopy: "Read what tamamono 1.x is optimizing for before you make deeper product decisions.",
      overviewDocsLabel: "Docs hub",
      overviewDocsTitle: "One readable shelf",
      overviewDocsCopy: "Use one place for orientation first, then branch into operations, accuracy, or troubleshooting only when needed.",
      overviewCurrentLabel: "Current line",
      overviewCurrentTitle: "tamamono 1.x",
      overviewCurrentCopy: "Read the current product posture, version line, and what this generation is trying to harden.",
      overviewTroubleshootingLabel: "Troubleshooting",
      overviewTroubleshootingTitle: "Find the shortest path",
      overviewTroubleshootingCopy: "Use the first-line support notes before you dive into deeper runtime or packaging details.",
      operationsTitle: "Operations",
      operationsCopy: "Use this when you need the runtime, stack, or operator path explained as a coherent workflow.",
      troubleshootingTitle: "Troubleshooting",
      troubleshootingCopy: "Use the shortest failure path before you dig into full logs or packaging output.",
      accuracyLabel: "Accuracy and confidence",
      accuracyTitle: "Read the trust story",
      accuracyCopy: "These are the documents that explain what the current line is trying to verify and why that matters before moxi.",
      accuracyPlanTitle: "Accuracy plan",
      accuracyPlanCopy: "See the long-line plan for verified baselines, benchmark expansion, and solver trust.",
      accuracyBaselinesTitle: "Accuracy baselines",
      accuracyBaselinesCopy: "Read which benchmark families are already locked into regression and which are still next.",
    },
    assistant: {
      introLabel: "Need help?",
      introTitle: "Pick the next safe step",
      introCopy: "Start with the local guide. Reach for a model only when the built-in path is not enough.",
      close: "Close",
      engine: "Engine",
      localMode: "Local guide",
      llmMode: "Model assist",
      section: "Section",
      runtime: "Runtime",
      bundle: "Bundle",
      quickActions: "Quick actions",
      quickStart: "Start local stack",
      quickLibrary: "Open library",
      quickBundles: "Inspect bundle",
      quickGuides: "Open guides",
      ask: "Ask",
      askLabel: "Ask the local guide",
      askButton: "Ask local guide",
      askEmpty: "Ask about the next step, bundle inspection, runtime health, documentation, or packaging.",
      docs: "Docs & guides",
      docsIndexTitle: "Docs index",
      docsIndexCopy: "Open the full documentation entry point.",
      docsCurrentTitle: "Current line",
      docsCurrentCopy: "Read the current tamamono 1.x posture.",
      docsOperationsTitle: "Operations",
      docsOperationsCopy: "Read the runtime and operator workflow.",
      docsTroubleshootingTitle: "Troubleshooting",
      docsTroubleshootingCopy: "Jump to the first-line support notes.",
      suggested: "Suggested next steps",
      llmIntro: "Connect an OpenAI-compatible model only when you want a longer onboarding or operations plan.",
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
      assistantPromptEmpty: "Ask something short, like: what should I do first, how do I inspect a bundle, how do I open Workbench, or why is packaging still partial.",
      assistantNoUrgent: "The local guide does not see an urgent next step right now.",
      assistantNoPlan: "No model plan yet.",
      assistantNoExecutable: "The model returned no executable Hub actions.",
      assistantCancelled: "Cancelled {action}.",
      assistantFocusedSection: "Focused {section} section.",
      assistantUpdatedBundle: "Updated bundle context in the Hub.",
      assistantExecuteCount: "Executed {count} assistant actions.",
      assistantNoPlanToExecute: "No assistant plan is available to execute.",
      assistantReviewFirst: "Review the generated plan and confirm execution first.",
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
      endpointPolicyDefault: "Use https:// for remote providers, or http://localhost / 127.0.0.1 for local gateways. The API key is sent directly to the configured base URL.",
      endpointPolicyAllowed: "Assistant endpoint looks allowed. The API key is sent directly to the configured base URL for plan generation.",
      guideFirstNoRuntime: "Start with the local stack, then sync or register work, inspect once, and only then open Workbench. Right now the local runtime does not look ready, so `Start local stack` is the safest first move.",
      guideFirstNoBundle: "Start with the local stack if needed, then open `Bundle tools` and paste a `.kyuubiki` path. After that, inspect once and move into Workbench.",
      guideFirstReady: "You already have a runtime and bundle context. The safe path now is: inspect the current bundle, confirm the result looks sane, then open Workbench.",
      guideBundleNoPath: "Bundle operations live under `Home > Bundle tools`. Paste a `.kyuubiki` bundle path first. Then use `Inspect` for a quick read, `Validate` for schema checks, and `Normalize` only when you also have an output path.",
      guideNormalizeNoOutput: "Normalization needs both a bundle path and an output path. You already have the bundle, so the missing piece is the output destination in `Bundle tools`.",
      guideDiffNoCompare: "Bundle diff needs both the current bundle and a compare path. Fill the compare field in `Bundle tools`, then run `Diff bundles`.",
      guideBundleGeneral: "Use `Inspect` first for a safe structural read. Use `Validate` when you want schema confidence, `Normalize` when you want a cleaned output bundle, and `Diff` only after both bundle paths are filled.",
      guideWorkbench: "Open Workbench only after the runtime is healthy and the bundle context looks sane. In Hub, the short path is: `Home > Start here`, then `Open workbench`.",
      guideDocs: "Use `Home > Guides` as the single documentation shelf. Start with `Docs index`, then open `Current line`, `Operations`, or `Troubleshooting` only when you know which kind of question you are answering.",
      guideRuntime: "Use `Runtimes` when you want to change the loop, and `Observe` when you only want to scan or copy state. `Local runtime` is the short health read, `Hot loop` is for dev tails, and `Stack watch` is for sanitized runtime logs.",
      guideLibrary: "Use `Home > Library` for workload intake. `Sync local control plane` pulls first-party work in, `Sync remote catalog` brings remote entries in, and the domain/family filters help you narrow the shelf before opening Workbench.",
      guidePackaging: "Use `Installer` when you need release layout or workstation bootstrap. In Hub, `Tools > Packages` is for build actions, `Status` is the readiness wall, and `Output` is where the packaging logs land. In this automation session, `.app` bundles are reliable, while `.dmg` can still show as partial because `hdiutil` is session-sensitive.",
      guideFailure: "If something looks partial, read the shortest surface first: `Observe > Health` for runtime issues, `Tools > Status` for desktop packaging readiness, and `Bundle tools > Inspect` for project bundle shape. Then decide whether the problem is runtime, bundle, or packaging.",
      guideFallback: "The local guide can help with first steps, bundle inspection, runtime health, Workbench launch, workload library intake, and desktop packaging. Try asking one of those directly.",
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
      managedWorkloadsFilterEmpty: "No workloads match {domain} / {family}.",
      workflowCatalogEmpty: "No named workflows are available yet.",
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
          copy: "把验证、doctor 检查和包 staging 放在同一个面板里，让 preflight 工作更容易重复。",
          validate: "验证环境",
          stage: "准备 manifest",
          doctor: "运行 doctor",
        },
        release: {
          label: "推荐部署路径",
          title: "模式、验证、打包",
          copy: "先切运行姿态，再验证，然后从 Tools 进入桌面打包和校验。",
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
          { label: "诊断", title: "先读平台状态", copy: "桌面就绪度、benchmark 和 bundle 验证放在打包旁边，让发布故事保持连贯。" },
          { label: "打包", title: "一个平台选择器，一条路径", copy: "状态、staging、host build 和 verify 共用同一套目标选择，不再在断裂的面板间跳转。" },
          { label: "操作输出", title: "让日志和结果留在手边", copy: "状态和操作输出放在一起，让打包工作在迭代时更容易审阅。" },
        ],
        packages: {
          label: "平台动作",
          title: "诊断与打包",
          copy: "把本地诊断、bundle 验证和按平台桌面打包放在同一个工具面里。",
          platform: "目标平台",
          benchmark: "运行 benchmark",
          validate: "验证项目 bundle",
          export: "导出数据库",
          status: "桌面状态",
          stage: "staging 桌面包",
          build: "构建 host bundles",
          verify: "验证桌面发布",
          stop: "停止本地栈",
        },
        status: {
          label: "就绪度",
          title: "桌面状态",
          copy: "把这份输出当成短就绪墙，再决定是否做 stage、build 或 verify。",
        },
        output: {
          label: "操作输出",
          title: "工具输出",
          copy: "让打包和诊断输出留在手边，这样发布工作在运行时也可检查。",
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
    managedWorkloads: "已管理工作负载",
    all: "全部",
    mechanical: "力学",
    thermal: "热",
    thermo: "力热",
    allFamilies: "全部 family",
    axial: "轴向与弹簧",
    beams: "梁与刚架",
    trusses: "桁架",
    planes: "平面",
    workflowCatalogLabel: "工作流目录",
    workflowCatalogTitle: "命名工作流运行",
    workflowCatalogCopy: "发现内建多算子工作流，并直接运行参考样板，不必手填 raw graph。",
    workflowCatalogRefresh: "刷新工作流目录",
    workflowCatalogReady: "工作流目录已就绪。",
    ready: "工作负载库已就绪。",
  },
    bundles: {
      introLabel: "Bundle 操作",
      introTitle: "项目 bundle 工具",
      introCopy: "把重复的归档操作收在一个地方，然后直接继续分析。",
      bundlePath: "Bundle 路径",
      comparePath: "对比路径",
      outputPath: "输出路径",
      inspect: "检查 .kyuubiki",
      validate: "验证 .kyuubiki",
      normalize: "规范化 bundle",
      unpack: "解包 bundle",
      pack: "打包项目",
      diff: "对比 bundle",
      openWorkbench: "打开 Workbench",
      desktopTools: "桌面工具",
      recentBundles: "最近 bundles",
      recentCompare: "最近对比路径",
      recentOutputs: "最近输出",
      recentActions: "最近 bundle 动作",
      all: "全部",
      failed: "失败",
      keepFailed: "只保留失败项",
      import: "导入 JSON",
      export: "导出 JSON",
      clear: "清空历史",
      favorites: "收藏",
      recent: "最近",
      ready: "项目 bundle 工具已就绪。",
    },
    guides: {
      primaryLabel: "主文档",
      primaryTitle: "打开正确的指南",
      primaryCopy: "先从 docs index 开始，再只进入和当前工作匹配的那份指南。",
      docsTitle: "文档索引",
      docsCopy: "current-line、operations、testing、accuracy 和历史 release notes 的统一入口。",
      currentTitle: "当前版本线",
      currentCopy: "先读 tamamono 1.x 现在到底在强化什么，再做更深的产品判断。",
      overviewDocsLabel: "文档中枢",
      overviewDocsTitle: "一个清晰的文档架",
      overviewDocsCopy: "先在一个地方完成定向，再按需要进入 operations、accuracy 或 troubleshooting。",
      overviewCurrentLabel: "当前版本线",
      overviewCurrentTitle: "tamamono 1.x",
      overviewCurrentCopy: "先读当前产品姿态、版本线和这一代在重点加固什么。",
      overviewTroubleshootingLabel: "故障排查",
      overviewTroubleshootingTitle: "找到最短路径",
      overviewTroubleshootingCopy: "先走第一线支持说明，再决定是否深入 runtime 或打包细节。",
      operationsTitle: "Operations",
      operationsCopy: "当你需要把 runtime、stack 或 operator 路径看成一条完整工作流时，用这份文档。",
      troubleshootingTitle: "故障排查",
      troubleshootingCopy: "先走最短的失败路径，再决定是否深挖完整日志或打包输出。",
      accuracyLabel: "精度与可信度",
      accuracyTitle: "阅读信任路径",
      accuracyCopy: "这些文档解释当前版本线正在验证什么，以及为什么这在 moxi 之前很重要。",
      accuracyPlanTitle: "精度计划",
      accuracyPlanCopy: "看 verified baselines、benchmark 扩展和 solver 信任链的长期计划。",
      accuracyBaselinesTitle: "精度基线",
      accuracyBaselinesCopy: "查看哪些 benchmark family 已经锁进回归，哪些还在下一步。",
    },
    assistant: {
      introLabel: "需要帮助？",
      introTitle: "选择更安全的下一步",
      introCopy: "先用本地向导。只有当内建路径不够时，再接模型。",
      close: "关闭",
      engine: "引擎",
      localMode: "本地向导",
      llmMode: "模型助手",
      section: "页面",
      runtime: "运行时",
      bundle: "Bundle",
      quickActions: "快捷动作",
      quickStart: "启动本地栈",
      quickLibrary: "打开库",
      quickBundles: "检查 bundle",
      quickGuides: "打开指南",
      ask: "提问",
      askLabel: "询问本地向导",
      askButton: "询问本地向导",
      askEmpty: "可以问下一步该做什么、如何检查 bundle、运行时健康、文档入口或打包路径。",
      docs: "文档与指南",
      docsIndexTitle: "文档索引",
      docsIndexCopy: "打开完整的文档总入口。",
      docsCurrentTitle: "当前版本线",
      docsCurrentCopy: "查看 tamamono 1.x 当前的产品姿态。",
      docsOperationsTitle: "Operations",
      docsOperationsCopy: "查看运行时和 operator 工作流。",
      docsTroubleshootingTitle: "故障排查",
      docsTroubleshootingCopy: "跳到第一线支持说明。",
      suggested: "建议的下一步",
      llmIntro: "只有在你想要更长的 onboarding 或运维计划时，才连接 OpenAI-compatible 模型。",
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
      assistantPromptEmpty: "可以直接问：第一步该做什么、如何检查 bundle、怎么打开 Workbench，或者为什么打包还是 partial。",
      assistantNoUrgent: "本地向导暂时没有看到特别紧急的下一步。",
      assistantNoPlan: "还没有模型计划。",
      assistantNoExecutable: "模型没有返回可执行的 Hub 动作。",
      assistantCancelled: "已取消 {action}。",
      assistantFocusedSection: "已聚焦到 {section} 页面。",
      assistantUpdatedBundle: "已更新 Hub 里的 bundle 上下文。",
      assistantExecuteCount: "已执行 {count} 个助手动作。",
      assistantNoPlanToExecute: "当前没有可执行的助手计划。",
      assistantReviewFirst: "请先阅读生成的计划并确认执行。",
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
      endpointPolicyDefault: "远程提供方请使用 https://，本地网关可使用 http://localhost / 127.0.0.1。API key 会直接发送到配置的 Base URL。",
      endpointPolicyAllowed: "当前助手端点看起来可用。API key 会直接发送到配置的 Base URL 以生成计划。",
      guideFirstNoRuntime: "先启动本地栈，再同步或注册工作，检查一次，然后再打开 Workbench。当前本地运行时看起来还没准备好，所以 `启动本地栈` 是最安全的第一步。",
      guideFirstNoBundle: "如果需要，先启动本地栈，然后打开 `Bundle 工具`，填入 `.kyuubiki` 路径。之后先检查一次，再进入 Workbench。",
      guideFirstReady: "你现在已经有运行时和 bundle 上下文了。更安全的下一步是：先检查当前 bundle，确认结果正常，再打开 Workbench。",
      guideBundleNoPath: "Bundle 相关操作都在 `首页 > Bundle 工具`。先填入 `.kyuubiki` 路径。然后用 `检查` 做快速结构读取，用 `验证` 做 schema 检查；只有填了输出路径时再做 `规范化`。",
      guideNormalizeNoOutput: "规范化需要同时提供 bundle 路径和输出路径。你现在已经有 bundle 了，缺的是 `Bundle 工具` 里的输出目标。",
      guideDiffNoCompare: "Bundle 对比需要当前 bundle 和 compare path。先在 `Bundle 工具` 里填 compare path，然后运行 `对比 bundle`。",
      guideBundleGeneral: "更稳的顺序是先 `检查` 做结构读取，再用 `验证` 看 schema，只有在两边路径都填好后再 `规范化` 或 `对比`。",
      guideWorkbench: "只有在运行时健康、bundle 上下文也看起来正常时，再打开 Workbench。Hub 里的短路径是：`首页 > 从这里开始`，然后点 `打开 Workbench`。",
      guideDocs: "把 `首页 > 文档` 当成单一文档入口。先看 `文档索引`，只有在明确问题类型后，再进入 `当前版本线`、`Operations` 或 `故障排查`。",
      guideRuntime: "当你需要改变 loop 时去 `运行时`；当你只是想扫一眼或复制状态时去 `观察`。`本地运行时` 负责短健康读，`Hot loop` 看开发 tail，`Stack watch` 看脱敏后的运行时日志。",
      guideLibrary: "工作负载入口在 `首页 > 库`。`同步本地控制平面` 会拉入第一方工作，`同步远程目录` 会带入远端条目，domain/family 筛选则帮助你在进入 Workbench 前先把 shelf 收窄。",
      guidePackaging: "当你需要发布布局或工作站 bootstrap 时去 `Installer`。在 Hub 里，`工具 > Packages` 用来执行构建动作，`Status` 是 readiness 墙，`Output` 看打包输出。在当前自动化会话里，`.app` 一般可靠，而 `.dmg` 仍可能因为 `hdiutil` 的会话敏感性而显示 partial。",
      guideFailure: "如果某件事看起来是 partial，先走最短路径：`观察 > 健康` 看运行时问题，`工具 > Status` 看桌面打包就绪度，`Bundle 工具 > 检查` 看项目 bundle 结构。然后再判断问题属于 runtime、bundle 还是 packaging。",
      guideFallback: "本地向导可以帮你处理第一步、bundle 检查、运行时健康、Workbench 打开、工作负载导入和桌面打包这些问题。可以直接围绕这些提问。",
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
      managedWorkloadsFilterEmpty: "没有工作负载匹配 {domain} / {family}。",
      workflowCatalogEmpty: "当前还没有可用的命名工作流。",
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
          copy: "検証、doctor、パッケージ staging を同じ面に集め、preflight を繰り返しやすくします。",
          validate: "環境を確認",
          stage: "manifest を準備",
          doctor: "doctor を実行",
        },
        release: {
          label: "推奨デプロイ経路",
          title: "モード、検証、パッケージ",
          copy: "まず姿勢を切り替え、次に確認し、その後 Tools からデスクトップ packaging と verification に進みます。",
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
          { label: "診断", title: "まずプラットフォームを読む", copy: "desktop readiness、benchmark、bundle validation を packaging の横に置き、リリースの流れをつなげます。" },
          { label: "パッケージング", title: "一つの platform selector、一つの経路", copy: "status、staging、host build、verify を一つの target selector で進め、断片化した面を行き来しません。" },
          { label: "操作出力", title: "ログと結果を近くに置く", copy: "status と operation output を近くに置き、パッケージ作業を監査しやすくします。" },
        ],
        packages: {
          label: "プラットフォーム操作",
          title: "診断とパッケージング",
          copy: "ローカル診断、bundle validation、プラットフォーム別の desktop packaging を同じツール面で扱います。",
          platform: "対象プラットフォーム",
          benchmark: "benchmark を実行",
          validate: "project bundle を検証",
          export: "データベースを出力",
          status: "desktop status",
          stage: "desktop package を stage",
          build: "host bundles を build",
          verify: "desktop release を verify",
          stop: "ローカル stack を停止",
        },
        status: {
          label: "準備状況",
          title: "Desktop status",
          copy: "stage、build、verify の前に、この出力を短い readiness wall として使います。",
        },
        output: {
          label: "操作出力",
          title: "Tool output",
          copy: "packaging と diagnostics の出力を近くに置き、リリース作業を実行中でも確認できるようにします。",
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
      managedWorkloads: "管理された workloads",
      all: "すべて",
      mechanical: "力学",
      thermal: "熱",
      thermo: "熱・構造",
      allFamilies: "全 family",
      axial: "軸・ばね",
      beams: "梁・フレーム",
      trusses: "トラス",
      planes: "平面",
      workflowCatalogLabel: "workflow catalog",
      workflowCatalogTitle: "名前付き workflow 実行",
      workflowCatalogCopy: "raw graph を貼り付けずに、組み込み multi-operator workflow を見つけて reference sample を実行します。",
      workflowCatalogRefresh: "workflow catalog を更新",
      workflowCatalogReady: "workflow catalog の準備ができました。",
      ready: "ワークロードライブラリの準備ができました。",
    },
    bundles: {
      introLabel: "Bundle 操作",
      introTitle: "プロジェクト bundle ツール",
      introCopy: "繰り返しのアーカイブ作業を一か所に集め、そのまま解析へ進みます。",
      bundlePath: "Bundle パス",
      comparePath: "比較パス",
      outputPath: "出力パス",
      inspect: ".kyuubiki を確認",
      validate: ".kyuubiki を検証",
      normalize: "bundle を正規化",
      unpack: "bundle を展開",
      pack: "project を pack",
      diff: "bundle を diff",
      openWorkbench: "Workbench を開く",
      desktopTools: "Desktop tools",
      recentBundles: "最近の bundles",
      recentCompare: "最近の比較パス",
      recentOutputs: "最近の出力",
      recentActions: "最近の bundle 操作",
      all: "すべて",
      failed: "失敗",
      keepFailed: "失敗だけ残す",
      import: "JSON を読み込み",
      export: "JSON を出力",
      clear: "履歴をクリア",
      favorites: "お気に入り",
      recent: "最近",
      ready: "プロジェクト bundle ツールの準備ができました。",
    },
    guides: {
      primaryLabel: "主要ドキュメント",
      primaryTitle: "正しいガイドを開く",
      primaryCopy: "まず docs index から入り、今の作業に合うガイドだけに進みます。",
      docsTitle: "Docs index",
      docsCopy: "current-line、operations、testing、accuracy、archive をまとめた入口です。",
      currentTitle: "Current line",
      currentCopy: "より深い判断の前に、tamamono 1.x が何を強化しているかを確認します。",
      overviewDocsLabel: "Docs hub",
      overviewDocsTitle: "読みやすい一つの棚",
      overviewDocsCopy: "まず一か所で向きを合わせ、その後必要に応じて operations、accuracy、troubleshooting に分岐します。",
      overviewCurrentLabel: "Current line",
      overviewCurrentTitle: "tamamono 1.x",
      overviewCurrentCopy: "現在のプロダクト姿勢、version line、この世代が何を硬くしているかを確認します。",
      overviewTroubleshootingLabel: "Troubleshooting",
      overviewTroubleshootingTitle: "最短経路を見つける",
      overviewTroubleshootingCopy: "より深い runtime や packaging の詳細に入る前に、第一線のサポートノートを使います。",
      operationsTitle: "Operations",
      operationsCopy: "runtime、stack、operator path を一つの流れとして理解したいときに使います。",
      troubleshootingTitle: "Troubleshooting",
      troubleshootingCopy: "完全なログや packaging 出力へ潜る前に、最短の失敗経路を使います。",
      accuracyLabel: "精度と信頼性",
      accuracyTitle: "信頼の筋道を読む",
      accuracyCopy: "これらの文書は、現在のバージョンラインが何を検証しようとしているのか、そしてそれが moxi の前に重要である理由を説明します。",
      accuracyPlanTitle: "Accuracy plan",
      accuracyPlanCopy: "verified baseline、benchmark 拡張、solver trust の長期計画を確認します。",
      accuracyBaselinesTitle: "Accuracy baselines",
      accuracyBaselinesCopy: "どの benchmark family が回帰に固定され、どれが次なのかを確認します。",
    },
    assistant: {
      introLabel: "困ったら",
      introTitle: "安全な次の一手を選ぶ",
      introCopy: "まずはローカルガイドを使います。組み込みの道筋で足りないときだけモデルを使います。",
      close: "閉じる",
      engine: "エンジン",
      localMode: "ローカルガイド",
      llmMode: "モデル補助",
      section: "セクション",
      runtime: "ランタイム",
      bundle: "Bundle",
      quickActions: "クイック操作",
      quickStart: "ローカルスタックを起動",
      quickLibrary: "ライブラリを開く",
      quickBundles: "bundle を確認",
      quickGuides: "ガイドを開く",
      ask: "質問",
      askLabel: "ローカルガイドに聞く",
      askButton: "ローカルガイドに聞く",
      askEmpty: "次の一手、bundle の確認、runtime の健康、ドキュメント、パッケージングについて聞けます。",
      docs: "Docs とガイド",
      docsIndexTitle: "Docs index",
      docsIndexCopy: "完全なドキュメント入口を開きます。",
      docsCurrentTitle: "Current line",
      docsCurrentCopy: "tamamono 1.x の現在の姿勢を読みます。",
      docsOperationsTitle: "Operations",
      docsOperationsCopy: "runtime と operator workflow を読みます。",
      docsTroubleshootingTitle: "Troubleshooting",
      docsTroubleshootingCopy: "第一線のサポートノートへ進みます。",
      suggested: "おすすめの次の一手",
      llmIntro: "より長い onboarding や運用計画が必要なときだけ OpenAI-compatible モデルを接続します。",
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
      assistantPromptEmpty: "短く聞けます。たとえば、最初に何をすべきか、bundle をどう確認するか、Workbench をどう開くか、なぜ packaging が partial のままなのか、などです。",
      assistantNoUrgent: "ローカルガイドは今のところ緊急の次の一手を見つけていません。",
      assistantNoPlan: "モデル計画はまだありません。",
      assistantNoExecutable: "モデルは実行可能な Hub アクションを返しませんでした。",
      assistantCancelled: "{action} を取り消しました。",
      assistantFocusedSection: "{section} セクションに移動しました。",
      assistantUpdatedBundle: "Hub 内の bundle コンテキストを更新しました。",
      assistantExecuteCount: "{count} 件のアシスタントアクションを実行しました。",
      assistantNoPlanToExecute: "実行できるアシスタント計画がありません。",
      assistantReviewFirst: "まず生成された計画を確認してから実行してください。",
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
      endpointPolicyDefault: "リモートプロバイダーには https:// を使い、ローカルゲートウェイには http://localhost / 127.0.0.1 を使ってください。API key は設定された Base URL に直接送信されます。",
      endpointPolicyAllowed: "現在のアシスタント endpoint は許可された形に見えます。API key は計画生成のために設定された Base URL に直接送信されます。",
      guideFirstNoRuntime: "まずローカルスタックを起動し、その後で作業を同期または登録し、一度確認してから Workbench を開いてください。今はローカルランタイムがまだ準備完了に見えないため、`ローカルスタックを起動` が最も安全な第一歩です。",
      guideFirstNoBundle: "必要なら先にローカルスタックを起動し、その後 `Bundle tools` を開いて `.kyuubiki` パスを貼り付けてください。そのあと一度確認してから Workbench に進みます。",
      guideFirstReady: "すでにランタイムと bundle コンテキストがあります。安全な次の一手は、現在の bundle を確認し、結果が妥当に見えることを確認してから Workbench を開くことです。",
      guideBundleNoPath: "bundle 操作は `ホーム > Bundle tools` にあります。まず `.kyuubiki` パスを入力してください。その後 `Inspect` で短い構造確認、`Validate` で schema 確認、出力先があるときだけ `Normalize` を使います。",
      guideNormalizeNoOutput: "Normalize には bundle パスと出力パスの両方が必要です。bundle はすでにあるので、足りないのは `Bundle tools` の出力先です。",
      guideDiffNoCompare: "bundle diff には現在の bundle と compare path の両方が必要です。`Bundle tools` の compare フィールドを埋めてから `Diff bundles` を実行してください。",
      guideBundleGeneral: "まず `Inspect` で安全に構造を読み、`Validate` で schema の確信を取り、両方のパスが埋まっているときだけ `Normalize` や `Diff` を使うのがよい流れです。",
      guideWorkbench: "Workbench はランタイムが健全で、bundle コンテキストも妥当に見えるときにだけ開いてください。Hub の短い流れは `ホーム > Start here` から `Open workbench` です。",
      guideDocs: "`ホーム > Guides` を単一のドキュメント棚として使ってください。まず `Docs index` を見て、質問の種類が分かってから `Current line`、`Operations`、`Troubleshooting` に進みます。",
      guideRuntime: "loop を変えたいなら `Runtimes`、状態を見たりコピーしたりするだけなら `Observe` を使います。`Local runtime` は短い health 読み、`Hot loop` は開発 tail、`Stack watch` はサニタイズ済み runtime ログです。",
      guideLibrary: "workload の取り込みは `ホーム > Library` です。`Sync local control plane` は第一者の作業を取り込み、`Sync remote catalog` はリモート項目を取り込み、domain/family filter で Workbench を開く前に棚を絞れます。",
      guidePackaging: "`Installer` は release layout や workstation bootstrap が必要なときに使います。Hub では `Tools > Packages` が build アクション、`Status` が readiness wall、`Output` が packaging ログです。この自動化セッションでは `.app` は信頼しやすい一方、`.dmg` は `hdiutil` のセッション依存で partial のままになることがあります。",
      guideFailure: "何かが partial に見えるなら、まず最短面を見てください。`Observe > Health` で runtime、`Tools > Status` で desktop packaging readiness、`Bundle tools > Inspect` で project bundle の形を見て、それが runtime・bundle・packaging のどれかを判断します。",
      guideFallback: "ローカルガイドは、最初の一歩、bundle 確認、runtime の健康、Workbench 起動、workload library の取り込み、desktop packaging について手伝えます。そのあたりを直接聞いてください。",
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
    managedWorkloads: "Cargas gestionadas",
    workflowCatalogLabel: "Catálogo de workflows",
    workflowCatalogTitle: "Ejecuciones con nombre",
    workflowCatalogCopy: "Descubre workflows integrados de varios operadores y ejecuta una muestra de referencia sin pegar un grafo crudo.",
    workflowCatalogRefresh: "Actualizar catálogo de workflows",
    workflowCatalogReady: "El catálogo de workflows está listo.",
  },
  bundles: {
    ...HUB_I18N.en.bundles,
    introLabel: "Herramientas de bundle",
    introTitle: "Trabajar con bundles",
    introCopy: "Inspecciona, valida y compara bundles de proyecto sin salir de Hub.",
    bundlePath: "Ruta del bundle",
    comparePath: "Ruta a comparar",
    outputPath: "Ruta de salida",
  },
  guides: {
    ...HUB_I18N.en.guides,
    primaryLabel: "Biblioteca principal",
    primaryTitle: "Centro de documentación",
    primaryCopy: "Usa una sola estantería clara para la línea actual, operaciones, troubleshooting y exactitud.",
    docsTitle: "Índice de docs",
    docsCopy: "Abre el índice principal de documentación.",
    currentTitle: "Línea actual",
    currentCopy: "Lee la postura actual de tamamono 1.x.",
  },
  assistant: {
    ...HUB_I18N.en.assistant,
    introLabel: "¿Necesitas ayuda?",
    introTitle: "Elige el siguiente paso seguro",
    introCopy: "Empieza con la guía local. Recurre a un modelo solo cuando la ruta integrada no sea suficiente.",
    close: "Cerrar",
    engine: "Motor",
    title: "Asistente",
    localGuide: "Guía local",
    modelAssist: "Asistencia con modelo",
    section: "Sección",
    runtime: "Runtime",
    bundle: "Bundle",
    quickActions: "Acciones rápidas",
    quickStart: "Iniciar pila local",
    quickLibrary: "Abrir biblioteca",
    quickBundles: "Inspeccionar bundle",
    quickGuides: "Abrir guías",
    ask: "Preguntar",
    askLocal: "Preguntar a la guía local",
    askLabel: "Pregunta a la guía local",
    askButton: "Preguntar a la guía local",
    askEmpty: "Pregunta por el siguiente paso, la inspección de bundles, la salud del runtime, la documentación o el empaquetado.",
    docsGuides: "Docs y guías",
    docsIndexTitle: "Índice de docs",
    docsIndexCopy: "Abre la entrada completa de documentación.",
    docsCurrentTitle: "Línea actual",
    docsCurrentCopy: "Lee la postura actual de tamamono 1.x.",
    docsOperationsTitle: "Operaciones",
    docsOperationsCopy: "Lee el flujo de runtime y operador.",
    docsTroubleshootingTitle: "Troubleshooting",
    docsTroubleshootingCopy: "Salta a las notas de soporte de primera línea.",
    suggested: "Siguientes pasos sugeridos",
    llmIntro: "Conecta un modelo compatible con OpenAI solo cuando quieras un plan de onboarding u operaciones más largo.",
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
    assistantPromptEmpty: "Pregunta algo corto, por ejemplo: qué debo hacer primero, cómo inspecciono un bundle, cómo abro Workbench o por qué el empaquetado sigue en partial.",
    assistantNoUrgent: "La guía local no ve un siguiente paso urgente ahora mismo.",
    assistantNoPlan: "Todavía no hay plan del modelo.",
    assistantNoExecutable: "El modelo no devolvió acciones ejecutables para Hub.",
    assistantCancelled: "Se canceló {action}.",
    assistantFocusedSection: "Se enfocó la sección {section}.",
    assistantUpdatedBundle: "Se actualizó el contexto del bundle en Hub.",
    assistantExecuteCount: "Se ejecutaron {count} acciones del asistente.",
    assistantNoPlanToExecute: "No hay un plan del asistente disponible para ejecutar.",
    assistantReviewFirst: "Revisa primero el plan generado y confirma la ejecución.",
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
    endpointPolicyDefault: "Usa https:// para proveedores remotos, o http://localhost / 127.0.0.1 para pasarelas locales. La API key se envía directamente a la URL base configurada.",
    endpointPolicyAllowed: "El endpoint del asistente parece permitido. La API key se envía directamente a la URL base configurada para generar el plan.",
    guideFirstNoRuntime: "Empieza por la pila local, luego sincroniza o registra trabajo, inspecciónalo una vez y solo entonces abre Workbench. Ahora mismo el runtime local no parece listo, así que `Iniciar pila local` es el primer paso más seguro.",
    guideFirstNoBundle: "Empieza por la pila local si hace falta, luego abre `Bundle tools` y pega una ruta `.kyuubiki`. Después inspecciona una vez y pasa a Workbench.",
    guideFirstReady: "Ya tienes runtime y contexto de bundle. La ruta segura ahora es: inspecciona el bundle actual, confirma que el resultado parece razonable y después abre Workbench.",
    guideBundleNoPath: "Las operaciones con bundles viven en `Inicio > Bundle tools`. Pega primero una ruta de bundle `.kyuubiki`. Luego usa `Inspect` para una lectura rápida, `Validate` para comprobaciones de esquema y `Normalize` solo cuando también tengas una ruta de salida.",
    guideNormalizeNoOutput: "La normalización necesita tanto la ruta del bundle como la ruta de salida. Ya tienes el bundle; lo que falta es el destino de salida en `Bundle tools`.",
    guideDiffNoCompare: "El diff de bundles necesita tanto el bundle actual como una ruta de comparación. Rellena el campo compare en `Bundle tools` y luego ejecuta `Diff bundles`.",
    guideBundleGeneral: "Usa `Inspect` primero para una lectura estructural segura. Usa `Validate` cuando quieras confianza de esquema, `Normalize` cuando quieras un bundle de salida limpio y `Diff` solo cuando ambas rutas de bundle estén completas.",
    guideWorkbench: "Abre Workbench solo cuando el runtime esté sano y el contexto del bundle parezca coherente. En Hub, la ruta corta es: `Inicio > Empezar aquí`, luego `Abrir Workbench`.",
    guideDocs: "Usa `Inicio > Guías` como estantería única de documentación. Empieza por `Índice de docs` y abre `Línea actual`, `Operaciones` o `Troubleshooting` solo cuando sepas qué clase de pregunta estás respondiendo.",
    guideRuntime: "Usa `Runtimes` cuando quieras cambiar el bucle, y `Observe` cuando solo quieras escanear o copiar estado. `Local runtime` es la lectura corta de salud, `Hot loop` es para tails de desarrollo y `Stack watch` es para logs saneados del runtime.",
    guideLibrary: "Usa `Inicio > Biblioteca` para la entrada de workloads. `Sincronizar control local` trae trabajo de primera parte, `Sincronizar catálogo remoto` trae entradas remotas, y los filtros de dominio/familia ayudan a estrechar la estantería antes de abrir Workbench.",
    guidePackaging: "Usa `Installer` cuando necesites layout de release o bootstrap de estación. En Hub, `Tools > Packages` es para acciones de build, `Status` es el muro de readiness y `Output` es donde cae el log de empaquetado. En esta sesión automática, los `.app` son fiables, mientras que los `.dmg` pueden seguir apareciendo como partial porque `hdiutil` es sensible a la sesión.",
    guideFailure: "Si algo parece partial, lee primero la superficie más corta: `Observe > Health` para problemas de runtime, `Tools > Status` para readiness del empaquetado de escritorio y `Bundle tools > Inspect` para la forma del bundle. Luego decide si el problema es runtime, bundle o packaging.",
    guideFallback: "La guía local puede ayudarte con los primeros pasos, la inspección de bundles, la salud del runtime, el arranque de Workbench, la entrada a la biblioteca de workloads y el empaquetado de escritorio. Prueba a preguntar directamente por una de esas cosas.",
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
  "desktop-stage": "sensitive",
  "desktop-verify": "sensitive",
  "desktop-build-host": "high",
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
  { id: "hub/desktopStage", summary: "Prepare desktop manifests for the selected platform.", payloadExample: {} },
  { id: "hub/desktopBuildHost", summary: "Build host bundles for the current machine.", payloadExample: {} },
  { id: "hub/desktopVerify", summary: "Verify the current desktop release staging area.", payloadExample: {} },
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
  hostPlatform: "macos",
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
  assistantApiKey: "",
  assistantTrustedHosts: loadHubAssistantTrustedHosts(),
  remoteTrustedHosts: loadHubTrustedHosts(HUB_REMOTE_TRUSTED_HOSTS_KEY),
};

let hotRuntimeLogPollHandle = null;
let observeRuntimeLogPollHandle = null;

function hubCopy() {
  return HUB_I18N[state.language] || HUB_I18N.en;
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
  if (elements.projectsTabStart) {
    elements.projectsTabStart.textContent = copy.home.tabs.start;
  }
  if (elements.projectsTabLibrary) {
    elements.projectsTabLibrary.textContent = copy.home.tabs.library;
  }
  if (elements.projectsTabBundles) {
    elements.projectsTabBundles.textContent = copy.home.tabs.bundles;
  }
  if (elements.projectsTabGuides) {
    elements.projectsTabGuides.textContent = copy.home.tabs.guides;
  }
  setText(elements.homeStep1Label, copy.home.steps.step1Label);
  setText(elements.homeStep1Title, copy.home.steps.step1Title);
  setText(elements.homeStep1Copy, copy.home.steps.step1Copy);
  setText(elements.homeStep2Label, copy.home.steps.step2Label);
  setText(elements.homeStep2Title, copy.home.steps.step2Title);
  setText(elements.homeStep2Copy, copy.home.steps.step2Copy);
  setText(elements.homeStep3Label, copy.home.steps.step3Label);
  setText(elements.homeStep3Title, copy.home.steps.step3Title);
  setText(elements.homeStep3Copy, copy.home.steps.step3Copy);
  setText(elements.homePathLabel, copy.home.path.label);
  setText(elements.homePathTitle, copy.home.path.title);
  setText(elements.homePathCopy, copy.home.path.copy);
  setText(elements.homeFlow1Title, copy.home.flow.title1);
  setText(elements.homeFlow1Copy, copy.home.flow.copy1);
  setText(elements.homeFlow2Title, copy.home.flow.title2);
  setText(elements.homeFlow2Copy, copy.home.flow.copy2);
  setText(elements.homeFlow3Title, copy.home.flow.title3);
  setText(elements.homeFlow3Copy, copy.home.flow.copy3);
  setText(elements.homeQuickLabel, copy.home.quick.label);
  setText(elements.homeQuickTitle, copy.home.quick.title);
  setText(elements.homeQuickCopy, copy.home.quick.copy);
  setText(elements.homeClusterLibraryTitle, copy.home.quick.libraryTitle);
  setText(elements.homeClusterLibraryCopy, copy.home.quick.libraryCopy);
  setText(elements.homeClusterBundlesTitle, copy.home.quick.bundlesTitle);
  setText(elements.homeClusterBundlesCopy, copy.home.quick.bundlesCopy);
  setText(elements.homeClusterGuidesTitle, copy.home.quick.guidesTitle);
  setText(elements.homeClusterGuidesCopy, copy.home.quick.guidesCopy);
  setText(elements.homeClusterInstallerTitle, copy.home.quick.installerTitle);
  setText(elements.homeClusterInstallerCopy, copy.home.quick.installerCopy);
  setText(elements.homeClusterRuntimesTitle, copy.home.quick.runtimesTitle);
  setText(elements.homeClusterRuntimesCopy, copy.home.quick.runtimesCopy);
  setText(elements.homeActionStart, copy.home.actions.start);
  setText(elements.homeActionSync, copy.home.actions.sync);
  setText(elements.homeActionOpen, copy.home.actions.open);
  setText(elements.libraryIntroLabel, copy.library.introLabel);
  setText(elements.libraryIntroTitle, copy.library.introTitle);
  setText(elements.libraryIntroCopy, copy.library.introCopy);
  setText(elements.libraryCatalogUrlLabel, copy.library.catalogUrl);
  setText(elements.libraryLabelNoteLabel, copy.library.labelOrNote);
  setText(elements.libraryActionRegister, copy.library.register);
  setText(elements.libraryActionSyncLocal, copy.library.syncLocal);
  setText(elements.libraryActionSyncRemote, copy.library.syncRemote);
  setText(elements.libraryActionExport, copy.library.export);
  setText(elements.libraryActionImport, copy.library.import);
  setText(elements.libraryActionClear, copy.library.clear);
  setText(elements.libraryManagedWorkloadsLabel, copy.library.managedWorkloads);
  setText(elements.libraryFilterAll, copy.library.all);
  setText(elements.libraryFilterMechanical, copy.library.mechanical);
  setText(elements.libraryFilterThermal, copy.library.thermal);
  setText(elements.libraryFilterThermo, copy.library.thermo);
  setText(elements.libraryFamilyAll, copy.library.allFamilies);
  setText(elements.libraryFamilyAxial, copy.library.axial);
  setText(elements.libraryFamilyBeams, copy.library.beams);
  setText(elements.libraryFamilyTrusses, copy.library.trusses);
  setText(elements.libraryFamilyPlanes, copy.library.planes);
  setText(elements.workflowCatalogLabel, copy.library.workflowCatalogLabel);
  setText(elements.workflowCatalogTitle, copy.library.workflowCatalogTitle);
  setText(elements.workflowCatalogCopy, copy.library.workflowCatalogCopy);
  setText(elements.workflowCatalogRefresh, copy.library.workflowCatalogRefresh);
  if (elements.workflowCatalogOutput && !state.workflowCatalogBusy) {
    elements.workflowCatalogOutput.textContent = copy.library.workflowCatalogReady;
  }
  if (elements.workloadLibraryOutput && !state.isBusy) {
    elements.workloadLibraryOutput.textContent = copy.library.ready;
  }
  setText(elements.bundlesIntroLabel, copy.bundles.introLabel);
  setText(elements.bundlesIntroTitle, copy.bundles.introTitle);
  setText(elements.bundlesIntroCopy, copy.bundles.introCopy);
  setText(elements.bundlesBundlePathLabel, copy.bundles.bundlePath);
  setText(elements.bundlesComparePathLabel, copy.bundles.comparePath);
  setText(elements.bundlesOutputPathLabel, copy.bundles.outputPath);
  setText(elements.bundlesActionInspect, copy.bundles.inspect);
  setText(elements.bundlesActionValidate, copy.bundles.validate);
  setText(elements.bundlesActionNormalize, copy.bundles.normalize);
  setText(elements.bundlesActionUnpack, copy.bundles.unpack);
  setText(elements.bundlesActionPack, copy.bundles.pack);
  setText(elements.bundlesActionDiff, copy.bundles.diff);
  setText(elements.bundlesActionOpenWorkbench, copy.bundles.openWorkbench);
  setText(elements.bundlesActionDesktopTools, copy.bundles.desktopTools);
  setText(elements.bundlesRecentBundlesLabel, copy.bundles.recentBundles);
  setText(elements.bundlesRecentCompareLabel, copy.bundles.recentCompare);
  setText(elements.bundlesRecentOutputsLabel, copy.bundles.recentOutputs);
  setText(elements.bundlesRecentActionsLabel, copy.bundles.recentActions);
  setText(elements.bundlesHistoryAll, copy.bundles.all);
  setText(elements.bundlesHistoryFailed, copy.bundles.failed);
  setText(elements.bundlesHistoryInspect, copy.bundles.inspect);
  setText(elements.bundlesHistoryNormalize, copy.bundles.normalize);
  setText(elements.bundlesHistoryDiff, copy.bundles.diff);
  setText(elements.bundlesHistoryKeepFailed, copy.bundles.keepFailed);
  setText(elements.bundlesHistoryImport, copy.bundles.import);
  setText(elements.bundlesHistoryExport, copy.bundles.export);
  setText(elements.bundlesHistoryClear, copy.bundles.clear);
  setText(elements.bundlesFavoritesLabel, copy.bundles.favorites);
  setText(elements.bundlesRecentLabel, copy.bundles.recent);
  if (elements.projectBundleOutput && !state.isBusy) {
    elements.projectBundleOutput.textContent = copy.bundles.ready;
  }
  setText(elements.guidesPrimaryLabel, copy.guides.primaryLabel);
  setText(elements.guidesPrimaryTitle, copy.guides.primaryTitle);
  setText(elements.guidesPrimaryCopy, copy.guides.primaryCopy);
  setText(elements.guidesDocsTitle, copy.guides.docsTitle);
  setText(elements.guidesDocsCopy, copy.guides.docsCopy);
  setText(elements.guidesCurrentTitle, copy.guides.currentTitle);
  setText(elements.guidesCurrentCopy, copy.guides.currentCopy);
  setText(elements.guidesOverviewDocsLabel, copy.guides.overviewDocsLabel);
  setText(elements.guidesOverviewDocsTitle, copy.guides.overviewDocsTitle);
  setText(elements.guidesOverviewDocsCopy, copy.guides.overviewDocsCopy);
  setText(elements.guidesOverviewCurrentLabel, copy.guides.overviewCurrentLabel);
  setText(elements.guidesOverviewCurrentTitle, copy.guides.overviewCurrentTitle);
  setText(elements.guidesOverviewCurrentCopy, copy.guides.overviewCurrentCopy);
  setText(elements.guidesOverviewTroubleshootingLabel, copy.guides.overviewTroubleshootingLabel);
  setText(elements.guidesOverviewTroubleshootingTitle, copy.guides.overviewTroubleshootingTitle);
  setText(elements.guidesOverviewTroubleshootingCopy, copy.guides.overviewTroubleshootingCopy);
  setText(elements.guidesOperationsTitle, copy.guides.operationsTitle);
  setText(elements.guidesOperationsCopy, copy.guides.operationsCopy);
  setText(elements.guidesTroubleshootingTitle, copy.guides.troubleshootingTitle);
  setText(elements.guidesTroubleshootingCopy, copy.guides.troubleshootingCopy);
  setText(elements.guidesAccuracyLabel, copy.guides.accuracyLabel);
  setText(elements.guidesAccuracyTitle, copy.guides.accuracyTitle);
  setText(elements.guidesAccuracyCopy, copy.guides.accuracyCopy);
  setText(elements.guidesAccuracyPlanTitle, copy.guides.accuracyPlanTitle);
  setText(elements.guidesAccuracyPlanCopy, copy.guides.accuracyPlanCopy);
  setText(elements.guidesAccuracyBaselinesTitle, copy.guides.accuracyBaselinesTitle);
  setText(elements.guidesAccuracyBaselinesCopy, copy.guides.accuracyBaselinesCopy);
  setText(elements.assistantIntroLabel, copy.assistant.introLabel);
  setText(elements.assistantIntroTitle, copy.assistant.introTitle);
  setText(elements.assistantIntroCopy, copy.assistant.introCopy);
  setText(elements.assistantClose, copy.assistant.close);
  setText(elements.assistantEngineLabel, copy.assistant.engine);
  setText(elements.assistantContextSectionLabel, copy.assistant.section);
  setText(elements.assistantContextRuntimeLabel, copy.assistant.runtime);
  setText(elements.assistantContextBundleLabel, copy.assistant.bundle);
  setText(elements.assistantLocalActionsLabel, copy.assistant.quickActions);
  setText(elements.assistantLocalActionStart, copy.assistant.quickStart);
  setText(elements.assistantLocalActionLibrary, copy.assistant.quickLibrary);
  setText(elements.assistantLocalActionBundles, copy.assistant.quickBundles);
  setText(elements.assistantLocalActionGuides, copy.assistant.quickGuides);
  setText(elements.assistantLocalAskLabel, copy.assistant.ask);
  setText(elements.assistantLocalPromptLabel, copy.assistant.askLabel);
  setText(elements.assistantLocalAsk, copy.assistant.askButton);
  if (elements.assistantLocalOutput && !state.isBusy) {
    elements.assistantLocalOutput.textContent = copy.assistant.askEmpty;
  }
  setText(elements.assistantDocsLabel, copy.assistant.docs);
  setText(elements.assistantDocsIndexTitle, copy.assistant.docsIndexTitle);
  setText(elements.assistantDocsIndexCopy, copy.assistant.docsIndexCopy);
  setText(elements.assistantDocsCurrentTitle, copy.assistant.docsCurrentTitle);
  setText(elements.assistantDocsCurrentCopy, copy.assistant.docsCurrentCopy);
  setText(elements.assistantDocsOperationsTitle, copy.assistant.docsOperationsTitle);
  setText(elements.assistantDocsOperationsCopy, copy.assistant.docsOperationsCopy);
  setText(elements.assistantDocsTroubleshootingTitle, copy.assistant.docsTroubleshootingTitle);
  setText(elements.assistantDocsTroubleshootingCopy, copy.assistant.docsTroubleshootingCopy);
  setText(elements.assistantSuggestedLabel, copy.assistant.suggested);
  setText(elements.assistantLlmIntroCopy, copy.assistant.llmIntro);
  setText(elements.assistantBaseUrlLabel, copy.assistant.baseUrl);
  setText(elements.assistantApiKeyLabel, copy.assistant.apiKey);
  setText(elements.assistantPresetLabel, copy.assistant.preset);
  setText(elements.assistantModelLabel, copy.assistant.model);
  setText(elements.assistantRequestLabel, copy.assistant.request);
  setText(elements.assistantRequestPlan, copy.assistant.generate);
  setText(elements.assistantApproveLabel, copy.assistant.approve);
  setText(elements.assistantExecutePlan, copy.assistant.execute);
  setText(elements.assistantEndpointPolicy, copy.dynamic.endpointPolicyDefault);
  if (elements.assistantOutput && !state.isBusy) {
    elements.assistantOutput.textContent = copy.assistant.ready;
  }
  setText(elements.assistantAuditLabel, copy.assistant.audit);
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

function renderOverviewStrip(section, items) {
  const cards = document.querySelectorAll(`#${section}-panel .hub-overview-card`);
  items?.forEach((item, index) => {
    const card = cards[index];
    if (!card) return;
    const eyebrow = card.querySelector(".hub-card__eyebrow");
    const title = card.querySelector("h2");
    const copy = card.querySelector(".desktop-shell-note");
    if (eyebrow) eyebrow.textContent = item.label;
    if (title) title.textContent = item.title;
    if (copy) copy.textContent = item.copy;
  });
}

function renderPanelTabGroup(group, labels) {
  const buttons = document.querySelectorAll(`[data-panel-page-group="${group}"]`);
  labels?.forEach((label, index) => {
    const button = buttons[index];
    if (button) button.textContent = label;
  });
}

function renderPanelLanguage(copy) {
  renderPanelTabGroup("runtimes", copy.panels.runtimes.tabs);
  renderPanelTabGroup("deploy", copy.panels.deploy.tabs);
  renderPanelTabGroup("observe", copy.panels.observe.tabs);
  renderPanelTabGroup("tools", copy.panels.tools.tabs);
  renderOverviewStrip("runtimes", copy.panels.runtimes.overview);
  renderOverviewStrip("observe", copy.panels.observe.overview);
  renderOverviewStrip("tools", copy.panels.tools.overview);
  setText(elements.runtimeLocalLabel, copy.panels.runtimes.local.label);
  setText(elements.runtimeLocalTitle, copy.panels.runtimes.local.title);
  setText(elements.runtimeLocalCopy, copy.panels.runtimes.local.copy);
  setText(elements.runtimeLocalStatusLabel, copy.panels.runtimes.local.status);
  setText(elements.runtimeLocalFrontendLabel, copy.panels.runtimes.local.frontend);
  setText(elements.runtimeLocalControlLabel, copy.panels.runtimes.local.controlPlane);
  setText(elements.runtimeLocalAgentsLabel, copy.panels.runtimes.local.agents);
  setText(elements.runtimeHotLabel, copy.panels.runtimes.hot.label);
  setText(elements.runtimeHotTitle, copy.panels.runtimes.hot.title);
  setText(elements.runtimeHotCopy, copy.panels.runtimes.hot.copy);
  setText(elements.runtimeHotStatusLabel, copy.panels.runtimes.hot.status);
  setText(elements.runtimeHotModeLabel, copy.panels.runtimes.hot.mode);
  setText(elements.runtimeHotActionLocal, copy.panels.runtimes.hot.local);
  setText(elements.runtimeHotActionCloud, copy.panels.runtimes.hot.cloud);
  setText(elements.runtimeHotActionDistributed, copy.panels.runtimes.hot.distributed);
  setText(elements.runtimeHotActionRefresh, copy.panels.runtimes.hot.refreshStatus);
  setText(elements.runtimeHotActionStop, copy.panels.runtimes.hot.stop);
  setText(elements.runtimeHotLogsLabel, copy.panels.runtimes.hot.logs);
  setText(elements.runtimeHotAutoLabel, copy.panels.runtimes.hot.autoRefresh);
  setText(elements.runtimeHotIntervalLabel, copy.panels.runtimes.hot.interval);
  setText(elements.runtimeHotRefreshLog, copy.panels.runtimes.hot.refreshLog);
  setText(elements.runtimeHotCopyTail, copy.panels.runtimes.hot.copyTail);
  setText(elements.runtimeHotClearView, copy.panels.runtimes.hot.clearView);
  setText(elements.runtimeHotNote, copy.panels.runtimes.hot.note);
  setText(elements.runtimeTargetsLabel, copy.panels.runtimes.targets.label);
  setText(elements.runtimeTargetsTitle, copy.panels.runtimes.targets.title);
  setText(elements.runtimeTargetsCopy, copy.panels.runtimes.targets.copy);
  setText(elements.deployModesLabel, copy.panels.deploy.modes.label);
  setText(elements.deployModesTitle, copy.panels.deploy.modes.title);
  setText(elements.deployModesCopy, copy.panels.deploy.modes.copy);
  setText(elements.deployActionLocal, copy.panels.deploy.modes.local);
  setText(elements.deployActionCloud, copy.panels.deploy.modes.cloud);
  setText(elements.deployActionDistributed, copy.panels.deploy.modes.distributed);
  setText(elements.deployActionRestart, copy.panels.deploy.modes.restart);
  setText(elements.deployBootstrapLabel, copy.panels.deploy.bootstrap.label);
  setText(elements.deployBootstrapTitle, copy.panels.deploy.bootstrap.title);
  setText(elements.deployBootstrapCopy, copy.panels.deploy.bootstrap.copy);
  setText(elements.deployBootstrapValidate, copy.panels.deploy.bootstrap.validate);
  setText(elements.deployBootstrapStage, copy.panels.deploy.bootstrap.stage);
  setText(elements.deployBootstrapDoctor, copy.panels.deploy.bootstrap.doctor);
  setText(elements.deployReleaseLabel, copy.panels.deploy.release.label);
  setText(elements.deployReleaseTitle, copy.panels.deploy.release.title);
  setText(elements.deployReleaseCopy, copy.panels.deploy.release.copy);
  setText(elements.observeHealthLabel, copy.panels.observe.health.label);
  setText(elements.observeHealthTitle, copy.panels.observe.health.title);
  setText(elements.observeHealthCopy, copy.panels.observe.health.copy);
  setText(elements.observeHealthWatchdogLabel, copy.panels.observe.health.watchdog);
  setText(elements.observeHealthSecurityLabel, copy.panels.observe.health.security);
  setText(elements.observeHealthFailuresLabel, copy.panels.observe.health.failures);
  setText(elements.observeRuntimeTitle, copy.panels.observe.runtime.title);
  setText(elements.observeRuntimeStatusLabel, copy.panels.observe.runtime.localRuntime);
  setText(elements.observeRuntimeHotLabel, copy.panels.observe.runtime.hotLoop);
  setText(elements.observeRuntimeModeLabel, copy.panels.observe.runtime.mode);
  setText(elements.observeRuntimeSourceLabel, copy.panels.observe.runtime.logSource);
  setText(elements.observeRuntimeOpen, copy.panels.observe.runtime.open);
  setText(elements.observeRuntimeRefresh, copy.panels.observe.runtime.refresh);
  setText(elements.observeRuntimeCopy, copy.panels.observe.runtime.copy);
  setText(elements.observeStackTitle, copy.panels.observe.stack.title);
  setText(elements.observeStackLogsLabel, copy.panels.observe.stack.logs);
  setText(elements.observeStackAutoLabel, copy.panels.observe.stack.auto);
  setText(elements.observeStackRefresh, copy.panels.observe.stack.refresh);
  setText(elements.observeStackCopy, copy.panels.observe.stack.copy);
  setText(elements.observeStackNote, copy.panels.observe.stack.note);
  setText(elements.toolsPackagesLabel, copy.panels.tools.packages.label);
  setText(elements.toolsPackagesTitle, copy.panels.tools.packages.title);
  setText(elements.toolsPackagesCopy, copy.panels.tools.packages.copy);
  setText(elements.toolsPackagesPlatformLabel, copy.panels.tools.packages.platform);
  setText(elements.toolsPackagesBenchmark, copy.panels.tools.packages.benchmark);
  setText(elements.toolsPackagesValidate, copy.panels.tools.packages.validate);
  setText(elements.toolsPackagesExport, copy.panels.tools.packages.export);
  setText(elements.toolsPackagesStatus, copy.panels.tools.packages.status);
  setText(elements.toolsPackagesStage, copy.panels.tools.packages.stage);
  setText(elements.toolsPackagesBuild, copy.panels.tools.packages.build);
  setText(elements.toolsPackagesVerify, copy.panels.tools.packages.verify);
  setText(elements.toolsPackagesStop, copy.panels.tools.packages.stop);
  setText(elements.toolsStatusLabel, copy.panels.tools.status.label);
  setText(elements.toolsStatusTitle, copy.panels.tools.status.title);
  setText(elements.toolsStatusCopy, copy.panels.tools.status.copy);
  setText(elements.toolsOutputLabel, copy.panels.tools.output.label);
  setText(elements.toolsOutputTitle, copy.panels.tools.output.title);
  setText(elements.toolsOutputCopy, copy.panels.tools.output.copy);
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
  try {
    const raw = window.localStorage.getItem(HUB_WORKLOAD_LIBRARY_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persistHubWorkloadLibrary(entries) {
  window.localStorage.setItem(
    HUB_WORKLOAD_LIBRARY_KEY,
    JSON.stringify(entries.slice(0, HUB_WORKLOAD_LIBRARY_LIMIT)),
  );
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
  return [
    String(entry?.sourceKind || "").trim(),
    String(entry?.bundlePath || "").trim(),
    String(entry?.downloadUrl || "").trim(),
    String(entry?.projectId || "").trim(),
  ].join("::");
}

function normalizeHubWorkloadEntry(entry) {
  const label = String(entry?.label || entry?.projectName || "").trim();
  const sourceKind = String(entry?.sourceKind || "").trim() || "local-bundle";
  const bundlePath = String(entry?.bundlePath || "").trim();
  const downloadUrl = String(entry?.downloadUrl || "").trim();
  const projectId = String(entry?.projectId || "").trim();
  const projectName = String(entry?.projectName || "").trim();

  if (!label && !bundlePath && !downloadUrl && !projectId) {
    return null;
  }

  return {
    id: String(entry?.id || `workload-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    label: label || projectName || bundlePath || downloadUrl || "workload",
    note: String(entry?.note || "").trim(),
    sourceKind,
    sourceLabel: String(entry?.sourceLabel || "").trim(),
    bundlePath,
    downloadUrl,
    projectId,
    projectName,
    schema: String(entry?.schema || "").trim(),
    layout: String(entry?.layout || "").trim(),
    modelCount: Number.isFinite(Number(entry?.modelCount)) ? Number(entry.modelCount) : 0,
    versionCount: Number.isFinite(Number(entry?.versionCount)) ? Number(entry.versionCount) : 0,
    jobCount: Number.isFinite(Number(entry?.jobCount)) ? Number(entry.jobCount) : 0,
    resultCount: Number.isFinite(Number(entry?.resultCount)) ? Number(entry.resultCount) : 0,
    analysisDomains: Array.isArray(entry?.analysisDomains)
      ? entry.analysisDomains.filter((value) => typeof value === "string")
      : Array.isArray(entry?.analysis_domains)
        ? entry.analysis_domains.filter((value) => typeof value === "string")
        : [],
    analysisFamilies: Array.isArray(entry?.analysisFamilies)
      ? entry.analysisFamilies.filter((value) => typeof value === "string")
      : Array.isArray(entry?.analysis_families)
        ? entry.analysis_families.filter((value) => typeof value === "string")
        : [],
    thermalIntents: Array.isArray(entry?.thermalIntents)
      ? entry.thermalIntents.filter((value) => typeof value === "string")
      : Array.isArray(entry?.thermal_intents)
        ? entry.thermal_intents.filter((value) => typeof value === "string")
        : [],
    downloadedAt: String(entry?.downloadedAt || "").trim(),
    attachedAt: String(entry?.attachedAt || "").trim(),
    addedAt: String(entry?.addedAt || "").trim() || new Date().toISOString(),
    updatedAt: String(entry?.updatedAt || "").trim() || new Date().toISOString(),
  };
}

function mergeHubWorkloadLibrary(existingEntries, incomingEntries) {
  const merged = [];

  for (const candidate of [...incomingEntries, ...existingEntries]) {
    const normalized = normalizeHubWorkloadEntry(candidate);
    if (!normalized) {
      continue;
    }

    const duplicateIndex = merged.findIndex((entry) => workloadIdentity(entry) === workloadIdentity(normalized));
    if (duplicateIndex >= 0) {
      continue;
    }

    merged.push(normalized);
    if (merged.length >= HUB_WORKLOAD_LIBRARY_LIMIT) {
      break;
    }
  }

  return merged;
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
  try {
    const parsed = new URL(String(url || "").trim());
    const pathname = parsed.pathname.split("/").filter(Boolean).at(-1);
    return pathname || fallback;
  } catch {
    return fallback;
  }
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
  if (entry.sourceKind === "remote-catalog" && entry.bundlePath) {
    return ["attached local", "desktop-shell-state desktop-shell-state--healthy"];
  }

  if (entry.sourceKind === "remote-catalog" && entry.downloadedAt) {
    return ["downloaded", "desktop-shell-state desktop-shell-state--warning"];
  }

  switch (entry.sourceKind) {
    case "remote-catalog":
      return ["remote catalog", "desktop-shell-state desktop-shell-state--healthy"];
    case "imported-library":
      return ["imported", "desktop-shell-state desktop-shell-state--warning"];
    default:
      return ["local bundle", "desktop-shell-state desktop-shell-state--idle"];
  }
}

function workloadProvenanceLabel(entry) {
  if (entry.sourceKind === "remote-catalog") {
    if (entry.sourceLabel === "Kyuubiki Control Plane") {
      return "first-party control plane catalog";
    }
    const hostHint = workloadProvenanceHost(entry.sourceLabel || entry.downloadUrl || "");
    if (hostHint) {
      return `custom remote catalog · ${hostHint}`;
    }
    return `custom remote catalog${entry.sourceLabel ? ` · ${entry.sourceLabel}` : ""}`;
  }

  if (entry.sourceKind === "imported-library") {
    return "imported library snapshot";
  }

  if (entry.sourceLabel) {
    return entry.sourceLabel;
  }

  return "Hub local registration";
}

function workloadProvenanceHost(value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return "";
  }

  try {
    return new URL(normalized).host;
  } catch {
    return "";
  }
}

function workloadDomainLabel(domain) {
  switch (domain) {
    case "mechanical":
      return "Mechanical";
    case "thermal":
      return "Thermal";
    case "thermo_mechanical":
      return "Thermo-mechanical";
    default:
      return String(domain || "").trim();
  }
}

function workloadFamilyLabel(family) {
  switch (family) {
    case "axial_and_springs":
      return "Axial & Springs";
    case "beams_and_frames":
      return "Beams & Frames";
    case "trusses":
      return "Trusses";
    case "planes":
      return "Planes";
    default:
      return String(family || "").trim();
  }
}

function markHubWorkloadDownloaded(entry) {
  const next = loadHubWorkloadLibrary().map((candidate) => {
    if (workloadIdentity(candidate) !== workloadIdentity(entry)) {
      return candidate;
    }

    return {
      ...candidate,
      downloadedAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  });
  saveHubWorkloadLibrary(next);
}

function updateHubWorkloadEntry(entry, updater) {
  const next = loadHubWorkloadLibrary()
    .map((candidate) => {
      if (workloadIdentity(candidate) !== workloadIdentity(entry)) {
        return candidate;
      }

      return normalizeHubWorkloadEntry(
        updater({
          ...candidate,
        }),
      );
    })
    .filter(Boolean);
  saveHubWorkloadLibrary(next);
}

async function downloadRemoteWorkloadBundle(entry) {
  const validation = validateHubCatalogUrl(entry.downloadUrl || "");
  if (!validation.ok) {
    throw new Error(validation.reason);
  }

  if (!ensureRemoteHostTrust(validation.normalized, "This workload download")) {
    throw new Error("workload download cancelled before contacting the remote host");
  }

  const response = await fetch(validation.normalized);
  if (!response.ok) {
    throw new Error(`bundle download failed (${response.status})`);
  }

  const blob = await response.blob();
  const filename = inferDownloadFilename(validation.normalized);
  downloadHubBlob(filename, blob);
  markHubWorkloadDownloaded(entry);
  setWorkloadLibraryOutput(`downloaded ${entry.label} as ${filename}`);
}

async function openWorkloadInWorkbench(entry) {
  if (!entry.bundlePath) {
    throw new Error("This workload does not have a local bundle path yet.");
  }

  elements.projectBundlePath.value = entry.bundlePath;
  renderAssistantContext();
  renderHubAssistantLocalCards();
  setWorkloadLibraryOutput(`loaded ${entry.label} into the bundle path and opening Workbench`);
  await runAction("open-workbench");
}

async function attachCurrentBundleToWorkload(entry) {
  const bundlePath = String(elements.projectBundlePath?.value || "").trim();
  if (!bundlePath) {
    throw new Error("Fill in the current bundle path before attaching it to this workload.");
  }

  const inspectRaw = await invokeTauri("project_bundle_inspect", { payload: { path: bundlePath } });
  const summary = projectSummaryFromInspectPayload(inspectRaw);
  updateHubWorkloadEntry(entry, (candidate) => ({
    ...candidate,
    bundlePath,
    projectId: summary.projectId || candidate.projectId,
    projectName: summary.projectName || candidate.projectName,
    schema: summary.schema || candidate.schema,
    layout: summary.layout || candidate.layout,
    modelCount: summary.modelCount,
    versionCount: summary.versionCount,
    jobCount: summary.jobCount,
    resultCount: summary.resultCount,
    analysisDomains: summary.analysisDomains,
    analysisFamilies: summary.analysisFamilies,
    thermalIntents: summary.thermalIntents,
    attachedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  }));
  setWorkloadLibraryOutput(`attached local bundle ${bundlePath} to ${entry.label}`);
}

function saveHubWorkloadLibrary(entries) {
  persistHubWorkloadLibrary(entries);
  renderHubWorkloadLibrary(entries);
}

function matchesWorkloadFilter(entry) {
  if (state.workloadFilter === "all") {
    return matchesWorkloadFamilyFilter(entry);
  }
  return entry.analysisDomains.includes(state.workloadFilter) && matchesWorkloadFamilyFilter(entry);
}

function matchesWorkloadFamilyFilter(entry) {
  if (state.workloadFamilyFilter === "all") {
    return true;
  }
  return entry.analysisFamilies.includes(state.workloadFamilyFilter);
}

function renderWorkloadFilters() {
  elements.workloadFilterButtons.forEach((button) => {
    const matches = button.dataset.workloadFilter === state.workloadFilter;
    button.classList.toggle("desktop-shell-button-primary", matches);
    button.classList.toggle("desktop-shell-button-ghost", !matches);
  });
  elements.workloadFamilyFilterButtons.forEach((button) => {
    const matches = button.dataset.workloadFamilyFilter === state.workloadFamilyFilter;
    button.classList.toggle("desktop-shell-button-primary", matches);
    button.classList.toggle("desktop-shell-button-ghost", !matches);
  });
}

function renderHubWorkloadLibrary(entries = loadHubWorkloadLibrary()) {
  if (!elements.workloadLibraryList) {
    return;
  }

  renderWorkloadFilters();
  elements.workloadLibraryList.innerHTML = "";
  if (!entries.length) {
    renderEmptyHistoryState(
      elements.workloadLibraryList,
      hubCopy().dynamic?.managedWorkloadsEmpty
        || HUB_I18N.en.dynamic?.managedWorkloadsEmpty
        || "No managed workloads yet. Register a current bundle or sync a remote catalog.",
    );
    return;
  }

  const filteredEntries = entries.filter((entry) => matchesWorkloadFilter(entry));
  if (!filteredEntries.length) {
    const domainLabel = localizedWorkloadFilterLabel(state.workloadFilter);
    const familyLabel = localizedWorkloadFamilyFilterLabel(state.workloadFamilyFilter);
    renderEmptyHistoryState(
      elements.workloadLibraryList,
      hubMessage(
        hubCopy().dynamic?.managedWorkloadsFilterEmpty
          || HUB_I18N.en.dynamic?.managedWorkloadsFilterEmpty
          || "No workloads match {domain} / {family}.",
        { domain: domainLabel, family: familyLabel },
      ),
    );
    return;
  }

  filteredEntries.forEach((entry) => {
    const shell = document.createElement("div");
    shell.className = "hub-history-item";

    const summary = document.createElement("button");
    summary.type = "button";
    summary.className = "hub-history-item__summary desktop-shell-button-ghost";
    const [sourceLabel, sourceClass] = workloadSourceBadge(entry);
    const metaBits = [
      entry.projectId ? `project ${entry.projectId}` : "",
      entry.schema || "",
      entry.layout || "",
      entry.attachedAt ? `attached ${formatProjectActionTime(entry.attachedAt)}` : "",
      entry.downloadedAt ? `downloaded ${formatProjectActionTime(entry.downloadedAt)}` : "",
    ].filter(Boolean);
    const heading = document.createElement("div");
    heading.className = "hub-history-item__heading";
    appendTextElement(heading, "strong", entry.label);
    const meta = document.createElement("div");
    meta.className = "hub-history-item__meta";
    appendTextElement(meta, "span", sourceLabel, sourceClass);
    entry.analysisDomains.forEach((domain) => {
      appendTextElement(meta, "span", workloadDomainLabel(domain), "desktop-shell-chip");
    });
    entry.analysisFamilies.forEach((family) => {
      appendTextElement(meta, "span", workloadFamilyLabel(family), "desktop-shell-chip");
    });
    heading.appendChild(meta);
    summary.appendChild(heading);
    appendTextElement(summary, "span", metaBits.join(" · ") || "workload entry", "hub-history-item__alias");
    appendTextElement(summary, "span", entry.note || entry.bundlePath || entry.downloadUrl || "--");
    appendTextElement(summary, "span", workloadProvenanceLabel(entry), "hub-history-item__provenance");
    if (entry.thermalIntents.length) {
      appendTextElement(summary, "span", `thermal: ${entry.thermalIntents.join(", ")}`, "desktop-shell-note");
    }
    summary.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
      }
      if (entry.downloadUrl && elements.workloadCatalogUrl) {
        elements.workloadCatalogUrl.value = entry.downloadUrl;
      }
      setWorkloadLibraryOutput(
        hubMessage(
          hubCopy().dynamic?.restoredWorkloadContext
            || HUB_I18N.en.dynamic?.restoredWorkloadContext
            || "restored workload context for {label}",
          { label: entry.label },
        ),
      );
      renderAssistantContext();
      renderHubAssistantLocalCards();
    });

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";

    const useButton = document.createElement("button");
    useButton.type = "button";
    useButton.className = "desktop-shell-button-ghost";
    useButton.textContent = hubCopy().dynamic?.workloadUse || HUB_I18N.en.dynamic?.workloadUse || "Use";
    useButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
      }
      setWorkloadLibraryOutput(
        hubMessage(
          hubCopy().dynamic?.loadedWorkloadContext
            || HUB_I18N.en.dynamic?.loadedWorkloadContext
            || "loaded {label} into the bundle path",
          { label: entry.label },
        ),
      );
      renderAssistantContext();
      renderHubAssistantLocalCards();
    });

    const workbenchButton = document.createElement("button");
    workbenchButton.type = "button";
    workbenchButton.className = "desktop-shell-button-ghost";
    workbenchButton.textContent = hubCopy().dynamic?.workloadOpenWorkbench || HUB_I18N.en.dynamic?.workloadOpenWorkbench || "Open in Workbench";
    workbenchButton.disabled = !entry.bundlePath;
    workbenchButton.addEventListener("click", () => {
      void openWorkloadInWorkbench(entry).catch((error) => {
        setWorkloadLibraryOutput(formatHubOperatorError(error, {
          actionLabel: "Opening this workload in Workbench",
        }));
      });
    });

    const inspectButton = document.createElement("button");
    inspectButton.type = "button";
    inspectButton.className = "desktop-shell-button-ghost";
    inspectButton.textContent = hubCopy().dynamic?.workloadInspect || HUB_I18N.en.dynamic?.workloadInspect || "Inspect";
    inspectButton.disabled = !entry.bundlePath;
    inspectButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
        void runAction("project-inspect");
      }
    });

    const validateButton = document.createElement("button");
    validateButton.type = "button";
    validateButton.className = "desktop-shell-button-ghost";
    validateButton.textContent = hubCopy().dynamic?.workloadValidate || HUB_I18N.en.dynamic?.workloadValidate || "Validate";
    validateButton.disabled = !entry.bundlePath;
    validateButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
        void runAction("project-validate");
      }
    });

    const downloadButton = document.createElement("button");
    downloadButton.type = "button";
    downloadButton.className = "desktop-shell-button-ghost";
    downloadButton.textContent = hubCopy().dynamic?.workloadDownload || HUB_I18N.en.dynamic?.workloadDownload || "Download";
    downloadButton.disabled = !entry.downloadUrl;
    downloadButton.addEventListener("click", () => {
      void downloadRemoteWorkloadBundle(entry).catch((error) => {
        setWorkloadLibraryOutput(formatHubOperatorError(error, {
          actionLabel: "Downloading this workload",
        }));
      });
    });

    const attachButton = document.createElement("button");
    attachButton.type = "button";
    attachButton.className = "desktop-shell-button-ghost";
    attachButton.textContent = entry.bundlePath
      ? hubCopy().dynamic?.workloadReattach || HUB_I18N.en.dynamic?.workloadReattach || "Reattach bundle"
      : hubCopy().dynamic?.workloadAttach || HUB_I18N.en.dynamic?.workloadAttach || "Attach current bundle";
    attachButton.addEventListener("click", () => {
      void attachCurrentBundleToWorkload(entry).catch((error) => {
        setWorkloadLibraryOutput(formatHubOperatorError(error, {
          actionLabel: "Attaching the current bundle",
        }));
      });
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "desktop-shell-button-ghost";
    removeButton.textContent = hubCopy().dynamic?.workloadRemove || HUB_I18N.en.dynamic?.workloadRemove || "Remove";
    removeButton.addEventListener("click", () => {
      const next = loadHubWorkloadLibrary().filter((candidate) => workloadIdentity(candidate) !== workloadIdentity(entry));
      saveHubWorkloadLibrary(next);
      setWorkloadLibraryOutput(
        hubMessage(
          hubCopy().dynamic?.removedWorkload
            || HUB_I18N.en.dynamic?.removedWorkload
            || "removed {label} from the workload library",
          { label: entry.label },
        ),
      );
    });

    controls.append(useButton, workbenchButton, inspectButton, validateButton, downloadButton, attachButton, removeButton);
    shell.append(summary, controls);
    elements.workloadLibraryList.appendChild(shell);
  });
}

function projectSummaryFromInspectPayload(raw) {
  const parsed = JSON.parse(raw);
  return {
    projectId: String(parsed?.project_id || "").trim(),
    projectName: String(parsed?.project_name || "").trim(),
    schema: String(parsed?.schema || "").trim(),
    layout: String(parsed?.layout || "").trim(),
    modelCount: Number(parsed?.model_count || 0),
    versionCount: Number(parsed?.version_count || 0),
    jobCount: Number(parsed?.job_count || 0),
    resultCount: Number(parsed?.result_count || 0),
    analysisDomains: Array.isArray(parsed?.analysis_domains) ? parsed.analysis_domains.filter((value) => typeof value === "string") : [],
    analysisFamilies: Array.isArray(parsed?.analysis_families) ? parsed.analysis_families.filter((value) => typeof value === "string") : [],
    thermalIntents: Array.isArray(parsed?.thermal_intents) ? parsed.thermal_intents.filter((value) => typeof value === "string") : [],
  };
}

async function registerCurrentBundleAsWorkload() {
  const bundlePath = String(elements.projectBundlePath?.value || "").trim();
  if (!bundlePath) {
    throw new Error("Fill in a bundle path before registering a workload.");
  }

  const inspectRaw = await invokeTauri("project_bundle_inspect", { payload: { path: bundlePath } });
  const summary = projectSummaryFromInspectPayload(inspectRaw);
  const note = String(elements.workloadLabel?.value || "").trim();
  const entry = normalizeHubWorkloadEntry({
    label: note || summary.projectName || summary.projectId || bundlePath,
    note: note || `Registered from local bundle ${bundlePath}`,
    sourceKind: "local-bundle",
    sourceLabel: "Hub local registration",
    bundlePath,
    projectId: summary.projectId,
    projectName: summary.projectName,
    schema: summary.schema,
    layout: summary.layout,
    modelCount: summary.modelCount,
    versionCount: summary.versionCount,
    jobCount: summary.jobCount,
    resultCount: summary.resultCount,
    analysisDomains: summary.analysisDomains,
    analysisFamilies: summary.analysisFamilies,
    thermalIntents: summary.thermalIntents,
  });

  const next = mergeHubWorkloadLibrary(loadHubWorkloadLibrary(), [entry]);
  saveHubWorkloadLibrary(next);
  setWorkloadLibraryOutput(`registered ${entry.label} in the workload library`);
}

function validateHubCatalogUrl(value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return { ok: false, reason: "Fill in a workload catalog URL first." };
  }

  try {
    const parsed = new URL(normalized);
    const protocol = parsed.protocol.toLowerCase();
    const hostname = parsed.hostname.toLowerCase();
    const isLoopback =
      hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
    if (protocol === "https:" || (protocol === "http:" && isLoopback)) {
      return { ok: true, normalized };
    }
    return {
      ok: false,
      reason: "Catalog URL must use https, or http only for localhost / 127.0.0.1 / ::1.",
    };
  } catch {
    return { ok: false, reason: "Catalog URL must be a valid absolute URL." };
  }
}

function validateRemoteWorkloadCatalogPayload(payload) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return { ok: false, reason: "Catalog payload must be a JSON object." };
  }

  if (payload.schema_version !== "kyuubiki.workload-catalog/v1") {
    return {
      ok: false,
      reason: "Catalog schema_version must be kyuubiki.workload-catalog/v1.",
    };
  }

  if (!Array.isArray(payload.workloads)) {
    return { ok: false, reason: "Catalog workloads must be an array." };
  }

  for (const [index, workload] of payload.workloads.entries()) {
    if (!workload || typeof workload !== "object" || Array.isArray(workload)) {
      return { ok: false, reason: `Workload ${index + 1} must be an object.` };
    }

    if (!String(workload.label || "").trim()) {
      return { ok: false, reason: `Workload ${index + 1} is missing label.` };
    }

    const hasRequiredLocator =
      String(workload.download_url || "").trim() ||
      String(workload.bundle_path || "").trim() ||
      String(workload.project_id || "").trim();
    if (!hasRequiredLocator) {
    return {
      ok: false,
      reason: `Workload ${index + 1} must define download_url, bundle_path, or project_id.`,
    };
  }

    if (
      workload.analysis_domains !== undefined &&
      (!Array.isArray(workload.analysis_domains) ||
        workload.analysis_domains.some(
          (value) =>
            typeof value !== "string" ||
            !["mechanical", "thermal", "thermo_mechanical"].includes(value),
        ))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid analysis_domains.`,
      };
    }

    if (
      workload.analysis_families !== undefined &&
      (!Array.isArray(workload.analysis_families) ||
        workload.analysis_families.some(
          (value) =>
            typeof value !== "string" ||
            !["axial_and_springs", "beams_and_frames", "trusses", "planes"].includes(value),
        ))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid analysis_families.`,
      };
    }

    if (
      workload.thermal_intents !== undefined &&
      (!Array.isArray(workload.thermal_intents) ||
        workload.thermal_intents.some((value) => typeof value !== "string"))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid thermal_intents.`,
      };
    }
  }

  return { ok: true };
}

function normalizeRemoteWorkloadCatalogPayload(payload, catalogUrl) {
  const list = Array.isArray(payload)
    ? payload
    : Array.isArray(payload?.workloads)
      ? payload.workloads
      : Array.isArray(payload?.items)
        ? payload.items
        : [];

  return list
    .map((entry) =>
      normalizeHubWorkloadEntry({
        label: entry?.label || entry?.name || entry?.projectName || entry?.project_name,
        note: entry?.note || entry?.description || `Synced from ${catalogUrl}`,
        sourceKind: "remote-catalog",
        sourceLabel: entry?.sourceLabel || payload?.sourceLabel || catalogUrl,
        bundlePath: entry?.bundlePath || entry?.bundle_path || "",
        downloadUrl: entry?.downloadUrl || entry?.download_url || catalogUrl,
        projectId: entry?.projectId || entry?.project_id || "",
        projectName: entry?.projectName || entry?.project_name || "",
        schema: entry?.schema || "",
        layout: entry?.layout || "",
        modelCount: entry?.modelCount || entry?.model_count || 0,
        versionCount: entry?.versionCount || entry?.version_count || 0,
        jobCount: entry?.jobCount || entry?.job_count || 0,
        resultCount: entry?.resultCount || entry?.result_count || 0,
        analysisDomains: entry?.analysisDomains || entry?.analysis_domains || [],
        analysisFamilies: entry?.analysisFamilies || entry?.analysis_families || [],
        thermalIntents: entry?.thermalIntents || entry?.thermal_intents || [],
      }),
    )
    .filter(Boolean);
}

async function syncRemoteWorkloadCatalog(urlOverride = "") {
  const selectedUrl =
    String(urlOverride || "").trim() || String(elements.workloadCatalogUrl?.value || "").trim();
  const validation = validateHubCatalogUrl(selectedUrl);
  if (!validation.ok) {
    throw new Error(validation.reason);
  }

  if (elements.workloadCatalogUrl) {
    elements.workloadCatalogUrl.value = validation.normalized;
  }

  if (!ensureRemoteHostTrust(validation.normalized, "This remote catalog sync")) {
    throw new Error("remote catalog sync cancelled before contacting the remote host");
  }

  const response = await fetch(validation.normalized);
  if (!response.ok) {
    throw new Error(`catalog sync failed (${response.status})`);
  }

  const payload = await response.json();
  const payloadValidation = validateRemoteWorkloadCatalogPayload(payload);
  if (!payloadValidation.ok) {
    throw new Error(payloadValidation.reason);
  }
  const normalized = normalizeRemoteWorkloadCatalogPayload(payload, validation.normalized);
  const next = mergeHubWorkloadLibrary(loadHubWorkloadLibrary(), normalized);
  saveHubWorkloadLibrary(next);
  setWorkloadLibraryOutput(`synced ${normalized.length} workload entries from remote catalog`);
}

async function syncLocalControlPlaneWorkloads() {
  const catalogUrl = ensureDefaultWorkloadCatalogUrl(true);
  await syncRemoteWorkloadCatalog(catalogUrl);
}

function exportHubWorkloadLibrary() {
  const payload = {
    exportedAt: new Date().toISOString(),
    workloadCount: loadHubWorkloadLibrary().length,
    workloads: loadHubWorkloadLibrary(),
  };
  downloadHubJson("kyuubiki-hub-workloads.json", payload);
  setWorkloadLibraryOutput(`exported ${payload.workloadCount} workload entries as JSON`);
}

async function importHubWorkloadLibrary(file) {
  if (!file) {
    return;
  }

  const raw = await file.text();
  const parsed = JSON.parse(raw);
  const imported = Array.isArray(parsed?.workloads) ? parsed.workloads : [];
  const normalized = imported
    .map((entry) =>
      normalizeHubWorkloadEntry({
        ...entry,
        sourceKind: entry?.sourceKind || "imported-library",
      }),
    )
    .filter(Boolean);
  const next = mergeHubWorkloadLibrary(loadHubWorkloadLibrary(), normalized);
  saveHubWorkloadLibrary(next);
  setWorkloadLibraryOutput(`imported ${normalized.length} workload entries into the Hub library`);
}

function clearHubWorkloadLibrary() {
  saveHubWorkloadLibrary([]);
  setWorkloadLibraryOutput("cleared the Hub workload library");
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

function currentFilteredHistoryActions(actions = loadHubRecents().actions ?? []) {
  return actions.filter((entry) => matchesHistoryFilter(entry));
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

function downloadHubJson(filename, payload) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}

function exportRecentActionHistory() {
  const recents = loadHubRecents();
  const actions = currentFilteredHistoryActions(recents.actions ?? []);
  const payload = {
    exportedAt: new Date().toISOString(),
    filter: state.historyFilter,
    actionCount: actions.length,
    actions,
  };

  downloadHubJson(`kyuubiki-hub-recent-actions-${state.historyFilter}.json`, payload);
  setProjectBundleOutput(`exported ${actions.length} recent actions as JSON`);
}

async function importRecentActionHistory(file) {
  if (!file) {
    return;
  }

  const raw = await file.text();
  const parsed = JSON.parse(raw);
  const importedActions = Array.isArray(parsed?.actions) ? parsed.actions : [];
  const recents = loadHubRecents();
  recents.actions = mergeProjectActionHistory(recents.actions ?? [], importedActions);
  saveHubRecents(recents);
  setProjectBundleOutput(`imported ${recents.actions.length} recent actions from JSON`);
}

function manageRecentActionHistory(mode) {
  const recents = loadHubRecents();

  switch (mode) {
    case "keep-failed":
      recents.actions = (recents.actions ?? []).filter((entry) => entry.status === "failed");
      saveHubRecents(recents);
      setProjectBundleOutput("kept failed recent actions only");
      return;
    case "import-json":
      elements.historyImportInput?.click();
      return;
    case "clear":
      recents.actions = [];
      saveHubRecents(recents);
      setProjectBundleOutput("cleared recent action history");
      return;
    case "export-json":
      exportRecentActionHistory();
      return;
    default:
      return;
  }
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
  const executedAt = new Date().toISOString();
  const tauriPayload =
    command === "guarded_mutation_action"
      ? {
          action: PROJECT_ACTION_LABELS[action] ? `project_bundle_${action.split(" ").at(-1)}` : "",
          ...payload,
        }
      : payload;

  try {
    const result = await invokeTauri(command, { payload: tauriPayload });
    saveProjectBundleRecents({
      action,
      bundlePath: elements.projectBundlePath?.value,
      comparePath: elements.projectBundleComparePath?.value,
      outputPath: elements.projectBundleOutPath?.value,
      status: "ok",
      note: result,
      executedAt,
    });
    outputTarget(result);
    setBusy(false, "ready");
  } catch (error) {
    const message = String(error);
    saveProjectBundleRecents({
      action,
      bundlePath: elements.projectBundlePath?.value,
      comparePath: elements.projectBundleComparePath?.value,
      outputPath: elements.projectBundleOutPath?.value,
      status: "failed",
      note: message,
      executedAt,
    });
    outputTarget(message);
    setBusy(false, "failed");
  }
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
  return String(text || "")
    .replace(/(authorization\s*[:=]\s*)(bearer\s+)?([^\s]+)/giu, "$1[redacted]")
    .replace(/(api[_-]?key\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(token\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(password\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(secret\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]");
}

async function copyHotRuntimeLogView() {
  const text = sanitizeRuntimeLogForClipboard(
    String(elements.hotRuntimeLogOutput?.textContent || "").trim(),
  );
  await navigator.clipboard.writeText(text);
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
  const text = String(rendered || "");
  const running = /hot-loop:\s+running/i.test(text);
  const stopped = /hot-loop:\s+stopped/i.test(text);
  const modeMatch =
    /started managed hot-reload loop \((cloud|distributed|local)\)/i.exec(text)
    || /Mode\W*(cloud|distributed|local)/i.exec(text);

  return {
    status: running ? "running" : stopped ? "idle" : "unknown",
    mode: modeMatch?.[1] || elements.hotRuntimeMode?.textContent?.trim() || "local",
  };
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
  switch (workflowId) {
    case "workflow.heat-to-thermo-quad-2d":
      return {
        heat_model: {
          nodes: [
            { id: "h0", x: 0.0, y: 0.0, fix_temperature: true, temperature: 100.0, heat_load: 0.0 },
            { id: "h1", x: 1.0, y: 0.0, fix_temperature: false, temperature: 0.0, heat_load: 0.0 },
            { id: "h2", x: 1.0, y: 1.0, fix_temperature: true, temperature: 20.0, heat_load: 0.0 },
            { id: "h3", x: 0.0, y: 1.0, fix_temperature: true, temperature: 20.0, heat_load: 0.0 },
          ],
          elements: [
            {
              id: "hq0",
              node_i: 0,
              node_j: 1,
              node_k: 2,
              node_l: 3,
              thickness: 0.02,
              conductivity: 45.0,
            },
          ],
        },
      };
    default:
      return null;
  }
}

function describeWorkflowSummary(resultPayload) {
  const exported = resultPayload?.result?.artifacts?.["json_output.json"];
  if (!exported?.content) {
    return "no exported summary";
  }

  try {
    const summary = JSON.parse(exported.content);
    return Object.entries(summary)
      .slice(0, 3)
      .map(([key, value]) => `${key}=${value}`)
      .join(" · ");
  } catch (_error) {
    return "exported summary is not valid JSON";
  }
}

async function waitForWorkflowJob(jobId, options = {}) {
  const timeoutMs = Number(options.timeoutMs || 30_000);
  const intervalMs = Number(options.intervalMs || 700);
  const baseUrl = currentOrchestratorBaseUrl().replace(/\/+$/u, "");
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    const response = await fetch(`${baseUrl}/api/v1/jobs/${encodeURIComponent(jobId)}`);
    if (!response.ok) {
      throw new Error(`workflow job lookup failed (${response.status})`);
    }

    const payload = await response.json();
    const status = String(payload?.job?.status || "").trim();
    if (status === "completed") {
      return payload;
    }
    if (status === "failed" || status === "cancelled") {
      throw new Error(
        hubMessage(localizedWorkflowCatalogLabel("workflowCatalogFailed"), {
          workflow: payload?.result?.workflow_id || payload?.job?.job_type || "workflow",
          status,
        }),
      );
    }

    setWorkflowCatalogOutput(
      hubMessage(localizedWorkflowCatalogLabel("workflowCatalogPolling"), { job: jobId }),
    );
    await new Promise((resolve) => window.setTimeout(resolve, intervalMs));
  }

  throw new Error(`workflow job ${jobId} timed out`);
}

async function runWorkflowCatalogSample(entry) {
  const workflowId = String(entry?.id || "").trim();
  const inputArtifacts = builtInWorkflowSampleInputArtifacts(workflowId);
  if (!workflowId || !inputArtifacts) {
    setWorkflowCatalogOutput(
      hubMessage(localizedWorkflowCatalogLabel("workflowCatalogUnsupported"), {
        workflow: workflowId || "unknown",
      }),
    );
    return;
  }

  state.workflowCatalogBusy = true;
  renderWorkflowCatalog();
  applyDesktopState(elements.actionState, "running", { kind: "activity" });

  try {
    const response = await fetch(
      `${currentWorkflowCatalogUrl().replace(/\/+$/u, "")}/${encodeURIComponent(workflowId)}/jobs`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ input_artifacts: inputArtifacts }),
      },
    );

    if (!response.ok) {
      throw new Error(`workflow submit failed (${response.status})`);
    }

    const payload = await response.json();
    const jobId = String(payload?.job?.job_id || "").trim();
    const queuedMessage = hubMessage(localizedWorkflowCatalogLabel("workflowCatalogQueued"), {
      workflow: workflowId,
      job: jobId || "--",
    });
    setWorkflowCatalogOutput(queuedMessage);
    setOperationOutput(queuedMessage);

    if (!jobId) {
      throw new Error("workflow submit returned no job id");
    }

    const resultPayload = await waitForWorkflowJob(jobId);
    const completedNodes = Array.isArray(resultPayload?.result?.completed_nodes)
      ? resultPayload.result.completed_nodes
      : [];
    const completedMessage = hubMessage(localizedWorkflowCatalogLabel("workflowCatalogCompleted"), {
      workflow: workflowId,
      count: completedNodes.length,
      summary: describeWorkflowSummary(resultPayload),
    });
    setWorkflowCatalogOutput(completedMessage);
    setOperationOutput(completedMessage);
    applyDesktopState(elements.actionState, "ready", { kind: "activity" });
  } catch (error) {
    setWorkflowCatalogOutput(formatHubOperatorError(error, {
      actionLabel: "Running this workflow sample",
    }));
    applyDesktopState(elements.actionState, "failed", { kind: "activity" });
  } finally {
    state.workflowCatalogBusy = false;
    renderWorkflowCatalog();
  }
}

async function fetchWorkflowCatalog(options = {}) {
  const silent = options?.silent === true;
  if (state.workflowCatalogBusy) {
    return;
  }

  state.workflowCatalogBusy = true;
  renderWorkflowCatalog();
  if (!silent) {
    setWorkflowCatalogOutput(localizedWorkflowCatalogLabel("workflowCatalogLoading"));
    applyDesktopState(elements.actionState, "running", { kind: "activity" });
  }

  try {
    const response = await fetch(currentWorkflowCatalogUrl());
    if (!response.ok) {
      throw new Error(`workflow catalog fetch failed (${response.status})`);
    }

    const payload = await response.json();
    state.workflowCatalog = Array.isArray(payload?.workflows) ? payload.workflows : [];
    renderWorkflowCatalog();
    if (!silent) {
      const loadedMessage = hubMessage(localizedWorkflowCatalogLabel("workflowCatalogLoaded"), {
        count: state.workflowCatalog.length,
      });
      setWorkflowCatalogOutput(loadedMessage);
      setOperationOutput(loadedMessage);
      applyDesktopState(elements.actionState, "ready", { kind: "activity" });
    }
  } catch (error) {
    if (!silent) {
      setWorkflowCatalogOutput(formatHubOperatorError(error, {
        actionLabel: "Loading the workflow catalog",
      }));
      applyDesktopState(elements.actionState, "failed", { kind: "activity" });
    }
  } finally {
    state.workflowCatalogBusy = false;
    renderWorkflowCatalog();
  }
}

function renderWorkflowCatalog(entries = state.workflowCatalog) {
  if (!elements.workflowCatalogList) {
    return;
  }

  elements.workflowCatalogList.innerHTML = "";
  if (!entries.length) {
    renderEmptyHistoryState(
      elements.workflowCatalogList,
      localizedWorkflowCatalogLabel("workflowCatalogEmpty"),
    );
    return;
  }

  entries.forEach((entry) => {
    const shell = document.createElement("div");
    shell.className = "hub-history-item";

    const summary = document.createElement("div");
    summary.className = "hub-history-item__summary";
    const heading = document.createElement("div");
    heading.className = "hub-history-item__heading";
    appendTextElement(heading, "strong", entry.name || entry.id || "workflow");
    const meta = document.createElement("div");
    meta.className = "hub-history-item__meta";
    appendTextElement(meta, "span", entry.id || "--", "desktop-shell-chip");
    appendTextElement(meta, "span", entry.version || "v1", "desktop-shell-chip");
    heading.appendChild(meta);
    summary.appendChild(heading);
    appendTextElement(summary, "span", entry.summary || "named workflow");
    appendTextElement(
      summary,
      "span",
      hubMessage(localizedWorkflowCatalogLabel("workflowCatalogEntryInputs"), {
        inputs: Object.keys(entry.entry_inputs || {}).join(", ") || "--",
      }),
      "hub-history-item__alias",
    );
    appendTextElement(
      summary,
      "span",
      hubMessage(localizedWorkflowCatalogLabel("workflowCatalogOutputs"), {
        outputs: Array.isArray(entry.output_artifacts) ? entry.output_artifacts.join(", ") : "--",
      }),
      "hub-history-item__provenance",
    );

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";
    const runButton = document.createElement("button");
    runButton.type = "button";
    runButton.className = "desktop-shell-button-ghost";
    runButton.textContent = localizedWorkflowCatalogLabel("workflowCatalogRun");
    runButton.disabled = state.workflowCatalogBusy;
    runButton.addEventListener("click", () => {
      void runWorkflowCatalogSample(entry);
    });
    controls.appendChild(runButton);

    shell.append(summary, controls);
    elements.workflowCatalogList.appendChild(shell);
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
  return { path: elements.projectBundlePath?.value || "" };
}

function currentProjectBundleOutputPayload() {
  return {
    path: elements.projectBundlePath?.value || "",
    out: elements.projectBundleOutPath?.value || "",
  };
}

function currentProjectBundleComparePayload() {
  return {
    leftPath: elements.projectBundlePath?.value || "",
    rightPath: elements.projectBundleComparePath?.value || "",
  };
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
  const snapshot = currentAssistantSnapshot();
  const cards = [];

  if (!snapshot.bundlePath) {
    cards.push({
      id: "bundle-path",
      title: hubDynamic("cardBundlePathTitle"),
      summary: hubDynamic("cardBundlePathSummary"),
      actionLabel: hubCopy().home.quick.bundlesTitle,
      tone: "watch",
      onAction: () => {
        setSection("projects");
        setProjectsPage("bundles");
        elements.projectBundlePath?.focus();
        setProjectBundleOutput(hubDynamic("focusedBundleField"));
      },
    });
  }

  if (!/ready|healthy/i.test(snapshot.runtimeStatus)) {
    cards.push({
      id: "start-local",
      title: hubDynamic("cardStartLocalTitle"),
      summary: hubDynamic("cardStartLocalSummary"),
      actionLabel: hubCopy().shell.startLocal,
      tone: "risk",
      onAction: () => {
        void runAction("start-local");
      },
    });
  }

  if (snapshot.bundlePath) {
    cards.push({
      id: "inspect-bundle",
      title: hubDynamic("cardInspectTitle"),
      summary: hubDynamic("cardInspectSummary"),
      actionLabel: hubCopy().bundles.inspect,
      tone: "good",
      onAction: () => {
        void runAction("project-inspect");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.outputPath) {
    cards.push({
      id: "normalize-bundle",
      title: hubDynamic("cardNormalizeTitle"),
      summary: hubDynamic("cardNormalizeSummary"),
      actionLabel: hubCopy().bundles.normalize,
      tone: "good",
      onAction: () => {
        void runAction("project-normalize");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.comparePath) {
    cards.push({
      id: "diff-bundles",
      title: hubDynamic("cardDiffTitle"),
      summary: hubDynamic("cardDiffSummary"),
      actionLabel: hubCopy().bundles.diff,
      tone: "watch",
      onAction: () => {
        void runAction("project-diff");
      },
    });
  }

  cards.push({
    id: "open-guides",
    title: hubDynamic("cardGuidesTitle"),
    summary: hubDynamic("cardGuidesSummary"),
    actionLabel: hubCopy().home.tabs.guides,
    tone: "watch",
    onAction: () => {
      setSection("projects");
      setProjectsPage("guides");
      setProjectBundleOutput(hubDynamic("focusedGuidesPage"));
    },
  });

  cards.push({
    id: "open-workbench",
    title: hubDynamic("cardWorkbenchTitle"),
    summary: hubDynamic("cardWorkbenchSummary"),
    actionLabel: hubCopy().shell.openWorkbench,
    tone: "good",
    onAction: () => {
      void runAction("open-workbench");
    },
  });

  return cards.slice(0, 5);
}

function renderHubAssistantLocalCards() {
  if (!elements.assistantLocalCards) {
    return;
  }

  const cards = buildHubAssistantLocalCards();
  elements.assistantLocalCards.innerHTML = "";
  if (!cards.length) {
    renderEmptyHistoryState(elements.assistantLocalCards, hubDynamic("assistantNoUrgent"));
    return;
  }

  cards.forEach((card) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    appendAssistantCardHeader(
      article,
      card.title,
      card.tone,
      `desktop-shell-state desktop-shell-state--${
        card.tone === "risk" ? "danger" : card.tone === "watch" ? "warning" : "healthy"
      }`,
    );
    appendTextElement(article, "p", card.summary, "desktop-shell-note");
    const buttonRow = document.createElement("div");
    buttonRow.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = card.actionLabel;
    button.addEventListener("click", card.onAction);
    buttonRow.appendChild(button);
    article.appendChild(buttonRow);
    elements.assistantLocalCards.appendChild(article);
  });
}

function buildLocalGuideContext() {
  const snapshot = currentAssistantSnapshot();
  return {
    section: snapshot.activeSection,
    runtimeReady: /ready|healthy/i.test(snapshot.runtimeStatus),
    hasBundle: Boolean(snapshot.bundlePath),
    hasCompare: Boolean(snapshot.comparePath),
    hasOutput: Boolean(snapshot.outputPath),
    bundlePath: snapshot.bundlePath,
  };
}

function buildLocalGuideResponse(query) {
  const normalized = String(query || "").trim().toLowerCase();
  const context = buildLocalGuideContext();

  if (!normalized) {
    return hubDynamic("assistantPromptEmpty");
  }

  if (/first|start|begin|fresh|what should i do/.test(normalized)) {
    if (!context.runtimeReady) {
      return hubDynamic("guideFirstNoRuntime");
    }
    if (!context.hasBundle) {
      return hubDynamic("guideFirstNoBundle");
    }
    return hubDynamic("guideFirstReady");
  }

  if (/inspect|bundle|validate|normalize|diff|pack|unpack/.test(normalized)) {
    if (!context.hasBundle) {
      return hubDynamic("guideBundleNoPath");
    }
    if (/normalize/.test(normalized) && !context.hasOutput) {
      return hubDynamic("guideNormalizeNoOutput");
    }
    if (/diff/.test(normalized) && !context.hasCompare) {
      return hubDynamic("guideDiffNoCompare");
    }
    return hubDynamic("guideBundleGeneral");
  }

  if (/workbench|analysis|open/.test(normalized)) {
    return hubDynamic("guideWorkbench");
  }

  if (/docs|guide|read|document|manual|help/.test(normalized)) {
    return hubDynamic("guideDocs");
  }

  if (/runtime|stack|agent|hot|observe|log/.test(normalized)) {
    return hubDynamic("guideRuntime");
  }

  if (/catalog|library|workload|remote/.test(normalized)) {
    return hubDynamic("guideLibrary");
  }

  if (/installer|package|packaging|desktop|dmg|build/.test(normalized)) {
    return hubDynamic("guidePackaging");
  }

  if (/partial|failed|error|warning/.test(normalized)) {
    return hubDynamic("guideFailure");
  }

  return hubDynamic("guideFallback");
}

function answerWithLocalGuide() {
  const query = elements.assistantLocalPrompt?.value || "";
  setAssistantLocalOutput(buildLocalGuideResponse(query));
}

function extractAssistantJsonBlock(value) {
  const fenced = value.match(/```json\s*([\s\S]*?)```/i);
  if (fenced?.[1]) {
    return fenced[1].trim();
  }

  const firstBrace = value.indexOf("{");
  const lastBrace = value.lastIndexOf("}");
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    return value.slice(firstBrace, lastBrace + 1);
  }

  return value.trim();
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
  try {
    return new URL(baseUrl).origin.toLowerCase();
  } catch {
    return "";
  }
}

function assistantHostRequiresTrust(baseUrl) {
  try {
    const parsed = new URL(baseUrl);
    const protocol = parsed.protocol.toLowerCase();
    const hostname = parsed.hostname.toLowerCase();
    const isLoopback =
      hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
    return protocol === "https:" && !isLoopback;
  } catch {
    return false;
  }
}

function ensureAssistantHostTrust(baseUrl, apiKey) {
  if (!assistantHostRequiresTrust(baseUrl)) {
    return true;
  }

  const origin = assistantTrustHostOrigin(baseUrl);
  if (!origin) {
    return false;
  }

  if (state.assistantTrustedHosts.has(origin)) {
    return true;
  }

  const approved = window.confirm(
    `This assistant request will send${apiKey ? " your API key and" : ""} the prompt directly to ${origin}.\n\nOnly continue if you trust this host.`,
  );
  if (!approved) {
    return false;
  }

  state.assistantTrustedHosts.add(origin);
  persistHubAssistantTrustedHosts(state.assistantTrustedHosts);
  return true;
}

function ensureRemoteHostTrust(baseUrl, label) {
  if (!assistantHostRequiresTrust(baseUrl)) {
    return true;
  }

  const origin = assistantTrustHostOrigin(baseUrl);
  if (!origin) {
    return false;
  }

  if (state.remoteTrustedHosts.has(origin)) {
    return true;
  }

  const approved = window.confirm(
    `${label} will contact ${origin} directly.\n\nOnly continue if you trust this remote host.`,
  );
  if (!approved) {
    return false;
  }

  state.remoteTrustedHosts.add(origin);
  persistHubTrustedHosts(HUB_REMOTE_TRUSTED_HOSTS_KEY, state.remoteTrustedHosts);
  return true;
}

async function requestHubAssistantPlan() {
  const baseUrl = elements.assistantBaseUrl?.value?.trim() || "";
  const model = elements.assistantModelName?.value?.trim() || "";
  const prompt = elements.assistantPrompt?.value?.trim() || "";
  const apiKey = elements.assistantApiKey?.value?.trim() || "";
  const baseUrlValidation = validateAssistantBaseUrl(baseUrl);

  if (!baseUrlValidation.ok || !model) {
    throw new Error(baseUrlValidation.reason || "Fill in the assistant base URL and model before requesting a plan.");
  }

  if (!ensureAssistantHostTrust(baseUrlValidation.normalized, apiKey)) {
    throw new Error("assistant request cancelled before contacting the configured host");
  }

  const response = await fetch(`${baseUrlValidation.normalized.replace(/\/+$/, "")}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
    },
    body: JSON.stringify({
      model,
      temperature: 0.2,
      response_format: { type: "json_object" },
      messages: [
        {
          role: "system",
          content:
            "You are the Kyuubiki Hub assistant. Return strict JSON with keys summary, rationale, suggested_actions. suggested_actions must be an array of objects with action, payload, reason. Only suggest actions from the provided Hub action catalog. Keep it concise, safe, and onboarding-oriented.",
        },
        {
          role: "user",
          content: JSON.stringify(
            {
              prompt,
              snapshot: currentAssistantSnapshot(),
              action_catalog: HUB_ASSISTANT_ACTIONS,
              local_hints: buildHubAssistantLocalCards().map((card) => ({
                id: card.id,
                title: card.title,
                summary: card.summary,
                actionLabel: card.actionLabel,
              })),
            },
            null,
            2,
          ),
        },
      ],
    }),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`assistant request failed (${response.status}): ${body.slice(0, 240)}`);
  }

  const payload = await response.json();
  const content = payload?.choices?.[0]?.message?.content;
  if (!content) {
    throw new Error("assistant response did not include a message body");
  }

  const parsed = JSON.parse(extractAssistantJsonBlock(content));
  return {
    summary: String(parsed?.summary || ""),
    rationale: String(parsed?.rationale || ""),
    suggested_actions: Array.isArray(parsed?.suggested_actions)
      ? parsed.suggested_actions.map((entry) => ({
          action: String(entry?.action || ""),
          payload: entry && typeof entry.payload === "object" && entry.payload ? entry.payload : {},
          reason: String(entry?.reason || ""),
        }))
      : [],
  };
}

function renderHubAssistantPlan() {
  if (!elements.assistantPlanActions) {
    return;
  }

  const plan = state.assistantPlan;
  elements.assistantPlanActions.innerHTML = "";
  if (!plan) {
    renderEmptyHistoryState(elements.assistantPlanActions, hubDynamic("assistantNoPlan"));
    return;
  }

  const summaryCard = document.createElement("article");
  summaryCard.className = "hub-list__card";
  appendAssistantCardHeader(summaryCard, plan.summary || hubDynamic("modelPlanTitle"), `${plan.suggested_actions.length} actions`);
  appendTextElement(
    summaryCard,
    "p",
    plan.rationale || hubDynamic("noRationale"),
    "desktop-shell-note",
  );
  elements.assistantPlanActions.appendChild(summaryCard);

  if (!plan.suggested_actions.length) {
    renderEmptyHistoryState(elements.assistantPlanActions, hubDynamic("assistantNoExecutable"));
    return;
  }

  plan.suggested_actions.forEach((entry) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    appendAssistantCardHeader(
      article,
      entry.action,
      assistantRiskLevel(entry.action),
      assistantRiskStateClass(assistantRiskLevel(entry.action)),
    );
    appendTextElement(article, "p", entry.reason || hubDynamic("noRationale"), "desktop-shell-note");
    appendTextElement(article, "code", JSON.stringify(entry.payload || {}, null, 2));
    const row = document.createElement("div");
    row.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = hubDynamic("actionRun");
    button.addEventListener("click", () => {
      void executeHubAssistantAction(entry.action, entry.payload || {});
    });
    row.appendChild(button);
    article.appendChild(row);
    elements.assistantPlanActions.appendChild(article);
  });
}

function confirmHubAssistantAction(action, source = "assistant") {
  const risk = assistantRiskLevel(action);
  if (risk === "low") {
    return true;
  }

  const note = source === "plan" ? "model plan action" : "assistant action";
  rememberHubAssistantAudit({ action, risk, status: "prompted", source, note });
  const message =
    risk === "high"
      ? `High-risk ${note}: ${action}\n\nThis may launch builds or rewrite bundle outputs.\n\nContinue?`
      : `Sensitive ${note}: ${action}\n\nPlease confirm before the Hub continues.\n\nContinue?`;
  const approved = window.confirm(message);
  rememberHubAssistantAudit({
    action,
    risk,
    status: approved ? "confirmed" : "cancelled",
    source,
    note,
  });
  return approved;
}

function applyAssistantBundlePayload(payload) {
  if (typeof payload?.path === "string") {
    elements.projectBundlePath.value = payload.path;
  }
  if (typeof payload?.comparePath === "string" || typeof payload?.rightPath === "string") {
    elements.projectBundleComparePath.value = String(payload.comparePath ?? payload.rightPath ?? "");
  }
  if (typeof payload?.out === "string") {
    elements.projectBundleOutPath.value = payload.out;
  }
}

async function executeHubAssistantAction(action, payload = {}, source = "assistant") {
  const risk = assistantRiskLevel(action);
  if (!confirmHubAssistantAction(action, source)) {
    setAssistantOutput(hubDynamic("assistantCancelled", { action }));
    return;
  }

  switch (action) {
    case "hub/focusSection":
      setSection(typeof payload.section === "string" ? payload.section : "projects");
      setAssistantOutput(
        hubDynamic("assistantFocusedSection", {
          section: typeof payload.section === "string" ? payload.section : "projects",
        }),
      );
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "focused Hub section" });
      return;
    case "hub/openWorkbench":
      await runActionWithOptions("open-workbench", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Workbench shell" });
      return;
    case "hub/openInstaller":
      await runActionWithOptions("open-installer", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer shell" });
      return;
    case "hub/openDocsIndex":
      await runActionWithOptions("open-docs-index", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened docs index" });
      return;
    case "hub/openCurrentLineDoc":
      await runActionWithOptions("open-current-line-doc", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened current-line document" });
      return;
    case "hub/openOperationsDoc":
      await runActionWithOptions("open-operations-doc", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened operations guide" });
      return;
    case "hub/openTroubleshootingDoc":
      await runActionWithOptions("open-troubleshooting-doc", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened troubleshooting guide" });
      return;
    case "hub/startLocal":
      await runActionWithOptions("start-local", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "started local stack" });
      return;
    case "hub/validateEnv":
      await runActionWithOptions("validate-env", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated environment" });
      return;
    case "hub/desktopStage":
      await runActionWithOptions("desktop-stage", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "staged desktop release" });
      return;
    case "hub/desktopBuildHost":
      await runActionWithOptions("desktop-build-host", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "built host desktop bundles" });
      return;
    case "hub/desktopVerify":
      await runActionWithOptions("desktop-verify", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "verified desktop release" });
      return;
    case "hub/setBundleContext":
      applyAssistantBundlePayload(payload);
      renderAssistantContext();
      setAssistantOutput(hubDynamic("assistantUpdatedBundle"));
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "updated bundle inputs" });
      return;
    case "hub/projectInspect":
      applyAssistantBundlePayload(payload);
      await runActionWithOptions("project-inspect", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "inspected project bundle" });
      return;
    case "hub/projectValidate":
      applyAssistantBundlePayload(payload);
      await runActionWithOptions("project-validate", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated project bundle" });
      return;
    case "hub/projectNormalize":
      applyAssistantBundlePayload(payload);
      await runActionWithOptions("project-normalize", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "normalized project bundle" });
      return;
    case "hub/projectUnpack":
      applyAssistantBundlePayload(payload);
      await runActionWithOptions("project-unpack", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "unpacked project bundle" });
      return;
    case "hub/projectPack":
      applyAssistantBundlePayload(payload);
      await runActionWithOptions("project-pack", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "packed project bundle" });
      return;
    case "hub/projectDiff":
      applyAssistantBundlePayload(payload);
      await runActionWithOptions("project-diff", { skipConfirmation: true });
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "diffed project bundles" });
      return;
    default:
      rememberHubAssistantAudit({ action, risk, status: "failed", source, note: "unknown assistant action" });
      throw new Error(`Unknown assistant action: ${action}`);
  }
}

async function executeHubAssistantPlan() {
  if (!state.assistantPlan?.suggested_actions?.length) {
    setAssistantOutput(hubDynamic("assistantNoPlanToExecute"));
    return;
  }

  if (!elements.assistantApprovePlan?.checked) {
    setAssistantOutput(hubDynamic("assistantReviewFirst"));
    return;
  }

  for (const entry of state.assistantPlan.suggested_actions) {
    try {
      await executeHubAssistantAction(entry.action, entry.payload || {}, "plan");
    } catch (error) {
      rememberHubAssistantAudit({
        action: entry.action,
        risk: assistantRiskLevel(entry.action),
        status: "failed",
        source: "plan",
        note: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }
  setAssistantOutput(hubDynamic("assistantExecuteCount", { count: state.assistantPlan.suggested_actions.length }));
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
  state.hostPlatform = environment.host_platform;

  if (elements.releasePlatform && !elements.releasePlatform.value) {
    elements.releasePlatform.value = environment.host_platform;
  }

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

async function refreshRuntimeStatus() {
  try {
    const payload = await invokeTauri("service_status");
    setRuntimeStatusOutput(payload.rendered);
    applyDesktopState(elements.localRuntimeStatus, payload.rendered, { kind: "health" });
    applyDesktopState(elements.observeRuntimeStatus, payload.rendered, { kind: "health" });
  } catch (error) {
    const message = String(error);
    setRuntimeStatusOutput(message);
    applyDesktopState(elements.localRuntimeStatus, message, { kind: "health" });
    applyDesktopState(elements.observeRuntimeStatus, message, { kind: "health" });
  }
  renderAssistantContext();
  renderHubAssistantLocalCards();
}

async function refreshHotRuntimeStatus() {
  try {
    const payload = await invokeTauri("hot_service_status");
    setHotRuntimeStatusOutput(payload.rendered);
    const inferred = inferHotRuntimeState(payload.rendered);
    applyDesktopState(elements.hotRuntimeStatus, inferred.status, { kind: "activity" });
    applyDesktopState(elements.observeHotStatus, inferred.status, { kind: "activity" });
    if (elements.hotRuntimeMode) {
      elements.hotRuntimeMode.textContent = inferred.mode;
    }
    if (elements.observeHotMode) {
      elements.observeHotMode.textContent = inferred.mode;
    }
    syncHotRuntimeLogPolling();
    await refreshHotRuntimeLog({ silent: true });
  } catch (error) {
    const message = String(error);
    setHotRuntimeStatusOutput(message);
    applyDesktopState(elements.hotRuntimeStatus, "failed", { kind: "activity" });
    applyDesktopState(elements.observeHotStatus, "failed", { kind: "activity" });
    syncHotRuntimeLogPolling();
  }
}

async function refreshHotRuntimeLog(options = {}) {
  const silent = options?.silent === true;
  const service = currentHotRuntimeLogService();

  if (state.hotLogRefreshInFlight) {
    return;
  }

  state.hotLogRefreshInFlight = true;

  try {
    const payload = await invokeTauri("read_runtime_log", {
      payload: { service },
    });
    const rendered = String(payload?.rendered || "").trim();
    setHotRuntimeLogOutput(rendered || hubDynamic("noLogLines", { service }));
  } catch (error) {
    if (!silent) {
      setHotRuntimeLogOutput(formatHubOperatorError(error, {
        actionLabel: "Reading runtime logs",
        context: "log-read",
        service,
      }));
    }
  } finally {
    state.hotLogRefreshInFlight = false;
  }
}

async function refreshObserveRuntimeLog(options = {}) {
  const silent = options?.silent === true;
  const service = currentObserveRuntimeLogService();

  if (state.runtimeLogRefreshInFlight) {
    return;
  }

  state.runtimeLogRefreshInFlight = true;

  try {
    const payload = await invokeTauri("read_runtime_log", {
      payload: { service },
    });
    const rendered = String(payload?.rendered || "").trim();
    setObserveRuntimeLogOutput(rendered || hubDynamic("noLogLines", { service }));
  } catch (error) {
    if (!silent) {
      setObserveRuntimeLogOutput(formatHubOperatorError(error, {
        actionLabel: "Reading runtime logs",
        context: "log-read",
        service,
      }));
    }
  } finally {
    state.runtimeLogRefreshInFlight = false;
  }
}

async function copyObserveRuntimeLogView() {
  const text = sanitizeRuntimeLogForClipboard(
    String(elements.observeRuntimeLogOutput?.textContent || "").trim(),
  );
  await navigator.clipboard.writeText(text);
}

async function refreshDesktopStatusOutput() {
  try {
    setDesktopStatusOutput(
      await invokeTauri("desktop_status", {
        payload: { platform: elements.releasePlatform?.value || state.hostPlatform },
      }),
    );
  } catch (error) {
    setDesktopStatusOutput(formatHubOperatorError(error, {
      actionLabel: "Refreshing desktop packaging status",
      context: "desktop-status",
    }));
  }
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
    switch (action) {
      case "open-workbench":
        setOperationOutput(await invokeTauri("launch_workbench_gui"));
        setSection("projects");
        setBusy(false, "ready");
        return;
      case "open-installer":
        setOperationOutput(await invokeTauri("launch_installer_gui"));
        setSection("deploy");
        setBusy(false, "ready");
        return;
      case "open-docs-index":
        setOperationOutput(await invokeTauri("open_docs_index"));
        setSection("projects");
        setProjectsPage("guides");
        setBusy(false, "ready");
        return;
      case "open-current-line-doc":
        setOperationOutput(await invokeTauri("open_current_line_doc"));
        setSection("projects");
        setProjectsPage("guides");
        setBusy(false, "ready");
        return;
      case "open-operations-doc":
        setOperationOutput(await invokeTauri("open_operations_doc"));
        setSection("projects");
        setProjectsPage("guides");
        setBusy(false, "ready");
        return;
      case "open-troubleshooting-doc":
        setOperationOutput(await invokeTauri("open_troubleshooting_doc"));
        setSection("projects");
        setProjectsPage("guides");
        setBusy(false, "ready");
        return;
      case "project-inspect":
        await runProjectBundleAction({
          action: "project inspect",
          command: "project_bundle_inspect",
          payload: currentProjectBundlePayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-validate":
        await runProjectBundleAction({
          action: "project validate",
          command: "project_bundle_validate",
          payload: currentProjectBundlePayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-normalize":
        await runProjectBundleAction({
          action: "project normalize",
          command: "guarded_mutation_action",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-unpack":
        await runProjectBundleAction({
          action: "project unpack",
          command: "guarded_mutation_action",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-pack":
        await runProjectBundleAction({
          action: "project pack",
          command: "guarded_mutation_action",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-diff":
        await runProjectBundleAction({
          action: "project diff",
          command: "project_bundle_diff",
          payload: currentProjectBundleComparePayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "workload-register-local":
        await registerCurrentBundleAsWorkload();
        setBusy(false, "ready");
        return;
      case "workload-sync-local":
        await syncLocalControlPlaneWorkloads();
        setBusy(false, "ready");
        return;
      case "workload-sync-remote":
        await syncRemoteWorkloadCatalog();
        setBusy(false, "ready");
        return;
      case "workload-export-library":
        exportHubWorkloadLibrary();
        setBusy(false, "ready");
        return;
      case "workload-import-library":
        elements.workloadImportInput?.click();
        setBusy(false, "idle");
        return;
      case "workload-clear-library":
        clearHubWorkloadLibrary();
        setBusy(false, "ready");
        return;
      case "workflow-catalog-refresh":
        await fetchWorkflowCatalog();
        setBusy(false, "ready");
        return;
      case "start-local":
        setOperationOutput(await invokeGuardedMutation("service_start", { mode: "local" }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-local":
        setOperationOutput(await invokeGuardedMutation("hot_service_start", { mode: "local" }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-cloud":
        setOperationOutput(await invokeGuardedMutation("hot_service_start", { mode: "cloud" }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-distributed":
        setOperationOutput(await invokeGuardedMutation("hot_service_start", { mode: "distributed" }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-refresh-status":
        await refreshHotRuntimeStatus();
        setOperationOutput(hubDynamic("hotStatusRefreshed"));
        setBusy(false, "ready");
        return;
      case "hot-refresh-log":
        await refreshHotRuntimeLog();
        setOperationOutput(hubDynamic("hotLogRefreshed", { service: elements.hotRuntimeLogService?.value || "hot-stack" }));
        setBusy(false, "ready");
        return;
      case "hot-copy-log-view":
        await copyHotRuntimeLogView();
        setOperationOutput(hubDynamic("hotLogCopied", { service: elements.hotRuntimeLogService?.value || "hot-stack" }));
        setBusy(false, "ready");
        return;
      case "observe-refresh-runtime-log":
        await refreshObserveRuntimeLog();
        setOperationOutput(hubDynamic("runtimeLogRefreshed", { service: elements.observeRuntimeLogService?.value || "frontend" }));
        setBusy(false, "ready");
        return;
      case "observe-copy-runtime-log":
        await copyObserveRuntimeLogView();
        setOperationOutput(hubDynamic("runtimeLogCopied", { service: elements.observeRuntimeLogService?.value || "frontend" }));
        setBusy(false, "ready");
        return;
      case "hot-clear-log-view":
        clearHotRuntimeLogView();
        setOperationOutput(hubDynamic("hotLogCleared", { service: elements.hotRuntimeLogService?.value || "hot-stack" }));
        setBusy(false, "idle");
        return;
      case "hot-stop":
        setOperationOutput(await invokeGuardedMutation("hot_service_stop"));
        await refreshHotRuntimeStatus();
        setBusy(false, "idle");
        return;
      case "start-cloud":
        setOperationOutput(await invokeGuardedMutation("service_start", { mode: "cloud" }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "start-distributed":
        setOperationOutput(await invokeGuardedMutation("service_start", { mode: "distributed" }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "restart-local":
        setOperationOutput(await invokeGuardedMutation("service_restart", { mode: "local" }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "stop-stack":
        setOperationOutput(await invokeGuardedMutation("service_stop"));
        await refreshRuntimeStatus();
        setBusy(false, "idle");
        return;
      case "validate-env":
        setOperationOutput(await invokeGuardedMutation("validate_env"));
        setBusy(false, "ready");
        return;
      case "run-doctor": {
        const payload = await invokeTauri("doctor_report");
        setOperationOutput(payload.rendered);
        setBusy(false, "ready");
        return;
      }
      case "desktop-stage":
        setOperationOutput(
          await invokeGuardedMutation("desktop_stage", {
            platform: elements.releasePlatform?.value || state.hostPlatform,
          }),
        );
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      case "desktop-status":
        await refreshDesktopStatusOutput();
        setOperationOutput(hubDynamic("packagingRefreshed"));
        setBusy(false, "ready");
        return;
      case "desktop-verify":
        setOperationOutput(
          await invokeGuardedMutation("desktop_verify", {
            platform: elements.releasePlatform?.value || state.hostPlatform,
          }),
        );
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      case "desktop-build-host":
        setOperationOutput(await invokeGuardedMutation("desktop_build_host"));
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
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

elements.historyFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.historyFilter = button.dataset.historyFilter || "all";
    renderHubRecents();
    setProjectBundleOutput(`filtered recent actions: ${state.historyFilter}`);
  });
});

elements.workloadFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.workloadFilter = button.dataset.workloadFilter || "all";
    renderHubWorkloadLibrary();
    setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
  });
});

elements.workloadFamilyFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.workloadFamilyFilter = button.dataset.workloadFamilyFilter || "all";
    renderHubWorkloadLibrary();
    setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
  });
});

elements.historyManageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    manageRecentActionHistory(button.dataset.historyManage || "");
  });
});

elements.densityToggleButtons.forEach((button) => {
  button.addEventListener("click", () => {
    toggleHubDensityPanel(button.dataset.densityToggle || "");
  });
});

elements.historyImportInput?.addEventListener("change", async (event) => {
  const input = event.currentTarget;
  const file = input?.files?.[0];

  try {
    await importRecentActionHistory(file);
  } catch (error) {
    setProjectBundleOutput(formatHubOperatorError(error, {
      actionLabel: "Importing recent action history",
    }));
  } finally {
    if (input) {
      input.value = "";
    }
  }
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

elements.languageSelect?.addEventListener("change", async (event) => {
  state.language = await saveDesktopLanguagePreference(normalizeDesktopLanguage(event.target.value));
  rerenderLocalizedHubShell();
  window.requestAnimationFrame(() => {
    rerenderLocalizedHubShell();
  });
});

state.language = await loadDesktopLanguagePreference();
rerenderLocalizedHubShell();
await applyBrand();
await loadEnvironment();
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
