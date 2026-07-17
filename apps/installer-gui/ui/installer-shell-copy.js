import { normalizeDesktopLanguage } from "./shared/tauri-bridge.js";
import { createInstallerLanguagePackSupport } from "./installer-language-packs.js";

const installerShellCopy = {
  en: {
    language: "Language",
    platform: "Platform",
    workspace: "Workspace",
    currentMode: "Current mode",
    roleChip: "Bootstrap shell",
    description: "Visual installer for local SQLite workflows, cloud PostgreSQL deployment, and release staging.",
    pwdtStatus: "Pwdt is Workbench-first; Installer keeps only restricted diagnostics.",
    actions: {
      doctor: "Run doctor",
      bootstrap: "Bootstrap workspace",
      serviceStatus: "Check services",
      writeEnv: "Write env",
      validateEnv: "Validate env",
      stageRelease: "Stage release",
      buildInstaller: "Build installer app",
      clearOutput: "Clear",
    },
    tabs: ["Wizard", "Setup", "Integrity", "Updates", "Services", "Remote", "Release", "Output"],
    headings: {
      wizard: "Desktop install wizard",
      setup: "Choose your deployment mode",
      environment: "Environment",
      doctor: "Doctor checks",
      services: "Run the whole stack",
      status: "Current service status",
      logs: "Runtime logs",
      release: "Stage a release directory",
      output: "Installer output",
    },
    completion: "Setup flow updated.",
    ready: "Ready for the next step.",
    restartHint: "Restart already-open desktop shells if they do not refresh.",
  },
  zh: {
    language: "语言",
    platform: "平台",
    workspace: "工作区",
    currentMode: "当前模式",
    roleChip: "引导外壳",
    description: "用于本地 SQLite、云端 PostgreSQL 部署和发布暂存的可视化安装器。",
    pwdtStatus: "Pwdt 以 Workbench 为主；Installer 只保留受限诊断入口。",
    actions: {
      doctor: "运行诊断",
      bootstrap: "引导工作区",
      serviceStatus: "检查服务",
      writeEnv: "写入环境",
      validateEnv: "验证环境",
      stageRelease: "暂存发布",
      buildInstaller: "构建安装器",
      clearOutput: "清空",
    },
    tabs: ["向导", "设置", "完整性", "更新", "服务", "远程", "发布", "输出"],
    headings: {
      wizard: "桌面安装向导",
      setup: "选择部署模式",
      environment: "环境",
      doctor: "诊断检查",
      services: "运行整套服务",
      status: "当前服务状态",
      logs: "运行日志",
      release: "暂存发布目录",
      output: "安装器输出",
    },
    completion: "安装流程已更新。",
    ready: "可以进行下一步。",
    restartHint: "如果已打开的桌面外壳没有刷新，请重启它们。",
  },
  ja: {
    language: "言語",
    platform: "プラットフォーム",
    workspace: "ワークスペース",
    currentMode: "現在のモード",
    roleChip: "ブートストラップシェル",
    description: "ローカル SQLite、クラウド PostgreSQL、リリースステージング用のビジュアルインストーラ。",
    pwdtStatus: "Pwdt は Workbench 主体です。Installer には制限付き診断だけを置きます。",
    actions: {
      doctor: "診断を実行",
      bootstrap: "ワークスペース初期化",
      serviceStatus: "サービス確認",
      writeEnv: "環境を書き込む",
      validateEnv: "環境を検証",
      stageRelease: "リリースを準備",
      buildInstaller: "インストーラをビルド",
      clearOutput: "クリア",
    },
    tabs: ["ウィザード", "設定", "整合性", "更新", "サービス", "リモート", "リリース", "出力"],
    headings: {
      wizard: "デスクトップインストールウィザード",
      setup: "デプロイモードを選択",
      environment: "環境",
      doctor: "診断チェック",
      services: "スタックを実行",
      status: "現在のサービス状態",
      logs: "ランタイムログ",
      release: "リリースディレクトリを準備",
      output: "インストーラ出力",
    },
    completion: "セットアップ手順を更新しました。",
    ready: "次の手順に進めます。",
    restartHint: "すでに開いているデスクトップシェルが更新されない場合は再起動してください。",
  },
  es: {
    language: "Idioma",
    platform: "Plataforma",
    workspace: "Espacio de trabajo",
    currentMode: "Modo actual",
    roleChip: "Shell de arranque",
    description: "Instalador visual para SQLite local, despliegue PostgreSQL en nube y staging de releases.",
    pwdtStatus: "Pwdt vive primero en Workbench; Installer conserva solo diagnósticos restringidos.",
    actions: {
      doctor: "Ejecutar diagnóstico",
      bootstrap: "Preparar workspace",
      serviceStatus: "Comprobar servicios",
      writeEnv: "Escribir entorno",
      validateEnv: "Validar entorno",
      stageRelease: "Preparar release",
      buildInstaller: "Compilar instalador",
      clearOutput: "Limpiar",
    },
    tabs: ["Asistente", "Config", "Integridad", "Updates", "Servicios", "Remoto", "Release", "Salida"],
    headings: {
      wizard: "Asistente de instalación",
      setup: "Elegir modo de despliegue",
      environment: "Entorno",
      doctor: "Chequeos doctor",
      services: "Ejecutar el stack",
      status: "Estado actual del servicio",
      logs: "Logs de runtime",
      release: "Preparar directorio de release",
      output: "Salida del instalador",
    },
    completion: "Flujo de setup actualizado.",
    ready: "Listo para el siguiente paso.",
    restartHint: "Reinicia los shells de escritorio ya abiertos si no se actualizan.",
  },
};

const installerLanguageOptions = [
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
  { value: "ja", label: "日本語" },
  { value: "es", label: "Español" },
  { value: "ar", label: "العربية · Arabic" },
  { value: "bn", label: "বাংলা · Bengali" },
  { value: "cs", label: "Čeština · Czech" },
  { value: "da", label: "Dansk · Danish" },
  { value: "de", label: "Deutsch · German" },
  { value: "el", label: "Ελληνικά · Greek" },
  { value: "fa", label: "فارسی · Persian" },
  { value: "fi", label: "Suomi · Finnish" },
  { value: "fr", label: "Français · French" },
  { value: "he", label: "עברית · Hebrew" },
  { value: "hi", label: "हिन्दी · Hindi" },
  { value: "id", label: "Bahasa Indonesia · Indonesian" },
  { value: "it", label: "Italiano · Italian" },
  { value: "ko", label: "한국어 · Korean" },
  { value: "ms", label: "Bahasa Melayu · Malay" },
  { value: "nl", label: "Nederlands · Dutch" },
  { value: "no", label: "Norsk · Norwegian" },
  { value: "pl", label: "Polski · Polish" },
  { value: "pt-BR", label: "Português (Brasil)" },
  { value: "ro", label: "Română · Romanian" },
  { value: "ru", label: "Русский · Russian" },
  { value: "sv", label: "Svenska · Swedish" },
  { value: "sw", label: "Kiswahili · Swahili" },
  { value: "ta", label: "தமிழ் · Tamil" },
  { value: "th", label: "ไทย · Thai" },
  { value: "tr", label: "Türkçe · Turkish" },
  { value: "uk", label: "Українська · Ukrainian" },
  { value: "ur", label: "اردو · Urdu" },
  { value: "vi", label: "Tiếng Việt · Vietnamese" },
  { value: "zh-TW", label: "繁體中文 · Traditional Chinese" },
];

const { ensureInstallerLanguagePack, lazyShellCopyFor } = createInstallerLanguagePackSupport({
  installerShellCopy,
  normalizeDesktopLanguage,
});

export { ensureInstallerLanguagePack, installerLanguageOptions };

export function installerShellCopyFor(language) {
  const normalized = normalizeDesktopLanguage(language);
  if (installerShellCopy[normalized]) return installerShellCopy[normalized];
  const lazyCopy = lazyShellCopyFor(normalized);
  if (lazyCopy) return lazyCopy;
  if (normalized.toLowerCase().startsWith("zh")) return installerShellCopy.zh;
  return installerShellCopy.en;
}

export function populateInstallerLanguageSelect(select, language) {
  if (!select) return;
  const selected = normalizeDesktopLanguage(language);
  select.replaceChildren(...installerLanguageOptions.map((option) => {
    const element = document.createElement("option");
    element.value = option.value;
    element.textContent = option.label;
    return element;
  }));
  if (!installerLanguageOptions.some((option) => option.value === selected)) {
    const custom = document.createElement("option");
    custom.value = selected;
    custom.textContent = selected;
    select.appendChild(custom);
  }
  select.value = selected;
}
