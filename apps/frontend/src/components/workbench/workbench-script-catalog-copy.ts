import type {
  WorkbenchScriptActionDefinition,
  WorkbenchScriptLanguage,
  WorkbenchScriptMacroDefinition,
  WorkbenchScriptSnippetDefinition,
} from "@/lib/scripting/workbench-script-runtime";

export type WorkbenchScriptCatalogCopy = {
  snippetsMode: string;
  categoryRuntime: string;
  categoryWorkflow: string;
  categoryInspection: string;
  categoryNavigation: string;
  snippetPreset: string;
  parameterJson: string;
  insertConfigured: string;
  savePreset: string;
  emptyPreset: string;
  jsonError: string;
  importPreset: string;
};

type ScriptSummaryCopy = {
  action: string;
  macro: string;
  snippet: string;
  configuredRecipe: string;
};

const en: WorkbenchScriptCatalogCopy = {
  snippetsMode: "Snippets",
  categoryRuntime: "Runtime",
  categoryWorkflow: "Workflow",
  categoryInspection: "Inspection",
  categoryNavigation: "Navigation",
  snippetPreset: "Snippet presets",
  parameterJson: "Parameter JSON",
  insertConfigured: "Insert configured",
  savePreset: "Save preset",
  emptyPreset: "No presets saved for this snippet yet.",
  jsonError: "Parameter JSON is invalid.",
  importPreset: "Import preset",
};

const summaryByLanguage: Record<string, ScriptSummaryCopy> = {
  en: { action: "Action", macro: "Macro", snippet: "Snippet", configuredRecipe: "Runs the configured automation recipe." },
  zh: { action: "动作", macro: "宏", snippet: "配方", configuredRecipe: "运行已配置的自动化配方。" },
  ja: { action: "アクション", macro: "マクロ", snippet: "スニペット", configuredRecipe: "設定済みの自動化レシピを実行します。" },
  es: { action: "Acción", macro: "Macro", snippet: "Receta", configuredRecipe: "Ejecuta la receta de automatización configurada." },
  ar: { action: "إجراء", macro: "ماكرو", snippet: "وصفة", configuredRecipe: "يشغّل وصفة الأتمتة المهيأة." },
  bn: { action: "অ্যাকশন", macro: "ম্যাক্রো", snippet: "স্নিপেট", configuredRecipe: "কনফিগার করা অটোমেশন রেসিপি চালায়।" },
  cs: { action: "Akce", macro: "Makro", snippet: "Snippet", configuredRecipe: "Spustí nastavený automatizační recept." },
  da: { action: "Handling", macro: "Makro", snippet: "Snippet", configuredRecipe: "Kører den konfigurerede automatiseringsopskrift." },
  de: { action: "Aktion", macro: "Makro", snippet: "Snippet", configuredRecipe: "Führt das konfigurierte Automationsrezept aus." },
  el: { action: "Ενέργεια", macro: "Μακροεντολή", snippet: "Snippet", configuredRecipe: "Εκτελεί τη ρυθμισμένη συνταγή αυτοματοποίησης." },
  fa: { action: "کنش", macro: "ماکرو", snippet: "قطعه", configuredRecipe: "دستور خودکارسازی پیکربندی شده را اجرا می کند." },
  fi: { action: "Toiminto", macro: "Makro", snippet: "Snippet", configuredRecipe: "Suorittaa määritetyn automaatioreseptin." },
  fr: { action: "Action", macro: "Macro", snippet: "Snippet", configuredRecipe: "Exécute la recette d'automatisation configurée." },
  he: { action: "פעולה", macro: "מאקרו", snippet: "קטע", configuredRecipe: "מריץ את מתכון האוטומציה שהוגדר." },
  hi: { action: "क्रिया", macro: "मैक्रो", snippet: "स्निपेट", configuredRecipe: "कॉन्फ़िगर की गई ऑटोमेशन रेसिपी चलाता है।" },
  id: { action: "Aksi", macro: "Makro", snippet: "Snippet", configuredRecipe: "Menjalankan resep otomasi yang dikonfigurasi." },
  it: { action: "Azione", macro: "Macro", snippet: "Snippet", configuredRecipe: "Esegue la ricetta di automazione configurata." },
  ko: { action: "동작", macro: "매크로", snippet: "스니펫", configuredRecipe: "구성된 자동화 레시피를 실행합니다." },
  ms: { action: "Tindakan", macro: "Makro", snippet: "Snippet", configuredRecipe: "Menjalankan resipi automasi yang dikonfigurasi." },
  nl: { action: "Actie", macro: "Macro", snippet: "Snippet", configuredRecipe: "Voert het geconfigureerde automatiseringsrecept uit." },
  no: { action: "Handling", macro: "Makro", snippet: "Snippet", configuredRecipe: "Kjører den konfigurerte automatiseringsoppskriften." },
  pl: { action: "Akcja", macro: "Makro", snippet: "Snippet", configuredRecipe: "Uruchamia skonfigurowaną receptę automatyzacji." },
  "pt-br": { action: "Ação", macro: "Macro", snippet: "Snippet", configuredRecipe: "Executa a receita de automação configurada." },
  ro: { action: "Acțiune", macro: "Macro", snippet: "Snippet", configuredRecipe: "Rulează rețeta de automatizare configurată." },
  ru: { action: "Действие", macro: "Макрос", snippet: "Сниппет", configuredRecipe: "Запускает настроенный рецепт автоматизации." },
  sv: { action: "Åtgärd", macro: "Makro", snippet: "Snippet", configuredRecipe: "Kör det konfigurerade automatiseringsreceptet." },
  sw: { action: "Kitendo", macro: "Makro", snippet: "Kipande", configuredRecipe: "Huendesha mapishi ya otomatiki yaliyosanidiwa." },
  ta: { action: "செயல்", macro: "மேக்ரோ", snippet: "துணுக்கு", configuredRecipe: "அமைக்கப்பட்ட தானியக்க செய்முறையை இயக்கும்." },
  th: { action: "การทำงาน", macro: "มาโคร", snippet: "สแนิปเป็ต", configuredRecipe: "เรียกใช้สูตรอัตโนมัติที่กำหนดค่าไว้" },
  tr: { action: "Eylem", macro: "Makro", snippet: "Snippet", configuredRecipe: "Yapılandırılmış otomasyon tarifini çalıştırır." },
  uk: { action: "Дія", macro: "Макрос", snippet: "Сніпет", configuredRecipe: "Запускає налаштований рецепт автоматизації." },
  ur: { action: "عمل", macro: "میکرو", snippet: "سنپٹ", configuredRecipe: "ترتیب دی گئی خودکاری ترکیب چلاتا ہے۔" },
  vi: { action: "Hành động", macro: "Macro", snippet: "Snippet", configuredRecipe: "Chạy công thức tự động hóa đã cấu hình." },
  "zh-tw": { action: "動作", macro: "巨集", snippet: "配方", configuredRecipe: "執行已設定的自動化配方。" },
};

const tokenLabelsByLanguage: Record<string, Record<string, string>> = {
  zh: { click: "点击", open: "打开", set: "设置", select: "选择", update: "更新", delete: "删除", export: "导出", save: "保存", generate: "生成", replace: "替换", refresh: "刷新", cancel: "取消", focus: "聚焦", reset: "重置", toggle: "切换", project: "项目", model: "模型", runtime: "运行时", workflow: "工作流", data: "数据", result: "结果", results: "结果", viewport: "视口", control: "控制", panel: "面板", topology: "拓扑", snapshot: "快照", sidebar: "侧栏", section: "分区", study: "算例", kind: "类型", tabs: "标签", health: "健康", jobs: "任务", truss: "桁架", plane: "平面", version: "版本", review: "复核", filter: "过滤", database: "数据库" },
  "zh-tw": { click: "點擊", open: "開啟", set: "設定", select: "選擇", update: "更新", delete: "刪除", export: "匯出", save: "儲存", generate: "生成", replace: "替換", refresh: "重新整理", cancel: "取消", focus: "聚焦", reset: "重設", toggle: "切換", project: "專案", model: "模型", runtime: "執行時", workflow: "工作流", data: "資料", result: "結果", results: "結果", viewport: "視口", control: "控制", panel: "面板", topology: "拓撲", snapshot: "快照", sidebar: "側欄", section: "分區", study: "算例", kind: "類型", tabs: "分頁", health: "健康", jobs: "任務", truss: "桁架", plane: "平面", version: "版本", review: "複核", filter: "篩選", database: "資料庫" },
  ja: { click: "クリック", open: "開く", set: "設定", select: "選択", update: "更新", delete: "削除", export: "エクスポート", save: "保存", generate: "生成", replace: "置換", refresh: "更新", cancel: "取消", focus: "フォーカス", reset: "リセット", toggle: "切替", project: "プロジェクト", model: "モデル", runtime: "ランタイム", workflow: "ワークフロー", data: "データ", result: "結果", results: "結果", viewport: "ビューポート", control: "制御", panel: "パネル", topology: "トポロジ", snapshot: "スナップショット", sidebar: "サイドバー", section: "セクション", study: "スタディ", kind: "種別", tabs: "タブ", health: "健全性", jobs: "ジョブ", truss: "トラス", plane: "平面", version: "バージョン", review: "レビュー", filter: "フィルタ", database: "データベース" },
  es: { click: "clic", open: "abrir", set: "definir", select: "seleccionar", update: "actualizar", delete: "eliminar", export: "exportar", save: "guardar", generate: "generar", replace: "reemplazar", refresh: "refrescar", cancel: "cancelar", focus: "enfocar", reset: "restablecer", toggle: "alternar", project: "proyecto", model: "modelo", runtime: "runtime", workflow: "flujo de trabajo", data: "datos", result: "resultado", results: "resultados", viewport: "visor", control: "control", panel: "panel", topology: "topología", snapshot: "instantánea", sidebar: "barra lateral", section: "sección", study: "estudio", kind: "tipo", tabs: "pestañas", health: "salud", jobs: "tareas", truss: "cercha", plane: "plano", version: "versión", review: "revisión", filter: "filtro", database: "base de datos" },
  fr: { click: "clic", open: "ouvrir", set: "définir", select: "sélectionner", update: "mettre à jour", delete: "supprimer", export: "exporter", save: "enregistrer", generate: "générer", replace: "remplacer", refresh: "actualiser", cancel: "annuler", focus: "focaliser", reset: "réinitialiser", toggle: "basculer", project: "projet", model: "modèle", runtime: "runtime", workflow: "workflow", data: "données", result: "résultat", results: "résultats", viewport: "vue", control: "contrôle", panel: "panneau", topology: "topologie", snapshot: "instantané", sidebar: "barre latérale", section: "section", study: "étude", kind: "type", tabs: "onglets", health: "santé", jobs: "tâches", truss: "treillis", plane: "plan", version: "version", review: "revue", filter: "filtre", database: "base de données" },
  de: { click: "Klick", open: "öffnen", set: "setzen", select: "auswählen", update: "aktualisieren", delete: "löschen", export: "exportieren", save: "speichern", generate: "erzeugen", replace: "ersetzen", refresh: "aktualisieren", cancel: "abbrechen", focus: "fokussieren", reset: "zurücksetzen", toggle: "umschalten", project: "Projekt", model: "Modell", runtime: "Runtime", workflow: "Workflow", data: "Daten", result: "Ergebnis", results: "Ergebnisse", viewport: "Ansicht", control: "Steuerung", panel: "Panel", topology: "Topologie", snapshot: "Snapshot", sidebar: "Seitenleiste", section: "Bereich", study: "Studie", kind: "Typ", tabs: "Tabs", health: "Status", jobs: "Jobs", truss: "Fachwerk", plane: "Ebene", version: "Version", review: "Prüfung", filter: "Filter", database: "Datenbank" },
  ko: { click: "클릭", open: "열기", set: "설정", select: "선택", update: "업데이트", delete: "삭제", export: "내보내기", save: "저장", generate: "생성", replace: "교체", refresh: "새로고침", cancel: "취소", focus: "초점", reset: "초기화", toggle: "전환", project: "프로젝트", model: "모델", runtime: "런타임", workflow: "워크플로", data: "데이터", result: "결과", results: "결과", viewport: "뷰포트", control: "제어", panel: "패널", topology: "토폴로지", snapshot: "스냅샷", sidebar: "사이드바", section: "섹션", study: "스터디", kind: "종류", tabs: "탭", health: "상태", jobs: "작업", truss: "트러스", plane: "평면", version: "버전", review: "검토", filter: "필터", database: "데이터베이스" },
  ru: { click: "клик", open: "открыть", set: "задать", select: "выбрать", update: "обновить", delete: "удалить", export: "экспорт", save: "сохранить", generate: "создать", replace: "заменить", refresh: "обновить", cancel: "отменить", focus: "фокус", reset: "сброс", toggle: "переключить", project: "проект", model: "модель", runtime: "рантайм", workflow: "процесс", data: "данные", result: "результат", results: "результаты", viewport: "вид", control: "управление", panel: "панель", topology: "топология", snapshot: "снимок", sidebar: "боковая панель", section: "раздел", study: "исследование", kind: "тип", tabs: "вкладки", health: "состояние", jobs: "задания", truss: "ферма", plane: "плоскость", version: "версия", review: "проверка", filter: "фильтр", database: "база данных" },
};

const copyByLanguage: Record<string, WorkbenchScriptCatalogCopy> = {
  en,
  zh: {
    snippetsMode: "配方",
    categoryRuntime: "运行时",
    categoryWorkflow: "工作流",
    categoryInspection: "检查",
    categoryNavigation: "导航",
    snippetPreset: "配方预设",
    parameterJson: "参数 JSON",
    insertConfigured: "按当前参数插入",
    savePreset: "存为预设",
    emptyPreset: "当前项目下还没有这条配方的预设。",
    jsonError: "参数 JSON 无效，无法插入或保存。",
    importPreset: "导入预设",
  },
  ja: {
    snippetsMode: "スニペット",
    categoryRuntime: "ランタイム",
    categoryWorkflow: "ワークフロー",
    categoryInspection: "検査",
    categoryNavigation: "ナビゲーション",
    snippetPreset: "スニペットプリセット",
    parameterJson: "パラメータ JSON",
    insertConfigured: "現在の設定で挿入",
    savePreset: "プリセット保存",
    emptyPreset: "このスニペットのプリセットはまだありません。",
    jsonError: "パラメータ JSON が無効です。",
    importPreset: "プリセット読込",
  },
  es: {
    snippetsMode: "Recetas",
    categoryRuntime: "Runtime",
    categoryWorkflow: "Workflow",
    categoryInspection: "Inspección",
    categoryNavigation: "Navegación",
    snippetPreset: "Presets de receta",
    parameterJson: "JSON de parámetros",
    insertConfigured: "Insertar configurado",
    savePreset: "Guardar preset",
    emptyPreset: "Todavía no hay presets para este snippet.",
    jsonError: "El JSON de parámetros no es válido.",
    importPreset: "Importar preset",
  },
};

const compactCopy: Record<string, Partial<WorkbenchScriptCatalogCopy>> = {
  ar: { snippetsMode: "وصفات", categoryInspection: "فحص", categoryNavigation: "تنقل", insertConfigured: "إدراج", savePreset: "حفظ", importPreset: "استيراد" },
  bn: { snippetsMode: "স্নিপেট", categoryInspection: "পরীক্ষা", categoryNavigation: "নেভিগেশন", insertConfigured: "ইনসার্ট", savePreset: "সেভ", importPreset: "ইমপোর্ট" },
  cs: { snippetsMode: "Snippety", categoryInspection: "Kontrola", categoryNavigation: "Navigace", insertConfigured: "Vložit", savePreset: "Uložit", importPreset: "Import" },
  da: { snippetsMode: "Snippets", categoryInspection: "Inspektion", categoryNavigation: "Navigation", insertConfigured: "Indsæt", savePreset: "Gem", importPreset: "Importer" },
  de: { snippetsMode: "Snippets", categoryInspection: "Prüfung", categoryNavigation: "Navigation", insertConfigured: "Einfügen", savePreset: "Speichern", importPreset: "Importieren" },
  el: { snippetsMode: "Snippets", categoryInspection: "Έλεγχος", categoryNavigation: "Πλοήγηση", insertConfigured: "Εισαγωγή", savePreset: "Αποθήκευση", importPreset: "Εισαγωγή" },
  fa: { snippetsMode: "قطعه ها", categoryInspection: "بازرسی", categoryNavigation: "ناوبری", insertConfigured: "درج", savePreset: "ذخیره", importPreset: "وارد کردن" },
  fi: { snippetsMode: "Snippets", categoryInspection: "Tarkastus", categoryNavigation: "Navigointi", insertConfigured: "Lisää", savePreset: "Tallenna", importPreset: "Tuo" },
  fr: { snippetsMode: "Snippets", categoryInspection: "Inspection", categoryNavigation: "Navigation", insertConfigured: "Insérer", savePreset: "Enregistrer", importPreset: "Importer" },
  he: { snippetsMode: "קטעים", categoryInspection: "בדיקה", categoryNavigation: "ניווט", insertConfigured: "הוסף", savePreset: "שמור", importPreset: "ייבא" },
  hi: { snippetsMode: "स्निपेट", categoryInspection: "निरीक्षण", categoryNavigation: "नेविगेशन", insertConfigured: "डालें", savePreset: "सहेजें", importPreset: "आयात" },
  id: { snippetsMode: "Snippet", categoryInspection: "Inspeksi", categoryNavigation: "Navigasi", insertConfigured: "Sisipkan", savePreset: "Simpan", importPreset: "Impor" },
  it: { snippetsMode: "Snippet", categoryInspection: "Ispezione", categoryNavigation: "Navigazione", insertConfigured: "Inserisci", savePreset: "Salva", importPreset: "Importa" },
  ko: { snippetsMode: "스니펫", categoryInspection: "검사", categoryNavigation: "탐색", insertConfigured: "삽입", savePreset: "저장", importPreset: "가져오기" },
  ms: { snippetsMode: "Snippet", categoryInspection: "Pemeriksaan", categoryNavigation: "Navigasi", insertConfigured: "Sisip", savePreset: "Simpan", importPreset: "Import" },
  nl: { snippetsMode: "Snippets", categoryInspection: "Inspectie", categoryNavigation: "Navigatie", insertConfigured: "Invoegen", savePreset: "Opslaan", importPreset: "Importeren" },
  no: { snippetsMode: "Snippets", categoryInspection: "Inspeksjon", categoryNavigation: "Navigasjon", insertConfigured: "Sett inn", savePreset: "Lagre", importPreset: "Importer" },
  pl: { snippetsMode: "Snippety", categoryInspection: "Inspekcja", categoryNavigation: "Nawigacja", insertConfigured: "Wstaw", savePreset: "Zapisz", importPreset: "Importuj" },
  "pt-br": { snippetsMode: "Snippets", categoryInspection: "Inspeção", categoryNavigation: "Navegação", insertConfigured: "Inserir", savePreset: "Salvar", importPreset: "Importar" },
  ro: { snippetsMode: "Snippeturi", categoryInspection: "Inspecție", categoryNavigation: "Navigare", insertConfigured: "Inserează", savePreset: "Salvează", importPreset: "Importă" },
  ru: { snippetsMode: "Сниппеты", categoryInspection: "Проверка", categoryNavigation: "Навигация", insertConfigured: "Вставить", savePreset: "Сохранить", importPreset: "Импорт" },
  sv: { snippetsMode: "Snippets", categoryInspection: "Inspektion", categoryNavigation: "Navigering", insertConfigured: "Infoga", savePreset: "Spara", importPreset: "Importera" },
  sw: { snippetsMode: "Vipande", categoryInspection: "Ukaguzi", categoryNavigation: "Urambazaji", insertConfigured: "Ingiza", savePreset: "Hifadhi", importPreset: "Ingiza" },
  ta: { snippetsMode: "துணுக்குகள்", categoryInspection: "ஆய்வு", categoryNavigation: "வழிசெலுத்தல்", insertConfigured: "செருகு", savePreset: "சேமி", importPreset: "இறக்குமதி" },
  th: { snippetsMode: "ส्नิปเป็ต", categoryInspection: "ตรวจสอบ", categoryNavigation: "นำทาง", insertConfigured: "แทรก", savePreset: "บันทึก", importPreset: "นำเข้า" },
  tr: { snippetsMode: "Snippetler", categoryInspection: "Denetim", categoryNavigation: "Gezinme", insertConfigured: "Ekle", savePreset: "Kaydet", importPreset: "İçe aktar" },
  uk: { snippetsMode: "Сніпети", categoryInspection: "Перевірка", categoryNavigation: "Навігація", insertConfigured: "Вставити", savePreset: "Зберегти", importPreset: "Імпорт" },
  ur: { snippetsMode: "سنپٹس", categoryInspection: "جانچ", categoryNavigation: "نیویگیشن", insertConfigured: "داخل", savePreset: "محفوظ", importPreset: "درآمد" },
  vi: { snippetsMode: "Snippet", categoryInspection: "Kiểm tra", categoryNavigation: "Điều hướng", insertConfigured: "Chèn", savePreset: "Lưu", importPreset: "Nhập" },
  "zh-tw": { snippetsMode: "配方", categoryInspection: "檢查", categoryNavigation: "導航", insertConfigured: "按目前參數插入", savePreset: "儲存預設", importPreset: "匯入預設" },
};

for (const [language, copy] of Object.entries(compactCopy)) {
  copyByLanguage[language] = {
    ...en,
    categoryRuntime: "Runtime",
    categoryWorkflow: "Workflow",
    snippetPreset: `${copy.snippetsMode ?? en.snippetsMode} preset`,
    parameterJson: "JSON",
    emptyPreset: "∅",
    jsonError: "JSON !",
    ...copy,
  };
}

export function getWorkbenchScriptCatalogCopy(language: WorkbenchScriptLanguage): WorkbenchScriptCatalogCopy {
  return copyByLanguage[language] ?? copyByLanguage[language.toLowerCase()] ?? en;
}

export function getWorkbenchScriptSnippetCategoryLabel(
  category: WorkbenchScriptSnippetDefinition["category"],
  copy: WorkbenchScriptCatalogCopy,
) {
  if (category === "runtime") return copy.categoryRuntime;
  if (category === "workflow") return copy.categoryWorkflow;
  if (category === "inspection") return copy.categoryInspection;
  return copy.categoryNavigation;
}

function normalizeLanguage(language: WorkbenchScriptLanguage) {
  return language.toLowerCase();
}

function splitScriptId(id: string) {
  return id
    .replace(/^snippet\//, "")
    .replace(/^macro\//, "")
    .split(/[./-]/)
    .flatMap((part) => part.replace(/([a-z])([A-Z0-9])/g, "$1 $2").split(/\s+/))
    .filter(Boolean)
    .map((part) => part.toLowerCase());
}

function localizedScriptTitle(id: string, language: WorkbenchScriptLanguage) {
  const normalized = normalizeLanguage(language);
  const labels = tokenLabelsByLanguage[normalized] ?? tokenLabelsByLanguage[normalized.split("-")[0]] ?? {};
  return splitScriptId(id)
    .map((token) => labels[token] ?? token.toUpperCase().replace(/^UI$/, "UI"))
    .join(" ");
}

function getSummaryCopy(language: WorkbenchScriptLanguage) {
  const normalized = normalizeLanguage(language);
  return summaryByLanguage[normalized] ?? summaryByLanguage[normalized.split("-")[0]] ?? summaryByLanguage.en;
}

function localizedCategoryLabel(
  category: WorkbenchScriptActionDefinition["category"] | WorkbenchScriptMacroDefinition["category"],
  language: WorkbenchScriptLanguage,
) {
  const catalogCopy = getWorkbenchScriptCatalogCopy(language);
  if (category === "runtime") return catalogCopy.categoryRuntime;
  if (category === "workflow") return catalogCopy.categoryWorkflow;
  if (category === "inspection") return catalogCopy.categoryInspection;
  if (category === "navigation") return catalogCopy.categoryNavigation;
  return localizedScriptTitle(category, language);
}

export function getWorkbenchScriptActionSummary(
  action: WorkbenchScriptActionDefinition,
  language: WorkbenchScriptLanguage,
) {
  if (language === "zh") return action.summary.zh;
  if (language === "en") return action.summary.en;
  const summary = getSummaryCopy(language);
  return `${summary.action}: ${localizedCategoryLabel(action.category, language)} — ${localizedScriptTitle(action.id, language)}`;
}

export function getWorkbenchScriptMacroSummary(
  macro: WorkbenchScriptMacroDefinition,
  language: WorkbenchScriptLanguage,
) {
  if (language === "zh") return macro.summary.zh;
  if (language === "en") return macro.summary.en;
  const summary = getSummaryCopy(language);
  return `${summary.macro}: ${localizedCategoryLabel(macro.category, language)} — ${localizedScriptTitle(macro.id, language)}`;
}

export function getWorkbenchScriptSnippetTitle(
  snippet: WorkbenchScriptSnippetDefinition,
  language: WorkbenchScriptLanguage,
) {
  if (language === "zh") return snippet.title.zh;
  if (language === "en") return snippet.title.en;
  return localizedScriptTitle(snippet.id, language);
}

export function getWorkbenchScriptSnippetSummary(
  snippet: WorkbenchScriptSnippetDefinition,
  language: WorkbenchScriptLanguage,
) {
  if (language === "zh") return snippet.summary.zh;
  if (language === "en") return snippet.summary.en;
  const summary = getSummaryCopy(language);
  const category = getWorkbenchScriptSnippetCategoryLabel(snippet.category, getWorkbenchScriptCatalogCopy(language));
  return `${summary.snippet}: ${category} — ${summary.configuredRecipe}`;
}
