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
      register: "Register current bundle",
      syncLocal: "Sync local control plane",
      syncRemote: "Sync remote catalog",
    },
    bundles: {
      introLabel: "Bundle operations",
      introTitle: "Project bundle tools",
      introCopy: "Keep the repetitive archive work in one place, then move straight into analysis.",
      inspect: "Inspect .kyuubiki",
      validate: "Validate .kyuubiki",
      normalize: "Normalize bundle",
    },
    guides: {
      primaryLabel: "Primary docs",
      primaryTitle: "Open the right guide",
      primaryCopy: "Start with the docs index, then branch into only the guide that matches the job you are doing now.",
      docsTitle: "Docs index",
      docsCopy: "The single entry to current-line, operations, testing, accuracy, and archived release notes.",
      currentTitle: "Current line",
      currentCopy: "Read what tamamono 1.x is optimizing for before you make deeper product decisions.",
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
      register: "注册当前 bundle",
      syncLocal: "同步本地 control plane",
      syncRemote: "同步远端 catalog",
    },
    bundles: {
      introLabel: "Bundle 操作",
      introTitle: "项目 bundle 工具",
      introCopy: "把重复的归档操作收在一个地方，然后直接继续分析。",
      inspect: "检查 .kyuubiki",
      validate: "验证 .kyuubiki",
      normalize: "规范化 bundle",
    },
    guides: {
      primaryLabel: "主文档",
      primaryTitle: "打开正确的指南",
      primaryCopy: "先从 docs index 开始，再只进入和当前工作匹配的那份指南。",
      docsTitle: "文档索引",
      docsCopy: "current-line、operations、testing、accuracy 和历史 release notes 的统一入口。",
      currentTitle: "当前版本线",
      currentCopy: "先读 tamamono 1.x 现在到底在强化什么，再做更深的产品判断。",
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
      register: "現在の bundle を登録",
      syncLocal: "ローカル control plane を同期",
      syncRemote: "リモート catalog を同期",
    },
    bundles: {
      introLabel: "Bundle 操作",
      introTitle: "プロジェクト bundle ツール",
      introCopy: "繰り返しのアーカイブ作業を一か所に集め、そのまま解析へ進みます。",
      inspect: ".kyuubiki を確認",
      validate: ".kyuubiki を検証",
      normalize: "bundle を正規化",
    },
    guides: {
      primaryLabel: "主要ドキュメント",
      primaryTitle: "正しいガイドを開く",
      primaryCopy: "まず docs index から入り、今の作業に合うガイドだけに進みます。",
      docsTitle: "Docs index",
      docsCopy: "current-line、operations、testing、accuracy、archive をまとめた入口です。",
      currentTitle: "Current line",
      currentCopy: "より深い判断の前に、tamamono 1.x が何を強化しているかを確認します。",
    },
  },
};

const HUB_RECENTS_KEY = "kyuubiki.hub.recents.v1";
const HUB_WORKLOAD_LIBRARY_KEY = "kyuubiki.hub.workloads.v1";
const HUB_ASSISTANT_SETTINGS_KEY = "kyuubiki.hub.assistant.settings.v1";
const HUB_ASSISTANT_SECRETS_KEY = "kyuubiki.hub.assistant.secrets.v1";
const HUB_ASSISTANT_AUDIT_KEY = "kyuubiki.hub.assistant.audit.v1";
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
  assistantMode: "local",
  assistantPlan: null,
  hotLogRefreshInFlight: false,
  runtimeLogRefreshInFlight: false,
  density: { ...HUB_DENSITY_DEFAULTS },
  releaseVersion: "",
  releaseCodename: "",
  language: "en",
};

let hotRuntimeLogPollHandle = null;
let observeRuntimeLogPollHandle = null;

function hubCopy() {
  return HUB_I18N[state.language] || HUB_I18N.en;
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
  homeActionStart: document.getElementById("home-action-start"),
  homeActionSync: document.getElementById("home-action-sync"),
  homeActionOpen: document.getElementById("home-action-open"),
  libraryIntroLabel: document.getElementById("library-intro-label"),
  libraryIntroTitle: document.getElementById("library-intro-title"),
  libraryIntroCopy: document.getElementById("library-intro-copy"),
  libraryActionRegister: document.getElementById("library-action-register"),
  libraryActionSyncLocal: document.getElementById("library-action-sync-local"),
  libraryActionSyncRemote: document.getElementById("library-action-sync-remote"),
  bundlesIntroLabel: document.getElementById("bundles-intro-label"),
  bundlesIntroTitle: document.getElementById("bundles-intro-title"),
  bundlesIntroCopy: document.getElementById("bundles-intro-copy"),
  bundlesActionInspect: document.getElementById("bundles-action-inspect"),
  bundlesActionValidate: document.getElementById("bundles-action-validate"),
  bundlesActionNormalize: document.getElementById("bundles-action-normalize"),
  guidesPrimaryLabel: document.getElementById("guides-primary-label"),
  guidesPrimaryTitle: document.getElementById("guides-primary-title"),
  guidesPrimaryCopy: document.getElementById("guides-primary-copy"),
  guidesDocsTitle: document.getElementById("guides-docs-title"),
  guidesDocsCopy: document.getElementById("guides-docs-copy"),
  guidesCurrentTitle: document.getElementById("guides-current-title"),
  guidesCurrentCopy: document.getElementById("guides-current-copy"),
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
  assistantEngineState: document.getElementById("assistant-engine-state"),
  assistantContextSection: document.getElementById("assistant-context-section"),
  assistantContextRuntime: document.getElementById("assistant-context-runtime"),
  assistantContextBundle: document.getElementById("assistant-context-bundle"),
  assistantLocalPanel: document.getElementById("assistant-local-panel"),
  assistantLocalCards: document.getElementById("assistant-local-cards"),
  assistantLocalPrompt: document.getElementById("assistant-local-prompt"),
  assistantLocalAsk: document.getElementById("assistant-local-ask"),
  assistantLocalOutput: document.getElementById("assistant-local-output"),
  assistantLlmPanel: document.getElementById("assistant-llm-panel"),
  assistantBaseUrl: document.getElementById("assistant-base-url"),
  assistantApiKey: document.getElementById("assistant-api-key"),
  assistantModelPreset: document.getElementById("assistant-model-preset"),
  assistantModelName: document.getElementById("assistant-model-name"),
  assistantPrompt: document.getElementById("assistant-prompt"),
  assistantEndpointPolicy: document.getElementById("assistant-endpoint-policy"),
  assistantRequestPlan: document.getElementById("assistant-request-plan"),
  assistantApprovePlan: document.getElementById("assistant-approve-plan"),
  assistantExecutePlan: document.getElementById("assistant-execute-plan"),
  assistantPlanActions: document.getElementById("assistant-plan-actions"),
  assistantOutput: document.getElementById("assistant-output"),
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
  setText(elements.homeActionStart, copy.home.actions.start);
  setText(elements.homeActionSync, copy.home.actions.sync);
  setText(elements.homeActionOpen, copy.home.actions.open);
  setText(elements.libraryIntroLabel, copy.library.introLabel);
  setText(elements.libraryIntroTitle, copy.library.introTitle);
  setText(elements.libraryIntroCopy, copy.library.introCopy);
  setText(elements.libraryActionRegister, copy.library.register);
  setText(elements.libraryActionSyncLocal, copy.library.syncLocal);
  setText(elements.libraryActionSyncRemote, copy.library.syncRemote);
  setText(elements.bundlesIntroLabel, copy.bundles.introLabel);
  setText(elements.bundlesIntroTitle, copy.bundles.introTitle);
  setText(elements.bundlesIntroCopy, copy.bundles.introCopy);
  setText(elements.bundlesActionInspect, copy.bundles.inspect);
  setText(elements.bundlesActionValidate, copy.bundles.validate);
  setText(elements.bundlesActionNormalize, copy.bundles.normalize);
  setText(elements.guidesPrimaryLabel, copy.guides.primaryLabel);
  setText(elements.guidesPrimaryTitle, copy.guides.primaryTitle);
  setText(elements.guidesPrimaryCopy, copy.guides.primaryCopy);
  setText(elements.guidesDocsTitle, copy.guides.docsTitle);
  setText(elements.guidesDocsCopy, copy.guides.docsCopy);
  setText(elements.guidesCurrentTitle, copy.guides.currentTitle);
  setText(elements.guidesCurrentCopy, copy.guides.currentCopy);
  renderPanelLanguage(copy);
  setSection(state.activeSection);
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

function loadHubAssistantSecrets() {
  try {
    const raw = window.sessionStorage.getItem(HUB_ASSISTANT_SECRETS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return {
      apiKey: String(parsed?.apiKey || ""),
    };
  } catch {
    return { apiKey: "" };
  }
}

function persistHubAssistantSecrets(secrets) {
  window.sessionStorage.setItem(HUB_ASSISTANT_SECRETS_KEY, JSON.stringify(secrets));
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
      "No managed workloads yet. Register a current bundle or sync a remote catalog.",
    );
    return;
  }

  const filteredEntries = entries.filter((entry) => matchesWorkloadFilter(entry));
  if (!filteredEntries.length) {
    const domainLabel = state.workloadFilter === "all" ? "all domains" : state.workloadFilter;
    const familyLabel = state.workloadFamilyFilter === "all" ? "all families" : state.workloadFamilyFilter;
    renderEmptyHistoryState(
      elements.workloadLibraryList,
      `No workloads match ${domainLabel} / ${familyLabel}.`,
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
      setWorkloadLibraryOutput(`restored workload context for ${entry.label}`);
      renderAssistantContext();
      renderHubAssistantLocalCards();
    });

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";

    const useButton = document.createElement("button");
    useButton.type = "button";
    useButton.className = "desktop-shell-button-ghost";
    useButton.textContent = "Use";
    useButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
      }
      setWorkloadLibraryOutput(`loaded ${entry.label} into the bundle path`);
      renderAssistantContext();
      renderHubAssistantLocalCards();
    });

    const workbenchButton = document.createElement("button");
    workbenchButton.type = "button";
    workbenchButton.className = "desktop-shell-button-ghost";
    workbenchButton.textContent = "Open in Workbench";
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
    inspectButton.textContent = "Inspect";
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
    validateButton.textContent = "Validate";
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
    downloadButton.textContent = "Download";
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
    attachButton.textContent = entry.bundlePath ? "Reattach bundle" : "Attach current bundle";
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
    removeButton.textContent = "Remove";
    removeButton.addEventListener("click", () => {
      const next = loadHubWorkloadLibrary().filter((candidate) => workloadIdentity(candidate) !== workloadIdentity(entry));
      saveHubWorkloadLibrary(next);
      setWorkloadLibraryOutput(`removed ${entry.label} from the workload library`);
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
    empty.textContent = "No recent entries yet.";
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
    renderEmptyHistoryState(elements.favoriteActionList, "No favorite actions yet.");
    renderEmptyHistoryState(elements.recentActionList, "No recent project actions yet.");
    return;
  }

  if (!filteredActions.length) {
    renderEmptyHistoryState(elements.favoriteActionList, `No favorites match the ${state.historyFilter} filter.`);
    renderEmptyHistoryState(elements.recentActionList, `No actions match the ${state.historyFilter} filter.`);
    return;
  }

  if (!favoriteActions.length) {
    renderEmptyHistoryState(elements.favoriteActionList, "No pinned favorites yet.");
  } else {
    renderProjectActionEntries(elements.favoriteActionList, favoriteActions);
  }

  if (!recentActions.length) {
    renderEmptyHistoryState(elements.recentActionList, "No non-pinned actions in this view.");
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
      setProjectBundleOutput(`restored ${entry.action} context`);
    });

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";

    const restoreButton = document.createElement("button");
    restoreButton.type = "button";
    restoreButton.className = "desktop-shell-button-ghost";
    restoreButton.textContent = "Restore";
    restoreButton.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      setProjectBundleOutput(`restored ${entry.action} context`);
    });

    const rerunButton = document.createElement("button");
    rerunButton.type = "button";
    rerunButton.className = "desktop-shell-button-primary";
    rerunButton.textContent = "Re-run";
    rerunButton.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      void rerunProjectActionEntry(entry);
    });

    const pinButton = document.createElement("button");
    pinButton.type = "button";
    pinButton.className = entry.pinned ? "desktop-shell-button-primary" : "desktop-shell-button-ghost";
    pinButton.textContent = entry.pinned ? "Pinned" : "Pin";
    pinButton.addEventListener("click", () => {
      togglePinnedProjectAction(entry);
    });

    controls.append(restoreButton);

    if (entry.pinned) {
      const renameButton = document.createElement("button");
      renameButton.type = "button";
      renameButton.className = "desktop-shell-button-ghost";
      renameButton.textContent = "Label";
      renameButton.addEventListener("click", () => {
        renamePinnedProjectAction(entry);
      });
      controls.append(renameButton);

      const copyButton = document.createElement("button");
      copyButton.type = "button";
      copyButton.className = "desktop-shell-button-ghost";
      copyButton.textContent = "Copy CLI";
      copyButton.addEventListener("click", () => {
        void copyProjectCliCommand(entry);
      });
      controls.append(copyButton);

      const pythonButton = document.createElement("button");
      pythonButton.type = "button";
      pythonButton.className = "desktop-shell-button-ghost";
      pythonButton.textContent = "Copy Python";
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

  try {
    const result = await invokeTauri(command, { payload });
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
      title: "Start with a bundle path",
      summary: "Paste a .kyuubiki path first so the Hub can inspect, validate, or normalize it safely.",
      actionLabel: "Open Bundle tools",
      tone: "watch",
      onAction: () => {
        setSection("projects");
        setProjectsPage("bundles");
        elements.projectBundlePath?.focus();
        setProjectBundleOutput("focused the bundle path field");
      },
    });
  }

  if (!/ready|healthy/i.test(snapshot.runtimeStatus)) {
    cards.push({
      id: "start-local",
      title: "Bring the local stack online",
      summary: "The Hub does not currently see a healthy local runtime, so starting the local stack is the safest next step.",
      actionLabel: "Start local stack",
      tone: "risk",
      onAction: () => {
        void runAction("start-local");
      },
    });
  }

  if (snapshot.bundlePath) {
    cards.push({
      id: "inspect-bundle",
      title: "Inspect the selected bundle",
      summary: "Inspecting first gives a quick structural read before we normalize, unpack, or diff anything.",
      actionLabel: "Inspect bundle",
      tone: "good",
      onAction: () => {
        void runAction("project-inspect");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.outputPath) {
    cards.push({
      id: "normalize-bundle",
      title: "Normalize into the target path",
      summary: "You already have both the source and output path, so normalization is ready to run.",
      actionLabel: "Normalize bundle",
      tone: "good",
      onAction: () => {
        void runAction("project-normalize");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.comparePath) {
    cards.push({
      id: "diff-bundles",
      title: "Compare the current pair",
      summary: "Both bundle inputs are present, so the Hub can run a safe diff without more setup.",
      actionLabel: "Diff bundles",
      tone: "watch",
      onAction: () => {
        void runAction("project-diff");
      },
    });
  }

  cards.push({
    id: "open-guides",
    title: "Keep the docs shelf nearby",
    summary: "If you are still orienting yourself, the Guides page is the cleanest single entry to current-line, operations, troubleshooting, and accuracy notes.",
    actionLabel: "Open guides",
    tone: "watch",
    onAction: () => {
      setSection("projects");
      setProjectsPage("guides");
      setProjectBundleOutput("focused the guides page");
    },
  });

  cards.push({
    id: "open-workbench",
    title: "Jump into Workbench",
    summary: "Open the modeling and analysis surface when you are ready to move past bundle-level prep.",
    actionLabel: "Open Workbench",
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
    renderEmptyHistoryState(elements.assistantLocalCards, "The local guide does not see an urgent next step right now.");
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
    return "Ask something short, like: what should I do first, how do I inspect a bundle, how do I open Workbench, or why is packaging still partial.";
  }

  if (/first|start|begin|fresh|what should i do/.test(normalized)) {
    if (!context.runtimeReady) {
      return "Start with the local stack, then sync or register work, inspect once, and only then open Workbench. Right now the local runtime does not look ready, so `Start local stack` is the safest first move.";
    }
    if (!context.hasBundle) {
      return "Start with the local stack if needed, then open `Bundle tools` and paste a `.kyuubiki` path. After that, inspect once and move into Workbench.";
    }
    return "You already have a runtime and bundle context. The safe path now is: inspect the current bundle, confirm the result looks sane, then open Workbench.";
  }

  if (/inspect|bundle|validate|normalize|diff|pack|unpack/.test(normalized)) {
    if (!context.hasBundle) {
      return "Bundle operations live under `Home > Bundle tools`. Paste a `.kyuubiki` bundle path first. Then use `Inspect` for a quick read, `Validate` for schema checks, and `Normalize` only when you also have an output path.";
    }
    if (/normalize/.test(normalized) && !context.hasOutput) {
      return "Normalization needs both a bundle path and an output path. You already have the bundle, so the missing piece is the output destination in `Bundle tools`.";
    }
    if (/diff/.test(normalized) && !context.hasCompare) {
      return "Bundle diff needs both the current bundle and a compare path. Fill the compare field in `Bundle tools`, then run `Diff bundles`.";
    }
    return "Use `Inspect` first for a safe structural read. Use `Validate` when you want schema confidence, `Normalize` when you want a cleaned output bundle, and `Diff` only after both bundle paths are filled.";
  }

  if (/workbench|analysis|open/.test(normalized)) {
    return "Open Workbench only after the runtime is healthy and the bundle context looks sane. In Hub, the short path is: `Home > Start here`, then `Open workbench`.";
  }

  if (/docs|guide|read|document|manual|help/.test(normalized)) {
    return "Use `Home > Guides` as the single documentation shelf. Start with `Docs index`, then open `Current line`, `Operations`, or `Troubleshooting` only when you know which kind of question you are answering.";
  }

  if (/runtime|stack|agent|hot|observe|log/.test(normalized)) {
    return "Use `Runtimes` when you want to change the loop, and `Observe` when you only want to scan or copy state. `Local runtime` is the short health read, `Hot loop` is for dev tails, and `Stack watch` is for sanitized runtime logs.";
  }

  if (/catalog|library|workload|remote/.test(normalized)) {
    return "Use `Home > Library` for workload intake. `Sync local control plane` pulls first-party work in, `Sync remote catalog` brings remote entries in, and the domain/family filters help you narrow the shelf before opening Workbench.";
  }

  if (/installer|package|packaging|desktop|dmg|build/.test(normalized)) {
    return "Use `Installer` when you need release layout or workstation bootstrap. In Hub, `Tools > Packages` is for build actions, `Status` is the readiness wall, and `Output` is where the packaging logs land. In this automation session, `.app` bundles are reliable, while `.dmg` can still show as partial because `hdiutil` is session-sensitive.";
  }

  if (/partial|failed|error|warning/.test(normalized)) {
    return "If something looks partial, read the shortest surface first: `Observe > Health` for runtime issues, `Tools > Status` for desktop packaging readiness, and `Bundle tools > Inspect` for project bundle shape. Then decide whether the problem is runtime, bundle, or packaging.";
  }

  return "The local guide can help with first steps, bundle inspection, runtime health, Workbench launch, workload library intake, and desktop packaging. Try asking one of those directly.";
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
    elements.assistantEndpointPolicy.textContent =
      "Use https:// for remote providers, or http://localhost / 127.0.0.1 for local gateways. The API key is sent directly to the configured base URL.";
    return;
  }

  const validation = validateAssistantBaseUrl(baseUrl);
  if (!validation.ok) {
    elements.assistantEndpointPolicy.textContent = `${validation.reason} The API key is sent directly to the configured base URL.`;
    return;
  }

  elements.assistantEndpointPolicy.textContent =
    "Assistant endpoint looks allowed. The API key is sent directly to the configured base URL for plan generation.";
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
    renderEmptyHistoryState(elements.assistantPlanActions, "No model plan yet.");
    return;
  }

  const summaryCard = document.createElement("article");
  summaryCard.className = "hub-list__card";
  appendAssistantCardHeader(summaryCard, plan.summary || "Model plan", `${plan.suggested_actions.length} actions`);
  appendTextElement(
    summaryCard,
    "p",
    plan.rationale || "The connected model returned a concise operational plan.",
    "desktop-shell-note",
  );
  elements.assistantPlanActions.appendChild(summaryCard);

  if (!plan.suggested_actions.length) {
    renderEmptyHistoryState(elements.assistantPlanActions, "The model returned no executable Hub actions.");
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
    appendTextElement(article, "p", entry.reason || "No rationale supplied.", "desktop-shell-note");
    appendTextElement(article, "code", JSON.stringify(entry.payload || {}, null, 2));
    const row = document.createElement("div");
    row.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = "Run action";
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
    setAssistantOutput(`Cancelled ${action}.`);
    return;
  }

  switch (action) {
    case "hub/focusSection":
      setSection(typeof payload.section === "string" ? payload.section : "projects");
      setAssistantOutput(`Focused ${typeof payload.section === "string" ? payload.section : "projects"} section.`);
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "focused Hub section" });
      return;
    case "hub/openWorkbench":
      await runAction("open-workbench");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Workbench shell" });
      return;
    case "hub/openInstaller":
      await runAction("open-installer");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer shell" });
      return;
    case "hub/openDocsIndex":
      await runAction("open-docs-index");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened docs index" });
      return;
    case "hub/openCurrentLineDoc":
      await runAction("open-current-line-doc");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened current-line document" });
      return;
    case "hub/openOperationsDoc":
      await runAction("open-operations-doc");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened operations guide" });
      return;
    case "hub/openTroubleshootingDoc":
      await runAction("open-troubleshooting-doc");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened troubleshooting guide" });
      return;
    case "hub/startLocal":
      await runAction("start-local");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "started local stack" });
      return;
    case "hub/validateEnv":
      await runAction("validate-env");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated environment" });
      return;
    case "hub/desktopStage":
      await runAction("desktop-stage");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "staged desktop release" });
      return;
    case "hub/desktopBuildHost":
      await runAction("desktop-build-host");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "built host desktop bundles" });
      return;
    case "hub/desktopVerify":
      await runAction("desktop-verify");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "verified desktop release" });
      return;
    case "hub/setBundleContext":
      applyAssistantBundlePayload(payload);
      renderAssistantContext();
      setAssistantOutput("Updated bundle context in the Hub.");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "updated bundle inputs" });
      return;
    case "hub/projectInspect":
      applyAssistantBundlePayload(payload);
      await runAction("project-inspect");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "inspected project bundle" });
      return;
    case "hub/projectValidate":
      applyAssistantBundlePayload(payload);
      await runAction("project-validate");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated project bundle" });
      return;
    case "hub/projectNormalize":
      applyAssistantBundlePayload(payload);
      await runAction("project-normalize");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "normalized project bundle" });
      return;
    case "hub/projectUnpack":
      applyAssistantBundlePayload(payload);
      await runAction("project-unpack");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "unpacked project bundle" });
      return;
    case "hub/projectPack":
      applyAssistantBundlePayload(payload);
      await runAction("project-pack");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "packed project bundle" });
      return;
    case "hub/projectDiff":
      applyAssistantBundlePayload(payload);
      await runAction("project-diff");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "diffed project bundles" });
      return;
    default:
      rememberHubAssistantAudit({ action, risk, status: "failed", source, note: "unknown assistant action" });
      throw new Error(`Unknown assistant action: ${action}`);
  }
}

async function executeHubAssistantPlan() {
  if (!state.assistantPlan?.suggested_actions?.length) {
    setAssistantOutput("No assistant plan is available to execute.");
    return;
  }

  if (!elements.assistantApprovePlan?.checked) {
    setAssistantOutput("Review the generated plan and confirm execution first.");
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
  setAssistantOutput(`Executed ${state.assistantPlan.suggested_actions.length} assistant actions.`);
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
  persistHubAssistantSettings({
    mode: state.assistantMode,
    baseUrl: elements.assistantBaseUrl?.value || "",
    modelPreset: elements.assistantModelPreset?.value || "gpt-5",
    model: elements.assistantModelName?.value || "gpt-5",
  });
  persistHubAssistantSecrets({
    apiKey: elements.assistantApiKey?.value || "",
  });
}

function applyAssistantSettings() {
  const settings = loadHubAssistantSettings();
  const secrets = loadHubAssistantSecrets();
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
    elements.assistantApiKey.value = secrets.apiKey;
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
    setHotRuntimeLogOutput(rendered || `No log lines yet for ${service}.`);
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
    setObserveRuntimeLogOutput(rendered || `No log lines yet for ${service}.`);
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
  if (state.isBusy) {
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
          command: "project_bundle_normalize",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-unpack":
        await runProjectBundleAction({
          action: "project unpack",
          command: "project_bundle_unpack",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-pack":
        await runProjectBundleAction({
          action: "project pack",
          command: "project_bundle_pack",
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
      case "start-local":
        setOperationOutput(await invokeTauri("service_start", { payload: { mode: "local" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-local":
        setOperationOutput(await invokeTauri("hot_service_start", { payload: { mode: "local" } }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-cloud":
        setOperationOutput(await invokeTauri("hot_service_start", { payload: { mode: "cloud" } }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-distributed":
        setOperationOutput(await invokeTauri("hot_service_start", { payload: { mode: "distributed" } }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-refresh-status":
        await refreshHotRuntimeStatus();
        setOperationOutput("refreshed hot-reload runtime status");
        setBusy(false, "ready");
        return;
      case "hot-refresh-log":
        await refreshHotRuntimeLog();
        setOperationOutput(`refreshed hot log: ${elements.hotRuntimeLogService?.value || "hot-stack"}`);
        setBusy(false, "ready");
        return;
      case "hot-copy-log-view":
        await copyHotRuntimeLogView();
        setOperationOutput(`copied sanitized hot log tail: ${elements.hotRuntimeLogService?.value || "hot-stack"}`);
        setBusy(false, "ready");
        return;
      case "observe-refresh-runtime-log":
        await refreshObserveRuntimeLog();
        setOperationOutput(`refreshed runtime log: ${elements.observeRuntimeLogService?.value || "frontend"}`);
        setBusy(false, "ready");
        return;
      case "observe-copy-runtime-log":
        await copyObserveRuntimeLogView();
        setOperationOutput(`copied sanitized runtime log tail: ${elements.observeRuntimeLogService?.value || "frontend"}`);
        setBusy(false, "ready");
        return;
      case "hot-clear-log-view":
        clearHotRuntimeLogView();
        setOperationOutput(`cleared hot log view: ${elements.hotRuntimeLogService?.value || "hot-stack"}`);
        setBusy(false, "idle");
        return;
      case "hot-stop":
        setOperationOutput(await invokeTauri("hot_service_stop"));
        await refreshHotRuntimeStatus();
        setBusy(false, "idle");
        return;
      case "start-cloud":
        setOperationOutput(await invokeTauri("service_start", { payload: { mode: "cloud" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "start-distributed":
        setOperationOutput(await invokeTauri("service_start", { payload: { mode: "distributed" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "restart-local":
        setOperationOutput(await invokeTauri("service_restart", { payload: { mode: "local" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "stop-stack":
        setOperationOutput(await invokeTauri("service_stop"));
        await refreshRuntimeStatus();
        setBusy(false, "idle");
        return;
      case "validate-env":
        setOperationOutput(await invokeTauri("validate_env"));
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
          await invokeTauri("desktop_stage", {
            payload: { platform: elements.releasePlatform?.value || state.hostPlatform },
          }),
        );
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      case "desktop-status":
        await refreshDesktopStatusOutput();
        setOperationOutput("refreshed desktop packaging readiness");
        setBusy(false, "ready");
        return;
      case "desktop-verify":
        setOperationOutput(
          await invokeTauri("desktop_verify", {
            payload: { platform: elements.releasePlatform?.value || state.hostPlatform },
          }),
        );
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      case "desktop-build-host":
        setOperationOutput(await invokeTauri("desktop_build_host"));
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
    setOperationOutput(`focused ${button.dataset.projectsPage || "start"} home page`);
  });
});

elements.projectsTargetButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setProjectsPage(button.dataset.projectsTarget || "start");
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`opened ${button.dataset.projectsTarget || "start"} from home`);
  });
});

elements.panelPageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    const group = button.dataset.panelPageGroup || "";
    const page = button.dataset.panelPage || "";
    setPanelPage(group, page);
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`focused ${page} ${group} page`);
  });
});

elements.assistantFab?.addEventListener("click", () => {
  setAssistantPanelOpen(!state.assistantOpen);
  applyDesktopState(elements.actionState, "active", { kind: "activity" });
  setOperationOutput(state.assistantOpen ? "opened assistant panel" : "closed assistant panel");
});

elements.assistantClose?.addEventListener("click", () => {
  setAssistantPanelOpen(false);
  applyDesktopState(elements.actionState, "idle", { kind: "activity" });
  setOperationOutput("closed assistant panel");
});

elements.assistantPanel?.addEventListener("click", (event) => {
  if (event.target !== elements.assistantPanel) {
    return;
  }
  setAssistantPanelOpen(false);
  applyDesktopState(elements.actionState, "idle", { kind: "activity" });
  setOperationOutput("closed assistant panel");
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
  renderDesktopLanguagePreference();
});

state.language = await loadDesktopLanguagePreference();
renderDesktopLanguagePreference();
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
setSection(state.activeSection);
setBusy(false, "idle");
await refreshRuntimeStatus();
await refreshHotRuntimeStatus();
await refreshDesktopStatusOutput();
