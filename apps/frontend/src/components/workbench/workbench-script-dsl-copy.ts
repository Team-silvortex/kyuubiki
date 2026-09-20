export type WorkbenchScriptDslCopy = {
  title: string;
  subtitle: string;
  hint: string;
  compile: string;
  run: string;
  reset: string;
  recipe: string;
  macro: string;
};

const en: WorkbenchScriptDslCopy = {
  title: "Frontend DSL",
  subtitle: "Describe wasm Python frontend automation as structured steps, then compile into the Pyodide execution layer.",
  hint: "This DSL uses a stable JSON document format so recording, macros, snippets, and UI contracts can share one bridge.",
  compile: "Compile to script",
  run: "Run DSL",
  reset: "Load template",
  recipe: "Load truss recipe",
  macro: "Use current macro draft",
};

const copyByLanguage: Record<string, WorkbenchScriptDslCopy> = {
  en,
  zh: {
    title: "前端 DSL",
    subtitle: "用结构化步骤描述 wasm Python 前端自动化，再编译到 Pyodide 执行层。",
    hint: "DSL 当前采用稳定 JSON 文档格式，适合作为录制、宏、snippet 和 UI 合约之间的统一桥。",
    compile: "编译到脚本",
    run: "直接运行 DSL",
    reset: "载入模板",
    recipe: "载入桁架闭环",
    macro: "用当前宏草稿填充",
  },
  ja: {
    title: "Frontend DSL",
    subtitle: "構造化ステップで wasm Python のフロントエンド自動化を記述し、Pyodide 実行層へコンパイルします。",
    hint: "DSL は安定した JSON 文書形式を使い、録画・マクロ・スニペット・UI 契約の共通ブリッジになります。",
    compile: "スクリプトへコンパイル",
    run: "DSL を実行",
    reset: "テンプレート読込",
    recipe: "トラス手順を読込",
    macro: "現在のマクロ草稿を使う",
  },
  es: {
    title: "DSL de la interfaz",
    subtitle: "Describe la automatización de la interfaz con Python WASM como pasos estructurados y compílalos para ejecutarlos en Pyodide.",
    hint: "El DSL utiliza un formato JSON estable que conecta grabaciones, macros, fragmentos de código y contratos de la interfaz.",
    compile: "Compilar a script",
    run: "Ejecutar DSL",
    reset: "Cargar plantilla",
    recipe: "Cargar receta de celosía",
    macro: "Usar borrador de macro actual",
  },
  ar: {
    title: "DSL الواجهة الأمامية",
    subtitle: "صِف أتمتة الواجهة الأمامية باستخدام Python WASM كخطوات منظّمة، ثم حوّلها إلى طبقة تنفيذ Pyodide.",
    hint: "تستخدم هذه اللغة صيغة JSON ثابتة لربط التسجيلات ووحدات الماكرو ومقاطع الشيفرة وعقود الواجهة بجسر مشترك.",
    compile: "تحويل إلى برنامج نصي",
    run: "تشغيل DSL",
    reset: "تحميل القالب",
    recipe: "تحميل وصفة الجملون",
    macro: "استخدام مسودة الماكرو الحالية",
  },
  fa: {
    title: "DSL رابط کاربری",
    subtitle: "خودکارسازی رابط کاربری با Python WASM را به‌صورت گام‌های ساخت‌یافته توصیف کنید و سپس برای اجرای Pyodide کامپایل کنید.",
    hint: "این زبان از قالب پایدار JSON استفاده می‌کند تا ضبط‌ها، ماکروها، قطعه‌کدها و قراردادهای رابط کاربری از یک پل مشترک بهره ببرند.",
    compile: "کامپایل به اسکریپت",
    run: "اجرای DSL",
    reset: "بارگذاری الگو",
    recipe: "بارگذاری دستور خرپا",
    macro: "استفاده از پیش‌نویس ماکروی فعلی",
  },
};

const compactLabels: Record<string, Partial<WorkbenchScriptDslCopy>> = {
  bn: { title: "Frontend DSL", compile: "কম্পাইল", run: "রান", reset: "টেমপ্লেট", recipe: "ট্রাস রেসিপি", macro: "ম্যাক্রো খসড়া" },
  cs: { title: "Frontend DSL", compile: "Kompilovat", run: "Spustit", reset: "Šablona", recipe: "Recept truss", macro: "Návrh makra" },
  da: { title: "Frontend DSL", compile: "Kompiler", run: "Kør", reset: "Skabelon", recipe: "Truss-recept", macro: "Makrokladde" },
  de: { title: "Frontend DSL", compile: "Kompilieren", run: "Ausführen", reset: "Vorlage", recipe: "Truss-Rezept", macro: "Makroentwurf" },
  el: {
    title: "DSL διεπαφής",
    subtitle: "Περιγράψτε τον αυτοματισμό της διεπαφής με Python WASM ως δομημένα βήματα και μεταγλωττίστε τα για εκτέλεση στο Pyodide.",
    hint: "Η σταθερή μορφή JSON συνδέει καταγραφή, μακροεντολές, αποσπάσματα κώδικα και συμβάσεις διεπαφής.",
    compile: "Μεταγλώττιση σε σκριπτ", run: "Εκτέλεση DSL", reset: "Φόρτωση προτύπου",
    recipe: "Φόρτωση διαδικασίας δικτυώματος", macro: "Χρήση τρέχοντος προσχεδίου μακροεντολής",
  },
  fi: { title: "Frontend DSL", compile: "Käännä", run: "Aja", reset: "Malli", recipe: "Truss-resepti", macro: "Makroluonnos" },
  fr: { title: "DSL frontend", compile: "Compiler", run: "Exécuter", reset: "Modèle", recipe: "Recette treillis", macro: "Brouillon macro" },
  he: { title: "DSL חזית", compile: "הדר", run: "הרץ", reset: "תבנית", recipe: "מתכון truss", macro: "טיוטת macro" },
  hi: { title: "Frontend DSL", compile: "कंपाइल", run: "चलाएँ", reset: "टेम्पलेट", recipe: "ट्रस रेसिपी", macro: "मैक्रो ड्राफ्ट" },
  id: { title: "Frontend DSL", compile: "Kompilasi", run: "Jalankan", reset: "Template", recipe: "Resep truss", macro: "Draf macro" },
  it: { title: "DSL frontend", compile: "Compila", run: "Esegui", reset: "Template", recipe: "Ricetta truss", macro: "Bozza macro" },
  ko: { title: "Frontend DSL", compile: "컴파일", run: "실행", reset: "템플릿", recipe: "트러스 레시피", macro: "매크로 초안" },
  ms: { title: "Frontend DSL", compile: "Kompil", run: "Jalankan", reset: "Templat", recipe: "Resipi truss", macro: "Draf macro" },
  nl: { title: "Frontend DSL", compile: "Compileren", run: "Uitvoeren", reset: "Sjabloon", recipe: "Truss-recept", macro: "Macroconcept" },
  no: { title: "Frontend DSL", compile: "Kompiler", run: "Kjør", reset: "Mal", recipe: "Truss-oppskrift", macro: "Makrokladd" },
  pl: { title: "Frontend DSL", compile: "Kompiluj", run: "Uruchom", reset: "Szablon", recipe: "Recepta truss", macro: "Szkic makra" },
  "pt-br": { title: "DSL frontend", compile: "Compilar", run: "Executar", reset: "Modelo", recipe: "Receita truss", macro: "Rascunho macro" },
  ro: { title: "DSL frontend", compile: "Compilează", run: "Rulează", reset: "Șablon", recipe: "Rețetă truss", macro: "Ciornă macro" },
  ru: { title: "Frontend DSL", compile: "Собрать", run: "Запустить", reset: "Шаблон", recipe: "Рецепт фермы", macro: "Черновик macro" },
  sv: { title: "Frontend DSL", compile: "Kompilera", run: "Kör", reset: "Mall", recipe: "Truss-recept", macro: "Makroutkast" },
  sw: { title: "Frontend DSL", compile: "Kusanya", run: "Endesha", reset: "Kiolezo", recipe: "Mapishi truss", macro: "Rasimu macro" },
  ta: { title: "Frontend DSL", compile: "தொகு", run: "இயக்கு", reset: "வார்ப்புரு", recipe: "truss செய்முறை", macro: "macro வரைவு" },
  th: { title: "Frontend DSL", compile: "คอมไพล์", run: "รัน", reset: "เทมเพลต", recipe: "สูตร truss", macro: "ร่าง macro" },
  tr: { title: "Frontend DSL", compile: "Derle", run: "Çalıştır", reset: "Şablon", recipe: "Truss tarifi", macro: "Macro taslağı" },
  uk: { title: "Frontend DSL", compile: "Компілювати", run: "Запустити", reset: "Шаблон", recipe: "Рецепт ферми", macro: "Чернетка macro" },
  ur: { title: "Frontend DSL", compile: "کمپائل", run: "چلائیں", reset: "ٹیمپلیٹ", recipe: "truss ترکیب", macro: "macro مسودہ" },
  vi: { title: "Frontend DSL", compile: "Biên dịch", run: "Chạy", reset: "Mẫu", recipe: "Công thức truss", macro: "Nháp macro" },
  "zh-tw": { title: "前端 DSL", compile: "編譯到腳本", run: "執行 DSL", reset: "載入模板", recipe: "載入桁架閉環", macro: "使用目前 macro 草稿" },
};

for (const [language, copy] of Object.entries(compactLabels)) {
  copyByLanguage[language] = {
    ...en,
    subtitle: `${copy.title ?? en.title} · wasm Python · Pyodide`,
    hint: "JSON · macro · snippet · UI contract",
    ...copy,
  };
}

export function getWorkbenchScriptDslCopy(language: string): WorkbenchScriptDslCopy {
  const key = language.trim().toLowerCase().replace(/_/g, "-");
  if (Object.hasOwn(copyByLanguage, key)) return copyByLanguage[key];
  const base = key.split("-")[0];
  return Object.hasOwn(copyByLanguage, base) ? copyByLanguage[base] : en;
}
