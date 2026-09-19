import { describeDesktopLanguage, loadDesktopLanguagePack } from "./shared/language-pack-loader.js";
import { loadInstallerShellTranslation } from "./installer-shell-translations.js";

const PWDT_STATUS_BY_LANGUAGE = {
  ar: "Pwdt يعمل أولا في Workbench؛ يحتفظ Installer بتشخيصات مقيدة فقط.",
  bn: "Pwdt মূলত Workbench-এ চলে; Installer শুধু সীমিত diagnostics রাখে।",
  cs: "Pwdt je primárně ve Workbench; Installer ponechává jen omezenou diagnostiku.",
  da: "Pwdt hører først til i Workbench; Installer beholder kun begrænset diagnostik.",
  de: "Pwdt ist Workbench-first; Installer behält nur eingeschränkte Diagnosen.",
  el: "Το Pwdt είναι πρώτα για το Workbench· το Installer κρατά μόνο περιορισμένα διαγνωστικά.",
  fa: "Pwdt در درجه اول برای Workbench است؛ Installer فقط عیب‌یابی محدود را نگه می‌دارد.",
  fi: "Pwdt kuuluu ensisijaisesti Workbenchiin; Installer pitää vain rajatut diagnostiikat.",
  fr: "Pwdt vit d'abord dans Workbench ; Installer ne garde que des diagnostics restreints.",
  he: "Pwdt מיועד קודם ל-Workbench; ה-Installer שומר רק אבחון מוגבל.",
  hi: "Pwdt पहले Workbench के लिए है; Installer केवल सीमित diagnostics रखता है।",
  id: "Pwdt utama ada di Workbench; Installer hanya menyimpan diagnostik terbatas.",
  it: "Pwdt è prima di tutto in Workbench; Installer mantiene solo diagnostica limitata.",
  ko: "Pwdt는 Workbench 우선입니다. Installer에는 제한된 진단만 둡니다.",
  ms: "Pwdt diutamakan untuk Workbench; Installer hanya menyimpan diagnostik terhad.",
  nl: "Pwdt hoort eerst in Workbench; Installer houdt alleen beperkte diagnostiek.",
  no: "Pwdt hører først hjemme i Workbench; Installer beholder bare begrenset diagnostikk.",
  pl: "Pwdt jest przede wszystkim w Workbench; Installer ma tylko ograniczoną diagnostykę.",
  "pt-BR": "Pwdt fica primeiro no Workbench; o Installer mantém só diagnósticos restritos.",
  ro: "Pwdt este mai întâi în Workbench; Installer păstrează doar diagnosticare limitată.",
  ru: "Pwdt в первую очередь работает в Workbench; Installer хранит только ограниченную диагностику.",
  sv: "Pwdt hör först hemma i Workbench; Installer behåller bara begränsad diagnostik.",
  sw: "Pwdt hutangulia Workbench; Installer huhifadhi uchunguzi uliowekewa mipaka tu.",
  ta: "Pwdt முதலில் Workbench-க்கானது; Installer கட்டுப்படுத்தப்பட்ட diagnostics மட்டும் வைத்திருக்கும்.",
  th: "Pwdt ใช้งานหลักใน Workbench; Installer เก็บไว้เฉพาะ diagnostics แบบจำกัดเท่านั้น",
  tr: "Pwdt öncelikle Workbench içindir; Installer yalnızca kısıtlı tanılamayı tutar.",
  uk: "Pwdt насамперед працює у Workbench; Installer має лише обмежену діагностику.",
  ur: "Pwdt بنیادی طور پر Workbench کے لیے ہے؛ Installer صرف محدود diagnostics رکھتا ہے۔",
  vi: "Pwdt ưu tiên chạy trong Workbench; Installer chỉ giữ chẩn đoán giới hạn.",
  "zh-TW": "Pwdt 以 Workbench 為主；Installer 只保留受限診斷入口。",
};

async function installerShellCopyFromHubPack(pack, baseCopy) {
  const language = typeof pack?.language === "string" ? pack.language : "en";
  const translation = await loadInstallerShellTranslation(language);
  if (!translation) return null;
  return {
    ...baseCopy,
    ...translation,
    pwdtStatus: PWDT_STATUS_BY_LANGUAGE[language] || baseCopy.pwdtStatus,
  };
}

function installerLanguagePackMessage(language, status, pack) {
  const normalized = typeof language === "string" && language.trim() ? language.trim() : "en";
  const label = describeDesktopLanguage(normalized);
  if (status === "builtin") return `${label} is built in.`;
  if (status === "loaded") return `${pack?.name || label} loaded.`;
  return `${label} language pack is not bundled; falling back to English until the pack is installed.`;
}

export function createInstallerLanguagePackSupport({ installerShellCopy, normalizeDesktopLanguage }) {
  const lazyInstallerShellCopy = {};

  async function ensureInstallerLanguagePack(language) {
    const normalized = normalizeDesktopLanguage(language);
    if (installerShellCopy[normalized]) {
      return {
        status: "builtin",
        language: normalized,
        message: installerShellCopy[normalized].ready,
      };
    }
    const result = await loadDesktopLanguagePack("hub", normalized);
    if (result.status === "loaded" && result.pack) {
      try {
        const copy = await installerShellCopyFromHubPack(result.pack, installerShellCopy.en);
        if (!copy) throw new Error("Installer translation is unavailable.");
        lazyInstallerShellCopy[normalized] = copy;
        return { ...result, message: copy.ready };
      } catch (_error) {
        return { status: "invalid", language: normalized, message: installerLanguagePackMessage(normalized, "missing") };
      }
    }
    return {
      ...result,
      message: installerLanguagePackMessage(normalized, result.status, result.pack),
    };
  }

  function lazyShellCopyFor(language) {
    return lazyInstallerShellCopy[normalizeDesktopLanguage(language)] || null;
  }

  return { ensureInstallerLanguagePack, lazyShellCopyFor };
}
