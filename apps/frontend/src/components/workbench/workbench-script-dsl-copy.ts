export type WorkbenchScriptDslCopy = {
  title: string;
  subtitle: string;
  hint: string;
  compile: string;
  run: string;
  reset: string;
  macro: string;
};

const en: WorkbenchScriptDslCopy = {
  title: "Frontend DSL",
  subtitle: "Describe wasm Python frontend automation as structured steps, then compile into the Pyodide execution layer.",
  hint: "This DSL uses a stable JSON document format so recording, macros, snippets, and UI contracts can share one bridge.",
  compile: "Compile to script",
  run: "Run DSL",
  reset: "Load template",
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
    macro: "用当前宏草稿填充",
  },
  ja: {
    title: "Frontend DSL",
    subtitle: "構造化ステップで wasm Python のフロントエンド自動化を記述し、Pyodide 実行層へコンパイルします。",
    hint: "DSL は安定した JSON 文書形式を使い、録画・マクロ・スニペット・UI 契約の共通ブリッジになります。",
    compile: "スクリプトへコンパイル",
    run: "DSL を実行",
    reset: "テンプレート読込",
    macro: "現在のマクロ草稿を使う",
  },
  es: {
    title: "DSL frontend",
    subtitle: "Describe la automatización frontend wasm Python como pasos estructurados y compílala a Pyodide.",
    hint: "El DSL usa JSON estable para conectar grabación, macros, snippets y contratos UI.",
    compile: "Compilar a script",
    run: "Ejecutar DSL",
    reset: "Cargar plantilla",
    macro: "Usar macro actual",
  },
};

const compactLabels: Record<string, Partial<WorkbenchScriptDslCopy>> = {
  ar: { title: "DSL الواجهة", compile: "ترجمة", run: "تشغيل", reset: "قالب", macro: "مسودة macro" },
  bn: { title: "Frontend DSL", compile: "কম্পাইল", run: "রান", reset: "টেমপ্লেট", macro: "ম্যাক্রো খসড়া" },
  cs: { title: "Frontend DSL", compile: "Kompilovat", run: "Spustit", reset: "Šablona", macro: "Návrh makra" },
  da: { title: "Frontend DSL", compile: "Kompiler", run: "Kør", reset: "Skabelon", macro: "Makrokladde" },
  de: { title: "Frontend DSL", compile: "Kompilieren", run: "Ausführen", reset: "Vorlage", macro: "Makroentwurf" },
  el: { title: "Frontend DSL", compile: "Μεταγλώττιση", run: "Εκτέλεση", reset: "Πρότυπο", macro: "Πρόχειρο macro" },
  fa: { title: "DSL رابط", compile: "کامپایل", run: "اجرا", reset: "قالب", macro: "پیش نویس macro" },
  fi: { title: "Frontend DSL", compile: "Käännä", run: "Aja", reset: "Malli", macro: "Makroluonnos" },
  fr: { title: "DSL frontend", compile: "Compiler", run: "Exécuter", reset: "Modèle", macro: "Brouillon macro" },
  he: { title: "DSL חזית", compile: "הדר", run: "הרץ", reset: "תבנית", macro: "טיוטת macro" },
  hi: { title: "Frontend DSL", compile: "कंपाइल", run: "चलाएँ", reset: "टेम्पलेट", macro: "मैक्रो ड्राफ्ट" },
  id: { title: "Frontend DSL", compile: "Kompilasi", run: "Jalankan", reset: "Template", macro: "Draf macro" },
  it: { title: "DSL frontend", compile: "Compila", run: "Esegui", reset: "Template", macro: "Bozza macro" },
  ko: { title: "Frontend DSL", compile: "컴파일", run: "실행", reset: "템플릿", macro: "매크로 초안" },
  ms: { title: "Frontend DSL", compile: "Kompil", run: "Jalankan", reset: "Templat", macro: "Draf macro" },
  nl: { title: "Frontend DSL", compile: "Compileren", run: "Uitvoeren", reset: "Sjabloon", macro: "Macroconcept" },
  no: { title: "Frontend DSL", compile: "Kompiler", run: "Kjør", reset: "Mal", macro: "Makrokladd" },
  pl: { title: "Frontend DSL", compile: "Kompiluj", run: "Uruchom", reset: "Szablon", macro: "Szkic makra" },
  "pt-br": { title: "DSL frontend", compile: "Compilar", run: "Executar", reset: "Modelo", macro: "Rascunho macro" },
  ro: { title: "DSL frontend", compile: "Compilează", run: "Rulează", reset: "Șablon", macro: "Ciornă macro" },
  ru: { title: "Frontend DSL", compile: "Собрать", run: "Запустить", reset: "Шаблон", macro: "Черновик macro" },
  sv: { title: "Frontend DSL", compile: "Kompilera", run: "Kör", reset: "Mall", macro: "Makroutkast" },
  sw: { title: "Frontend DSL", compile: "Kusanya", run: "Endesha", reset: "Kiolezo", macro: "Rasimu macro" },
  ta: { title: "Frontend DSL", compile: "தொகு", run: "இயக்கு", reset: "வார்ப்புரு", macro: "macro வரைவு" },
  th: { title: "Frontend DSL", compile: "คอมไพล์", run: "รัน", reset: "เทมเพลต", macro: "ร่าง macro" },
  tr: { title: "Frontend DSL", compile: "Derle", run: "Çalıştır", reset: "Şablon", macro: "Macro taslağı" },
  uk: { title: "Frontend DSL", compile: "Компілювати", run: "Запустити", reset: "Шаблон", macro: "Чернетка macro" },
  ur: { title: "Frontend DSL", compile: "کمپائل", run: "چلائیں", reset: "ٹیمپلیٹ", macro: "macro مسودہ" },
  vi: { title: "Frontend DSL", compile: "Biên dịch", run: "Chạy", reset: "Mẫu", macro: "Nháp macro" },
  "zh-tw": { title: "前端 DSL", compile: "編譯到腳本", run: "執行 DSL", reset: "載入模板", macro: "使用目前 macro 草稿" },
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
  return copyByLanguage[language] ?? copyByLanguage[language.toLowerCase()] ?? en;
}
