export type WorkbenchLanguagePackSystemCopy = {
  compatibilityExact: string;
  compatibilityLine: string;
  compatibilityMismatch: string;
  compatibilityGeneric: string;
  targetPrefix: string;
  noVersionLine: string;
  noAppVersion: string;
  unsafeRejected: string;
  templateDescription: string;
  templateDownloaded: string;
  noCustomPack: string;
  exported: string;
  invalidJson: string;
  notFound: string;
  imported: string;
  importedMismatch: string;
  removed: string;
};

const en: WorkbenchLanguagePackSystemCopy = {
  compatibilityExact: "Compatibility: exact match for the current Workbench version",
  compatibilityLine: "Compatibility: matches the current moxi version line",
  compatibilityMismatch: "Compatibility: target version does not match the current Workbench",
  compatibilityGeneric: "Compatibility: unscoped pack, applied as a generic override",
  targetPrefix: "Target",
  noVersionLine: "no version line",
  noAppVersion: "no app version",
  unsafeRejected: "Language pack rejected because it contains unsafe UI text.",
  templateDescription: "Start from this template to override Workbench copy and keep version metadata aligned.",
  templateDownloaded: "Language pack template downloaded.",
  noCustomPack: "No custom language pack is installed for the current language yet.",
  exported: "Exported the current language pack.",
  invalidJson: "Invalid language pack JSON.",
  notFound: "Language pack not found.",
  imported: "Language pack imported.",
  importedMismatch: "Language pack imported, but its target version does not fully match the current Workbench.",
  removed: "Language pack removed.",
};

const SYSTEM_COPY: Record<string, WorkbenchLanguagePackSystemCopy> = {
  en,
  zh: {
    compatibilityExact: "兼容性：与当前 Workbench 版本完全匹配",
    compatibilityLine: "兼容性：与当前 moxi 版本线匹配",
    compatibilityMismatch: "兼容性：目标版本与当前 Workbench 不匹配",
    compatibilityGeneric: "兼容性：未声明目标版本，按通用覆盖处理",
    targetPrefix: "目标",
    noVersionLine: "未声明版本线",
    noAppVersion: "未声明应用版本",
    unsafeRejected: "语言包包含不安全的 UI 文案，已拒绝导入。",
    templateDescription: "从这个模板开始覆盖 Workbench 文案，并保留版本线与目标版本元数据。",
    templateDownloaded: "语言包模板已下载。",
    noCustomPack: "当前语言还没有安装自定义语言包。",
    exported: "当前语言包已导出。",
    invalidJson: "语言包 JSON 无效。",
    notFound: "没有找到这个语言包。",
    imported: "语言包已导入。",
    importedMismatch: "语言包已导入，但它的目标版本与当前 Workbench 不完全对齐。",
    removed: "语言包已移除。",
  },
  ja: {
    compatibilityExact: "互換性: 現在の Workbench バージョンに完全一致",
    compatibilityLine: "互換性: 現在の moxi 系統に一致",
    compatibilityMismatch: "互換性: 対象バージョンが現在の Workbench と不一致",
    compatibilityGeneric: "互換性: 対象バージョン未指定の汎用上書き",
    targetPrefix: "対象",
    noVersionLine: "系統未指定",
    noAppVersion: "アプリ版未指定",
    unsafeRejected: "言語パックに安全でない UI 文言が含まれるため、取り込みを拒否しました。",
    templateDescription: "このテンプレートから Workbench 文言を上書きし、バージョン系メタデータも保持します。",
    templateDownloaded: "言語パックのテンプレートを出力しました。",
    noCustomPack: "現在の言語にはまだカスタム言語パックがありません。",
    exported: "現在の言語パックを出力しました。",
    invalidJson: "言語パック JSON が無効です。",
    notFound: "この言語パックは見つかりません。",
    imported: "言語パックを取り込みました。",
    importedMismatch: "言語パックを取り込みましたが、対象バージョンが現在の Workbench と完全には一致していません。",
    removed: "言語パックを削除しました。",
  },
  es: {
    compatibilityExact: "Compatibilidad: coincide exactamente con la versión actual de Workbench",
    compatibilityLine: "Compatibilidad: coincide con la línea moxi actual",
    compatibilityMismatch: "Compatibilidad: la versión objetivo no coincide con Workbench",
    compatibilityGeneric: "Compatibilidad: paquete sin alcance, aplicado como cobertura genérica",
    targetPrefix: "Objetivo",
    noVersionLine: "sin línea de versión",
    noAppVersion: "sin versión de app",
    unsafeRejected: "Paquete de idioma rechazado porque contiene texto de UI no seguro.",
    templateDescription: "Usa esta plantilla para cubrir textos de Workbench y conservar metadatos de versión.",
    templateDownloaded: "Plantilla del paquete de idioma descargada.",
    noCustomPack: "Aún no hay un paquete personalizado instalado para el idioma actual.",
    exported: "Paquete de idioma actual exportado.",
    invalidJson: "JSON del paquete de idioma no válido.",
    notFound: "Paquete de idioma no encontrado.",
    imported: "Paquete de idioma importado.",
    importedMismatch: "Paquete importado, pero su versión objetivo no coincide completamente con Workbench.",
    removed: "Paquete de idioma eliminado.",
  },
};

const translatedCopy: Record<string, Partial<WorkbenchLanguagePackSystemCopy>> = {
  ar: { targetPrefix: "الهدف", noVersionLine: "لا يوجد خط إصدار", noAppVersion: "لا يوجد إصدار تطبيق", imported: "تم استيراد حزمة اللغة.", removed: "تمت إزالة حزمة اللغة.", invalidJson: "JSON حزمة اللغة غير صالح.", notFound: "لم يتم العثور على حزمة اللغة.", templateDownloaded: "تم تنزيل قالب حزمة اللغة.", exported: "تم تصدير حزمة اللغة الحالية.", noCustomPack: "لا توجد حزمة لغة مخصصة مثبتة للغة الحالية.", unsafeRejected: "تم رفض حزمة اللغة لأنها تحتوي على نص واجهة غير آمن." },
  bn: { targetPrefix: "লক্ষ্য", noVersionLine: "ভার্সন লাইন নেই", noAppVersion: "অ্যাপ সংস্করণ নেই", imported: "ভাষা প্যাক ইমপোর্ট করা হয়েছে।", removed: "ভাষা প্যাক সরানো হয়েছে।", invalidJson: "ভাষা প্যাক JSON বৈধ নয়।", notFound: "ভাষা প্যাক পাওয়া যায়নি।", templateDownloaded: "ভাষা প্যাক টেমপ্লেট ডাউনলোড হয়েছে।", exported: "বর্তমান ভাষা প্যাক এক্সপোর্ট হয়েছে।", noCustomPack: "বর্তমান ভাষার জন্য এখনো কাস্টম ভাষা প্যাক নেই।", unsafeRejected: "অনিরাপদ UI লেখা থাকায় ভাষা প্যাক প্রত্যাখ্যান করা হয়েছে।" },
  cs: { targetPrefix: "Cíl", noVersionLine: "bez řady verze", noAppVersion: "bez verze aplikace", imported: "Jazykový balíček byl importován.", removed: "Jazykový balíček byl odebrán.", invalidJson: "JSON jazykového balíčku není platný.", notFound: "Jazykový balíček nebyl nalezen.", templateDownloaded: "Šablona jazykového balíčku stažena.", exported: "Aktuální jazykový balíček byl exportován.", noCustomPack: "Pro aktuální jazyk ještě není nainstalován vlastní balíček.", unsafeRejected: "Jazykový balíček byl odmítnut kvůli nebezpečnému UI textu." },
  da: { targetPrefix: "Mål", noVersionLine: "ingen versionslinje", noAppVersion: "ingen appversion", imported: "Sprogpakken blev importeret.", removed: "Sprogpakken blev fjernet.", invalidJson: "Sprogpakke-JSON er ugyldig.", notFound: "Sprogpakken blev ikke fundet.", templateDownloaded: "Sprogpakke-skabelon hentet.", exported: "Den aktuelle sprogpakke blev eksporteret.", noCustomPack: "Der er endnu ingen brugerdefineret sprogpakke for det aktuelle sprog.", unsafeRejected: "Sprogpakken blev afvist, fordi den indeholder usikker UI-tekst." },
  de: { targetPrefix: "Ziel", noVersionLine: "keine Versionslinie", noAppVersion: "keine App-Version", imported: "Sprachpaket importiert.", removed: "Sprachpaket entfernt.", invalidJson: "Sprachpaket-JSON ist ungültig.", notFound: "Sprachpaket nicht gefunden.", templateDownloaded: "Sprachpaketvorlage heruntergeladen.", exported: "Aktuelles Sprachpaket exportiert.", noCustomPack: "Für die aktuelle Sprache ist noch kein eigenes Sprachpaket installiert.", unsafeRejected: "Sprachpaket abgelehnt, da es unsicheren UI-Text enthält." },
  el: { targetPrefix: "Στόχος", noVersionLine: "χωρίς γραμμή έκδοσης", noAppVersion: "χωρίς έκδοση εφαρμογής", imported: "Το πακέτο γλώσσας εισήχθη.", removed: "Το πακέτο γλώσσας αφαιρέθηκε.", invalidJson: "Το JSON του πακέτου γλώσσας δεν είναι έγκυρο.", notFound: "Το πακέτο γλώσσας δεν βρέθηκε.", templateDownloaded: "Το πρότυπο πακέτου γλώσσας λήφθηκε.", exported: "Το τρέχον πακέτο γλώσσας εξήχθη.", noCustomPack: "Δεν υπάρχει ακόμη προσαρμοσμένο πακέτο για την τρέχουσα γλώσσα.", unsafeRejected: "Το πακέτο γλώσσας απορρίφθηκε επειδή περιέχει μη ασφαλές κείμενο UI." },
  fa: { targetPrefix: "هدف", noVersionLine: "بدون خط نسخه", noAppVersion: "بدون نسخه برنامه", imported: "بسته زبان وارد شد.", removed: "بسته زبان حذف شد.", invalidJson: "JSON بسته زبان معتبر نیست.", notFound: "بسته زبان پیدا نشد.", templateDownloaded: "قالب بسته زبان دانلود شد.", exported: "بسته زبان فعلی خروجی گرفته شد.", noCustomPack: "برای زبان فعلی هنوز بسته زبان سفارشی نصب نشده است.", unsafeRejected: "بسته زبان به دلیل متن UI ناامن رد شد." },
  fi: { targetPrefix: "Kohde", noVersionLine: "ei versiolinjaa", noAppVersion: "ei sovellusversiota", imported: "Kielipaketti tuotu.", removed: "Kielipaketti poistettu.", invalidJson: "Kielipaketin JSON ei ole kelvollinen.", notFound: "Kielipakettia ei löytynyt.", templateDownloaded: "Kielipaketin malli ladattu.", exported: "Nykyinen kielipaketti viety.", noCustomPack: "Nykyiselle kielelle ei ole vielä asennettu mukautettua pakettia.", unsafeRejected: "Kielipaketti hylättiin, koska se sisältää turvatonta UI-tekstiä." },
  fr: { targetPrefix: "Cible", noVersionLine: "aucune ligne de version", noAppVersion: "aucune version d’app", imported: "Pack de langue importé.", removed: "Pack de langue retiré.", invalidJson: "JSON du pack de langue non valide.", notFound: "Pack de langue introuvable.", templateDownloaded: "Modèle de pack de langue téléchargé.", exported: "Pack de langue actuel exporté.", noCustomPack: "Aucun pack personnalisé n’est encore installé pour la langue actuelle.", unsafeRejected: "Pack de langue refusé car il contient du texte d’interface non sûr." },
  he: { targetPrefix: "יעד", noVersionLine: "אין קו גרסה", noAppVersion: "אין גרסת יישום", imported: "חבילת השפה יובאה.", removed: "חבילת השפה הוסרה.", invalidJson: "JSON של חבילת השפה אינו תקין.", notFound: "חבילת השפה לא נמצאה.", templateDownloaded: "תבנית חבילת השפה הורדה.", exported: "חבילת השפה הנוכחית יוצאה.", noCustomPack: "עדיין אין חבילת שפה מותאמת לשפה הנוכחית.", unsafeRejected: "חבילת השפה נדחתה כי היא מכילה טקסט UI לא בטוח." },
  hi: { targetPrefix: "लक्ष्य", noVersionLine: "कोई संस्करण लाइन नहीं", noAppVersion: "कोई ऐप संस्करण नहीं", imported: "भाषा पैक आयात हो गया।", removed: "भाषा पैक हटा दिया गया।", invalidJson: "भाषा पैक JSON अमान्य है।", notFound: "भाषा पैक नहीं मिला।", templateDownloaded: "भाषा पैक टेम्पलेट डाउनलोड हो गया।", exported: "वर्तमान भाषा पैक निर्यात हो गया।", noCustomPack: "वर्तमान भाषा के लिए अभी कोई कस्टम भाषा पैक स्थापित नहीं है।", unsafeRejected: "असुरक्षित UI पाठ होने के कारण भाषा पैक अस्वीकार किया गया।" },
  id: { targetPrefix: "Target", noVersionLine: "tanpa lini versi", noAppVersion: "tanpa versi aplikasi", imported: "Paket bahasa diimpor.", removed: "Paket bahasa dihapus.", invalidJson: "JSON paket bahasa tidak valid.", notFound: "Paket bahasa tidak ditemukan.", templateDownloaded: "Template paket bahasa diunduh.", exported: "Paket bahasa saat ini diekspor.", noCustomPack: "Belum ada paket bahasa kustom untuk bahasa saat ini.", unsafeRejected: "Paket bahasa ditolak karena berisi teks UI tidak aman." },
  it: { targetPrefix: "Target", noVersionLine: "nessuna linea versione", noAppVersion: "nessuna versione app", imported: "Pacchetto lingua importato.", removed: "Pacchetto lingua rimosso.", invalidJson: "JSON del pacchetto lingua non valido.", notFound: "Pacchetto lingua non trovato.", templateDownloaded: "Modello del pacchetto lingua scaricato.", exported: "Pacchetto lingua corrente esportato.", noCustomPack: "Nessun pacchetto personalizzato installato per la lingua corrente.", unsafeRejected: "Pacchetto lingua rifiutato perché contiene testo UI non sicuro." },
  ko: { targetPrefix: "대상", noVersionLine: "버전 라인 없음", noAppVersion: "앱 버전 없음", imported: "언어 팩을 가져왔습니다.", removed: "언어 팩을 제거했습니다.", invalidJson: "언어 팩 JSON이 올바르지 않습니다.", notFound: "언어 팩을 찾을 수 없습니다.", templateDownloaded: "언어 팩 템플릿을 다운로드했습니다.", exported: "현재 언어 팩을 내보냈습니다.", noCustomPack: "현재 언어에 설치된 사용자 언어 팩이 아직 없습니다.", unsafeRejected: "안전하지 않은 UI 문구가 포함되어 언어 팩을 거부했습니다." },
  ms: { targetPrefix: "Sasaran", noVersionLine: "tiada baris versi", noAppVersion: "tiada versi aplikasi", imported: "Pek bahasa diimport.", removed: "Pek bahasa dialih keluar.", invalidJson: "JSON pek bahasa tidak sah.", notFound: "Pek bahasa tidak ditemui.", templateDownloaded: "Templat pek bahasa dimuat turun.", exported: "Pek bahasa semasa dieksport.", noCustomPack: "Belum ada pek bahasa tersuai untuk bahasa semasa.", unsafeRejected: "Pek bahasa ditolak kerana mengandungi teks UI tidak selamat." },
  nl: { targetPrefix: "Doel", noVersionLine: "geen versielijn", noAppVersion: "geen appversie", imported: "Taalpakket geïmporteerd.", removed: "Taalpakket verwijderd.", invalidJson: "JSON van taalpakket is ongeldig.", notFound: "Taalpakket niet gevonden.", templateDownloaded: "Sjabloon voor taalpakket gedownload.", exported: "Huidig taalpakket geëxporteerd.", noCustomPack: "Er is nog geen aangepast taalpakket voor de huidige taal geïnstalleerd.", unsafeRejected: "Taalpakket geweigerd omdat het onveilige UI-tekst bevat." },
  no: { targetPrefix: "Mål", noVersionLine: "ingen versjonslinje", noAppVersion: "ingen appversjon", imported: "Språkpakken ble importert.", removed: "Språkpakken ble fjernet.", invalidJson: "Språkpakke-JSON er ugyldig.", notFound: "Språkpakken ble ikke funnet.", templateDownloaded: "Språkpakke-mal lastet ned.", exported: "Gjeldende språkpakke eksportert.", noCustomPack: "Ingen egendefinert språkpakke er installert for gjeldende språk ennå.", unsafeRejected: "Språkpakken ble avvist fordi den inneholder usikker UI-tekst." },
  pl: { targetPrefix: "Cel", noVersionLine: "brak linii wersji", noAppVersion: "brak wersji aplikacji", imported: "Pakiet językowy zaimportowany.", removed: "Pakiet językowy usunięty.", invalidJson: "JSON pakietu językowego jest nieprawidłowy.", notFound: "Nie znaleziono pakietu językowego.", templateDownloaded: "Szablon pakietu językowego pobrany.", exported: "Bieżący pakiet językowy wyeksportowany.", noCustomPack: "Dla bieżącego języka nie zainstalowano jeszcze pakietu niestandardowego.", unsafeRejected: "Pakiet językowy odrzucony, ponieważ zawiera niebezpieczny tekst UI." },
  "pt-br": { targetPrefix: "Alvo", noVersionLine: "sem linha de versão", noAppVersion: "sem versão do app", imported: "Pacote de idioma importado.", removed: "Pacote de idioma removido.", invalidJson: "JSON do pacote de idioma inválido.", notFound: "Pacote de idioma não encontrado.", templateDownloaded: "Modelo de pacote de idioma baixado.", exported: "Pacote de idioma atual exportado.", noCustomPack: "Ainda não há pacote personalizado instalado para o idioma atual.", unsafeRejected: "Pacote de idioma rejeitado porque contém texto de UI inseguro." },
  ro: { targetPrefix: "Țintă", noVersionLine: "fără linie de versiune", noAppVersion: "fără versiune aplicație", imported: "Pachetul de limbă a fost importat.", removed: "Pachetul de limbă a fost eliminat.", invalidJson: "JSON-ul pachetului de limbă este invalid.", notFound: "Pachetul de limbă nu a fost găsit.", templateDownloaded: "Șablonul pachetului de limbă a fost descărcat.", exported: "Pachetul de limbă curent a fost exportat.", noCustomPack: "Nu există încă un pachet personalizat pentru limba curentă.", unsafeRejected: "Pachetul de limbă a fost respins deoarece conține text UI nesigur." },
  ru: { targetPrefix: "Цель", noVersionLine: "нет линии версии", noAppVersion: "нет версии приложения", imported: "Языковой пакет импортирован.", removed: "Языковой пакет удален.", invalidJson: "JSON языкового пакета недействителен.", notFound: "Языковой пакет не найден.", templateDownloaded: "Шаблон языкового пакета загружен.", exported: "Текущий языковой пакет экспортирован.", noCustomPack: "Для текущего языка еще не установлен пользовательский пакет.", unsafeRejected: "Языковой пакет отклонен, так как содержит небезопасный UI-текст." },
  sv: { targetPrefix: "Mål", noVersionLine: "ingen versionslinje", noAppVersion: "ingen appversion", imported: "Språkpaketet importerades.", removed: "Språkpaketet togs bort.", invalidJson: "Språkpaketets JSON är ogiltig.", notFound: "Språkpaketet hittades inte.", templateDownloaded: "Språkpaketsmall hämtad.", exported: "Aktuellt språkpaket exporterades.", noCustomPack: "Inget anpassat språkpaket är ännu installerat för aktuellt språk.", unsafeRejected: "Språkpaketet avvisades eftersom det innehåller osäker UI-text." },
  sw: { targetPrefix: "Lengo", noVersionLine: "hakuna mstari wa toleo", noAppVersion: "hakuna toleo la programu", imported: "Pakiti ya lugha imeingizwa.", removed: "Pakiti ya lugha imeondolewa.", invalidJson: "JSON ya pakiti ya lugha si sahihi.", notFound: "Pakiti ya lugha haikupatikana.", templateDownloaded: "Kiolezo cha pakiti ya lugha kimepakuliwa.", exported: "Pakiti ya lugha ya sasa imetolewa.", noCustomPack: "Hakuna pakiti maalum ya lugha iliyosakinishwa kwa lugha ya sasa bado.", unsafeRejected: "Pakiti ya lugha imekataliwa kwa sababu ina maandishi ya UI yasiyo salama." },
  ta: { targetPrefix: "இலக்கு", noVersionLine: "பதிப்பு வரி இல்லை", noAppVersion: "பயன்பாட்டு பதிப்பு இல்லை", imported: "மொழி தொகுப்பு இறக்குமதி செய்யப்பட்டது.", removed: "மொழி தொகுப்பு அகற்றப்பட்டது.", invalidJson: "மொழி தொகுப்பு JSON செல்லுபடியாகவில்லை.", notFound: "மொழி தொகுப்பு கிடைக்கவில்லை.", templateDownloaded: "மொழி தொகுப்பு வார்ப்புரு பதிவிறக்கப்பட்டது.", exported: "தற்போதைய மொழி தொகுப்பு ஏற்றுமதி செய்யப்பட்டது.", noCustomPack: "தற்போதைய மொழிக்கான தனிப்பயன் மொழி தொகுப்பு இன்னும் நிறுவப்படவில்லை.", unsafeRejected: "பாதுகாப்பற்ற UI உரை இருப்பதால் மொழி தொகுப்பு நிராகரிக்கப்பட்டது." },
  th: { targetPrefix: "เป้าหมาย", noVersionLine: "ไม่มีสายเวอร์ชัน", noAppVersion: "ไม่มีเวอร์ชันแอป", imported: "นำเข้าแพ็กภาษาแล้ว", removed: "ลบแพ็กภาษาแล้ว", invalidJson: "JSON ของแพ็กภาษาไม่ถูกต้อง", notFound: "ไม่พบแพ็กภาษา", templateDownloaded: "ดาวน์โหลดเทมเพลตแพ็กภาษาแล้ว", exported: "ส่งออกแพ็กภาษาปัจจุบันแล้ว", noCustomPack: "ยังไม่มีแพ็กภาษากำหนดเองสำหรับภาษาปัจจุบัน", unsafeRejected: "ปฏิเสธแพ็กภาษาเพราะมีข้อความ UI ที่ไม่ปลอดภัย" },
  tr: { targetPrefix: "Hedef", noVersionLine: "sürüm çizgisi yok", noAppVersion: "uygulama sürümü yok", imported: "Dil paketi içe aktarıldı.", removed: "Dil paketi kaldırıldı.", invalidJson: "Dil paketi JSON geçersiz.", notFound: "Dil paketi bulunamadı.", templateDownloaded: "Dil paketi şablonu indirildi.", exported: "Geçerli dil paketi dışa aktarıldı.", noCustomPack: "Geçerli dil için henüz özel dil paketi yüklü değil.", unsafeRejected: "Güvenli olmayan UI metni içerdiği için dil paketi reddedildi." },
  uk: { targetPrefix: "Ціль", noVersionLine: "немає лінії версії", noAppVersion: "немає версії застосунку", imported: "Мовний пакет імпортовано.", removed: "Мовний пакет вилучено.", invalidJson: "JSON мовного пакета недійсний.", notFound: "Мовний пакет не знайдено.", templateDownloaded: "Шаблон мовного пакета завантажено.", exported: "Поточний мовний пакет експортовано.", noCustomPack: "Для поточної мови ще не встановлено користувацький пакет.", unsafeRejected: "Мовний пакет відхилено, бо він містить небезпечний UI-текст." },
  ur: { targetPrefix: "ہدف", noVersionLine: "کوئی ورژن لائن نہیں", noAppVersion: "کوئی ایپ ورژن نہیں", imported: "زبان پیک درآمد ہو گیا۔", removed: "زبان پیک ہٹا دیا گیا۔", invalidJson: "زبان پیک JSON درست نہیں۔", notFound: "زبان پیک نہیں ملا۔", templateDownloaded: "زبان پیک ٹیمپلیٹ ڈاؤن لوڈ ہو گیا۔", exported: "موجودہ زبان پیک برآمد ہو گیا۔", noCustomPack: "موجودہ زبان کے لیے ابھی کوئی کسٹم پیک نصب نہیں۔", unsafeRejected: "غیر محفوظ UI متن کی وجہ سے زبان پیک مسترد کر دیا گیا۔" },
  vi: { targetPrefix: "Đích", noVersionLine: "không có dòng phiên bản", noAppVersion: "không có phiên bản ứng dụng", imported: "Đã nhập gói ngôn ngữ.", removed: "Đã gỡ gói ngôn ngữ.", invalidJson: "JSON gói ngôn ngữ không hợp lệ.", notFound: "Không tìm thấy gói ngôn ngữ.", templateDownloaded: "Đã tải mẫu gói ngôn ngữ.", exported: "Đã xuất gói ngôn ngữ hiện tại.", noCustomPack: "Chưa có gói ngôn ngữ tùy chỉnh cho ngôn ngữ hiện tại.", unsafeRejected: "Gói ngôn ngữ bị từ chối vì chứa văn bản UI không an toàn." },
  "zh-tw": { targetPrefix: "目標", noVersionLine: "未指定版本線", noAppVersion: "未指定應用版本", imported: "語言包已匯入。", removed: "語言包已移除。", invalidJson: "語言包 JSON 無效。", notFound: "找不到這個語言包。", templateDownloaded: "語言包模板已下載。", exported: "目前語言包已匯出。", noCustomPack: "目前語言尚未安裝自訂語言包。", unsafeRejected: "語言包包含不安全的 UI 文案，已拒絕匯入。" },
};

for (const [language, copy] of Object.entries(translatedCopy)) {
  const targetPrefix = copy.targetPrefix ?? en.targetPrefix;
  SYSTEM_COPY[language] = {
    ...en,
    compatibilityExact: `✓ Workbench 2.0.0`,
    compatibilityLine: `✓ moxi 2.x`,
    compatibilityMismatch: `⚠ ${targetPrefix}: Workbench`,
    compatibilityGeneric: `◇ ${targetPrefix}: moxi`,
    templateDescription: `Workbench · moxi 2.x · ${targetPrefix}`,
    importedMismatch: `⚠ ${targetPrefix}: Workbench / moxi`,
    ...copy,
  };
}

export function getWorkbenchLanguagePackSystemCopy(language: string): WorkbenchLanguagePackSystemCopy {
  return SYSTEM_COPY[language] ?? SYSTEM_COPY[language.toLowerCase()] ?? en;
}
