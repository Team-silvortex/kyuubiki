export type WorkbenchGuardrailCopy = {
  title: string;
  subtitle: string;
  blocked: string;
  warnings: string;
  runtime: string;
  workflow: string;
  tone: { ok: string; warn: string; block: string };
};

type RuntimeAuditCopy = {
  sensitive: string;
  destructive: string;
  completed: string;
  cancelled: string;
  failed: string;
  prompted: string;
  assistant: string;
  script: string;
};

type MaterialCopy = {
  steel: string;
  aluminum: string;
  titanium: string;
  concrete: string;
  carbonFiber: string;
  custom: string;
};

export type WorkbenchMaterialLibraryCopy = {
  title: string;
  addMaterial: string;
  newCustom: string;
  importMaterials: string;
  importHint: string;
  show: string;
  hide: string;
  applySelected: string;
  applyAll: string;
  deleteMaterial: string;
};

export type WorkbenchScriptErrorCopy = {
  macroMissing: string;
  linkedContextMissing: string;
  linkedVersionMissing: string;
  linkedProjectMissing: string;
};

export type WorkbenchScriptInspectCopy = {
  layoutSummary: string;
  reportedAt: string;
  failureCode: string;
  failure: string;
  recovery: string;
  anchors: string;
  activeSidebar: string;
  runtimeTabs: string;
  immersive: string;
  overviewTab: string;
  selection3d: string;
  time: string;
};

export type WorkbenchProtocolAgentCopy = {
  countLabel: (agents: number, activeLeases: number, staleLeases: number) => string;
  reachableAgents: string;
  activeLeases: string;
  staleLeases: string;
};

export type WorkbenchProjectFlowCopy = {
  noJobVersion: string;
  noResultVersion: string;
  noRecordContext: string;
  linkedProjectMissing: string;
  noJobProject: string;
  noResultProject: string;
  selectJobFirst: string;
  missingResultJob: string;
  skippedSensitivePresets: (count: number) => string;
};

export type WorkbenchAssistantAuditCopy = {
  manualRecording: string;
  governanceDriftDetected: (driftLabel: string) => string;
  transactionRolledBack: string;
  planExecuted: string;
};

const guardrail: Record<string, WorkbenchGuardrailCopy> = {
  en: { title: "UX guardrails", subtitle: "Pre-run checks and next step", blocked: "Blocked", warnings: "Warnings", runtime: "Open runtime", workflow: "Open workflow", tone: { ok: "ready", warn: "review", block: "blocked" } },
  zh: { title: "操作防呆", subtitle: "运行前检查和下一步", blocked: "阻断项", warnings: "提醒项", runtime: "打开运行时", workflow: "打开工作流", tone: { ok: "可继续", warn: "需注意", block: "先处理" } },
  ja: { title: "操作ガード", subtitle: "実行前チェックと次の一手", blocked: "ブロック", warnings: "注意", runtime: "ランタイム", workflow: "ワークフロー", tone: { ok: "続行可", warn: "注意", block: "先に対応" } },
  es: { title: "Protecciones UX", subtitle: "Comprobaciones previas y siguiente paso", blocked: "Bloqueos", warnings: "Avisos", runtime: "Abrir runtime", workflow: "Abrir flujo", tone: { ok: "listo", warn: "revisar", block: "bloqueado" } },
  ar: { title: "حواجز UX", subtitle: "فحوصات ما قبل التشغيل والخطوة التالية", blocked: "محظور", warnings: "تحذيرات", runtime: "فتح وقت التشغيل", workflow: "فتح سير العمل", tone: { ok: "جاهز", warn: "راجع", block: "محظور" } },
  bn: { title: "UX গার্ডরেল", subtitle: "চালুর আগে পরীক্ষা ও পরের ধাপ", blocked: "ব্লক", warnings: "সতর্কতা", runtime: "রানটাইম খুলুন", workflow: "ওয়ার্কফ্লো খুলুন", tone: { ok: "প্রস্তুত", warn: "পর্যালোচনা", block: "ব্লক" } },
  cs: { title: "UX mantinely", subtitle: "Kontroly před spuštěním a další krok", blocked: "Blokováno", warnings: "Varování", runtime: "Otevřít runtime", workflow: "Otevřít workflow", tone: { ok: "připraveno", warn: "zkontrolovat", block: "blokováno" } },
  da: { title: "UX-værn", subtitle: "Før-kørselstjek og næste trin", blocked: "Blokeret", warnings: "Advarsler", runtime: "Åbn runtime", workflow: "Åbn workflow", tone: { ok: "klar", warn: "gennemgå", block: "blokeret" } },
  de: { title: "UX-Schutz", subtitle: "Vorabprüfungen und nächster Schritt", blocked: "Blockiert", warnings: "Warnungen", runtime: "Runtime öffnen", workflow: "Workflow öffnen", tone: { ok: "bereit", warn: "prüfen", block: "blockiert" } },
  el: { title: "Όρια UX", subtitle: "Έλεγχοι πριν την εκτέλεση και επόμενο βήμα", blocked: "Μπλοκαρισμένα", warnings: "Προειδοποιήσεις", runtime: "Άνοιγμα runtime", workflow: "Άνοιγμα workflow", tone: { ok: "έτοιμο", warn: "έλεγχος", block: "μπλοκ" } },
  fa: { title: "گارد UX", subtitle: "بررسی پیش از اجرا و گام بعد", blocked: "مسدود", warnings: "هشدارها", runtime: "باز کردن runtime", workflow: "باز کردن workflow", tone: { ok: "آماده", warn: "بازبینی", block: "مسدود" } },
  fi: { title: "UX-suojat", subtitle: "Esitarkistukset ja seuraava askel", blocked: "Estetty", warnings: "Varoitukset", runtime: "Avaa runtime", workflow: "Avaa työnkulku", tone: { ok: "valmis", warn: "tarkista", block: "estetty" } },
  fr: { title: "Garde-fous UX", subtitle: "Contrôles avant exécution et étape suivante", blocked: "Bloqués", warnings: "Avertissements", runtime: "Ouvrir le runtime", workflow: "Ouvrir le workflow", tone: { ok: "prêt", warn: "à revoir", block: "bloqué" } },
  he: { title: "מסילות UX", subtitle: "בדיקות לפני הרצה והצעד הבא", blocked: "חסום", warnings: "אזהרות", runtime: "פתח runtime", workflow: "פתח workflow", tone: { ok: "מוכן", warn: "סקירה", block: "חסום" } },
  hi: { title: "UX सुरक्षा", subtitle: "चलाने से पहले जांच और अगला चरण", blocked: "अवरुद्ध", warnings: "चेतावनी", runtime: "रनटाइम खोलें", workflow: "वर्कफ़्लो खोलें", tone: { ok: "तैयार", warn: "समीक्षा", block: "अवरुद्ध" } },
  id: { title: "Pengaman UX", subtitle: "Cek pra-jalan dan langkah berikut", blocked: "Diblokir", warnings: "Peringatan", runtime: "Buka runtime", workflow: "Buka workflow", tone: { ok: "siap", warn: "tinjau", block: "diblokir" } },
  it: { title: "Guardrail UX", subtitle: "Controlli pre-esecuzione e prossimo passo", blocked: "Bloccati", warnings: "Avvisi", runtime: "Apri runtime", workflow: "Apri workflow", tone: { ok: "pronto", warn: "rivedi", block: "bloccato" } },
  ko: { title: "UX 가드레일", subtitle: "실행 전 점검과 다음 단계", blocked: "차단", warnings: "경고", runtime: "런타임 열기", workflow: "워크플로 열기", tone: { ok: "준비", warn: "검토", block: "차단" } },
  ms: { title: "Penghad UX", subtitle: "Semakan pra-jalan dan langkah seterusnya", blocked: "Disekat", warnings: "Amaran", runtime: "Buka runtime", workflow: "Buka workflow", tone: { ok: "sedia", warn: "semak", block: "disekat" } },
  nl: { title: "UX-vangrails", subtitle: "Pre-run controles en volgende stap", blocked: "Geblokkeerd", warnings: "Waarschuwingen", runtime: "Runtime openen", workflow: "Workflow openen", tone: { ok: "gereed", warn: "bekijk", block: "geblokkeerd" } },
  no: { title: "UX-vern", subtitle: "Førkjøringssjekker og neste steg", blocked: "Blokkert", warnings: "Advarsler", runtime: "Åpne runtime", workflow: "Åpne workflow", tone: { ok: "klar", warn: "se over", block: "blokkert" } },
  pl: { title: "Bezpieczniki UX", subtitle: "Kontrole przed uruchomieniem i następny krok", blocked: "Zablokowane", warnings: "Ostrzeżenia", runtime: "Otwórz runtime", workflow: "Otwórz workflow", tone: { ok: "gotowe", warn: "sprawdź", block: "blokada" } },
  "pt-br": { title: "Proteções UX", subtitle: "Checagens antes da execução e próximo passo", blocked: "Bloqueados", warnings: "Avisos", runtime: "Abrir runtime", workflow: "Abrir workflow", tone: { ok: "pronto", warn: "revisar", block: "bloqueado" } },
  ro: { title: "Gărzi UX", subtitle: "Verificări înainte de rulare și pasul următor", blocked: "Blocate", warnings: "Avertismente", runtime: "Deschide runtime", workflow: "Deschide workflow", tone: { ok: "gata", warn: "revizuire", block: "blocat" } },
  ru: { title: "UX-защита", subtitle: "Проверки перед запуском и следующий шаг", blocked: "Блокировки", warnings: "Предупреждения", runtime: "Открыть рантайм", workflow: "Открыть workflow", tone: { ok: "готово", warn: "проверить", block: "заблокировано" } },
  sv: { title: "UX-skydd", subtitle: "Förkörningskontroller och nästa steg", blocked: "Blockerade", warnings: "Varningar", runtime: "Öppna runtime", workflow: "Öppna workflow", tone: { ok: "klar", warn: "granska", block: "blockerad" } },
  sw: { title: "Vizingiti vya UX", subtitle: "Ukaguzi kabla ya kuendesha na hatua inayofuata", blocked: "Imezuiwa", warnings: "Tahadhari", runtime: "Fungua runtime", workflow: "Fungua workflow", tone: { ok: "tayari", warn: "kagua", block: "imezuiwa" } },
  ta: { title: "UX காவல்", subtitle: "இயக்கத்திற்கு முன் சோதனை மற்றும் அடுத்த படி", blocked: "தடுக்கப்பட்டது", warnings: "எச்சரிக்கைகள்", runtime: "runtime திற", workflow: "workflow திற", tone: { ok: "தயார்", warn: "பரிசீலனை", block: "தடை" } },
  th: { title: "ราวกัน UX", subtitle: "ตรวจสอบก่อนรันและขั้นถัดไป", blocked: "ถูกบล็อก", warnings: "คำเตือน", runtime: "เปิด runtime", workflow: "เปิด workflow", tone: { ok: "พร้อม", warn: "ตรวจทาน", block: "บล็อก" } },
  tr: { title: "UX korumaları", subtitle: "Çalıştırma öncesi kontroller ve sonraki adım", blocked: "Engelli", warnings: "Uyarılar", runtime: "Runtime aç", workflow: "Workflow aç", tone: { ok: "hazır", warn: "incele", block: "engelli" } },
  uk: { title: "UX-запобіжники", subtitle: "Перевірки перед запуском і наступний крок", blocked: "Заблоковано", warnings: "Попередження", runtime: "Відкрити runtime", workflow: "Відкрити workflow", tone: { ok: "готово", warn: "перевірити", block: "блок" } },
  ur: { title: "UX حفاظتی حدیں", subtitle: "چلانے سے پہلے جانچ اور اگلا قدم", blocked: "روکا گیا", warnings: "تنبیہات", runtime: "runtime کھولیں", workflow: "workflow کھولیں", tone: { ok: "تیار", warn: "جائزہ", block: "روکا" } },
  vi: { title: "Rào chắn UX", subtitle: "Kiểm tra trước khi chạy và bước tiếp theo", blocked: "Bị chặn", warnings: "Cảnh báo", runtime: "Mở runtime", workflow: "Mở workflow", tone: { ok: "sẵn sàng", warn: "xem lại", block: "bị chặn" } },
  "zh-tw": { title: "操作防呆", subtitle: "執行前檢查與下一步", blocked: "阻斷項", warnings: "提醒項", runtime: "開啟執行時", workflow: "開啟工作流", tone: { ok: "可繼續", warn: "需注意", block: "先處理" } },
};

const audit: Record<string, RuntimeAuditCopy> = {
  en: { sensitive: "Sensitive", destructive: "Destructive", completed: "Completed", cancelled: "Cancelled", failed: "Failed", prompted: "Prompted", assistant: "Assistant", script: "Script" },
  zh: { sensitive: "敏感", destructive: "高风险", completed: "已执行", cancelled: "已取消", failed: "失败", prompted: "待确认", assistant: "助手", script: "脚本" },
  ja: { sensitive: "機微", destructive: "破壊的", completed: "完了", cancelled: "取消済み", failed: "失敗", prompted: "確認待ち", assistant: "アシスタント", script: "スクリプト" },
  es: { sensitive: "Sensible", destructive: "Destructivo", completed: "Completado", cancelled: "Cancelado", failed: "Fallido", prompted: "Pendiente", assistant: "Asistente", script: "Script" },
  ar: { sensitive: "حساس", destructive: "مدمر", completed: "مكتمل", cancelled: "ملغى", failed: "فشل", prompted: "بانتظار التأكيد", assistant: "المساعد", script: "سكريبت" },
  bn: { sensitive: "সংবেদনশীল", destructive: "ধ্বংসাত্মক", completed: "সম্পন্ন", cancelled: "বাতিল", failed: "ব্যর্থ", prompted: "নিশ্চিতকরণ", assistant: "সহকারী", script: "স্ক্রিপ্ট" },
  cs: { sensitive: "Citlivé", destructive: "Destruktivní", completed: "Hotovo", cancelled: "Zrušeno", failed: "Selhalo", prompted: "Čeká", assistant: "Asistent", script: "Skript" },
  da: { sensitive: "Følsom", destructive: "Destruktiv", completed: "Fuldført", cancelled: "Annulleret", failed: "Fejlet", prompted: "Afventer", assistant: "Assistent", script: "Script" },
  de: { sensitive: "Sensibel", destructive: "Destruktiv", completed: "Abgeschlossen", cancelled: "Abgebrochen", failed: "Fehlgeschlagen", prompted: "Bestätigung", assistant: "Assistent", script: "Skript" },
  el: { sensitive: "Ευαίσθητο", destructive: "Καταστροφικό", completed: "Ολοκληρώθηκε", cancelled: "Ακυρώθηκε", failed: "Απέτυχε", prompted: "Σε αναμονή", assistant: "Βοηθός", script: "Σκριπτ" },
  fa: { sensitive: "حساس", destructive: "مخرب", completed: "کامل", cancelled: "لغو شد", failed: "ناموفق", prompted: "در انتظار تایید", assistant: "دستیار", script: "اسکریپت" },
  fi: { sensitive: "Arkaluonteinen", destructive: "Tuhoava", completed: "Valmis", cancelled: "Peruttu", failed: "Epäonnistui", prompted: "Odottaa", assistant: "Avustaja", script: "Skripti" },
  fr: { sensitive: "Sensible", destructive: "Destructif", completed: "Terminé", cancelled: "Annulé", failed: "Échec", prompted: "Confirmation", assistant: "Assistant", script: "Script" },
  he: { sensitive: "רגיש", destructive: "הרסני", completed: "הושלם", cancelled: "בוטל", failed: "נכשל", prompted: "ממתין", assistant: "עוזר", script: "סקריפט" },
  hi: { sensitive: "संवेदनशील", destructive: "विनाशकारी", completed: "पूर्ण", cancelled: "रद्द", failed: "विफल", prompted: "पुष्टि लंबित", assistant: "सहायक", script: "स्क्रिप्ट" },
  id: { sensitive: "Sensitif", destructive: "Destruktif", completed: "Selesai", cancelled: "Dibatalkan", failed: "Gagal", prompted: "Menunggu", assistant: "Asisten", script: "Skrip" },
  it: { sensitive: "Sensibile", destructive: "Distruttivo", completed: "Completato", cancelled: "Annullato", failed: "Fallito", prompted: "In attesa", assistant: "Assistente", script: "Script" },
  ko: { sensitive: "민감", destructive: "파괴적", completed: "완료", cancelled: "취소됨", failed: "실패", prompted: "확인 대기", assistant: "도우미", script: "스크립트" },
  ms: { sensitive: "Sensitif", destructive: "Memusnahkan", completed: "Selesai", cancelled: "Dibatalkan", failed: "Gagal", prompted: "Menunggu", assistant: "Pembantu", script: "Skrip" },
  nl: { sensitive: "Gevoelig", destructive: "Destructief", completed: "Voltooid", cancelled: "Geannuleerd", failed: "Mislukt", prompted: "Wachtend", assistant: "Assistent", script: "Script" },
  no: { sensitive: "Sensitiv", destructive: "Destruktiv", completed: "Fullført", cancelled: "Avbrutt", failed: "Feilet", prompted: "Venter", assistant: "Assistent", script: "Skript" },
  pl: { sensitive: "Wrażliwe", destructive: "Destrukcyjne", completed: "Ukończone", cancelled: "Anulowane", failed: "Niepowodzenie", prompted: "Oczekuje", assistant: "Asystent", script: "Skrypt" },
  "pt-br": { sensitive: "Sensível", destructive: "Destrutivo", completed: "Concluído", cancelled: "Cancelado", failed: "Falhou", prompted: "Aguardando", assistant: "Assistente", script: "Script" },
  ro: { sensitive: "Sensibil", destructive: "Distructiv", completed: "Finalizat", cancelled: "Anulat", failed: "Eșuat", prompted: "În așteptare", assistant: "Asistent", script: "Script" },
  ru: { sensitive: "Чувствительно", destructive: "Разрушительно", completed: "Выполнено", cancelled: "Отменено", failed: "Сбой", prompted: "Ожидает", assistant: "Ассистент", script: "Скрипт" },
  sv: { sensitive: "Känslig", destructive: "Destruktiv", completed: "Slutförd", cancelled: "Avbruten", failed: "Misslyckad", prompted: "Väntar", assistant: "Assistent", script: "Skript" },
  sw: { sensitive: "Nyeti", destructive: "Haribifu", completed: "Imekamilika", cancelled: "Imeghairiwa", failed: "Imeshindwa", prompted: "Inasubiri", assistant: "Msaidizi", script: "Skripti" },
  ta: { sensitive: "நுணுக்கமான", destructive: "அழிக்கும்", completed: "முடிந்தது", cancelled: "ரத்து", failed: "தோல்வி", prompted: "காத்திருப்பு", assistant: "உதவியாளர்", script: "ஸ்கிரிப்ட்" },
  th: { sensitive: "ละเอียดอ่อน", destructive: "ทำลาย", completed: "เสร็จแล้ว", cancelled: "ยกเลิก", failed: "ล้มเหลว", prompted: "รอยืนยัน", assistant: "ผู้ช่วย", script: "สคริปต์" },
  tr: { sensitive: "Hassas", destructive: "Yıkıcı", completed: "Tamamlandı", cancelled: "İptal", failed: "Başarısız", prompted: "Onay bekliyor", assistant: "Asistan", script: "Betik" },
  uk: { sensitive: "Чутливе", destructive: "Руйнівне", completed: "Виконано", cancelled: "Скасовано", failed: "Збій", prompted: "Очікує", assistant: "Асистент", script: "Скрипт" },
  ur: { sensitive: "حساس", destructive: "تباہ کن", completed: "مکمل", cancelled: "منسوخ", failed: "ناکام", prompted: "تصدیق منتظر", assistant: "معاون", script: "اسکرپٹ" },
  vi: { sensitive: "Nhạy cảm", destructive: "Phá hủy", completed: "Hoàn tất", cancelled: "Đã hủy", failed: "Thất bại", prompted: "Chờ xác nhận", assistant: "Trợ lý", script: "Script" },
  "zh-tw": { sensitive: "敏感", destructive: "高風險", completed: "已執行", cancelled: "已取消", failed: "失敗", prompted: "待確認", assistant: "助手", script: "腳本" },
};

const material: Record<string, MaterialCopy> = {
  en: { steel: "Steel", aluminum: "Aluminum", titanium: "Titanium", concrete: "Concrete", carbonFiber: "Carbon fiber", custom: "Custom" },
  zh: { steel: "钢", aluminum: "铝", titanium: "钛", concrete: "混凝土", carbonFiber: "碳纤维", custom: "自定义" },
  ja: { steel: "鋼", aluminum: "アルミ", titanium: "チタン", concrete: "コンクリート", carbonFiber: "炭素繊維", custom: "カスタム" },
  es: { steel: "Acero", aluminum: "Aluminio", titanium: "Titanio", concrete: "Hormigón", carbonFiber: "Fibra de carbono", custom: "Personalizado" },
  ar: { steel: "فولاذ", aluminum: "ألمنيوم", titanium: "تيتانيوم", concrete: "خرسانة", carbonFiber: "ألياف كربون", custom: "مخصص" },
  bn: { steel: "ইস্পাত", aluminum: "অ্যালুমিনিয়াম", titanium: "টাইটানিয়াম", concrete: "কংক্রিট", carbonFiber: "কার্বন ফাইবার", custom: "কাস্টম" },
  cs: { steel: "Ocel", aluminum: "Hliník", titanium: "Titan", concrete: "Beton", carbonFiber: "Uhlíkové vlákno", custom: "Vlastní" },
  da: { steel: "Stål", aluminum: "Aluminium", titanium: "Titan", concrete: "Beton", carbonFiber: "Kulfiber", custom: "Tilpasset" },
  fr: { steel: "Acier", aluminum: "Aluminium", titanium: "Titane", concrete: "Béton", carbonFiber: "Fibre de carbone", custom: "Personnalisé" },
  de: { steel: "Stahl", aluminum: "Aluminium", titanium: "Titan", concrete: "Beton", carbonFiber: "Kohlefaser", custom: "Benutzerdefiniert" },
  el: { steel: "Χάλυβας", aluminum: "Αλουμίνιο", titanium: "Τιτάνιο", concrete: "Σκυρόδεμα", carbonFiber: "Ίνα άνθρακα", custom: "Προσαρμοσμένο" },
  fa: { steel: "فولاد", aluminum: "آلومینیوم", titanium: "تیتانیوم", concrete: "بتن", carbonFiber: "فیبر کربن", custom: "سفارشی" },
  fi: { steel: "Teräs", aluminum: "Alumiini", titanium: "Titaani", concrete: "Betoni", carbonFiber: "Hiilikuitu", custom: "Mukautettu" },
  he: { steel: "פלדה", aluminum: "אלומיניום", titanium: "טיטניום", concrete: "בטון", carbonFiber: "סיבי פחמן", custom: "מותאם" },
  hi: { steel: "इस्पात", aluminum: "एल्युमिनियम", titanium: "टाइटेनियम", concrete: "कंक्रीट", carbonFiber: "कार्बन फाइबर", custom: "कस्टम" },
  id: { steel: "Baja", aluminum: "Aluminium", titanium: "Titanium", concrete: "Beton", carbonFiber: "Serat karbon", custom: "Kustom" },
  it: { steel: "Acciaio", aluminum: "Alluminio", titanium: "Titanio", concrete: "Calcestruzzo", carbonFiber: "Fibra di carbonio", custom: "Personalizzato" },
  ko: { steel: "강", aluminum: "알루미늄", titanium: "티타늄", concrete: "콘크리트", carbonFiber: "탄소섬유", custom: "사용자 정의" },
  ms: { steel: "Keluli", aluminum: "Aluminium", titanium: "Titanium", concrete: "Konkrit", carbonFiber: "Gentian karbon", custom: "Tersuai" },
  nl: { steel: "Staal", aluminum: "Aluminium", titanium: "Titanium", concrete: "Beton", carbonFiber: "Koolstofvezel", custom: "Aangepast" },
  no: { steel: "Stål", aluminum: "Aluminium", titanium: "Titan", concrete: "Betong", carbonFiber: "Karbonfiber", custom: "Egendefinert" },
  pl: { steel: "Stal", aluminum: "Aluminium", titanium: "Tytan", concrete: "Beton", carbonFiber: "Włókno węglowe", custom: "Własne" },
  "pt-br": { steel: "Aço", aluminum: "Alumínio", titanium: "Titânio", concrete: "Concreto", carbonFiber: "Fibra de carbono", custom: "Personalizado" },
  ro: { steel: "Oțel", aluminum: "Aluminiu", titanium: "Titan", concrete: "Beton", carbonFiber: "Fibră de carbon", custom: "Personalizat" },
  ru: { steel: "Сталь", aluminum: "Алюминий", titanium: "Титан", concrete: "Бетон", carbonFiber: "Углеволокно", custom: "Свое" },
  sv: { steel: "Stål", aluminum: "Aluminium", titanium: "Titan", concrete: "Betong", carbonFiber: "Kolfiber", custom: "Anpassad" },
  sw: { steel: "Chuma", aluminum: "Alumini", titanium: "Titanium", concrete: "Saruji", carbonFiber: "Nyuzi kaboni", custom: "Maalum" },
  ta: { steel: "எஃகு", aluminum: "அலுமினியம்", titanium: "டைட்டானியம்", concrete: "கான்கிரீட்", carbonFiber: "கார்பன் நார்", custom: "தனிப்பயன்" },
  th: { steel: "เหล็ก", aluminum: "อะลูมิเนียม", titanium: "ไทเทเนียม", concrete: "คอนกรีต", carbonFiber: "คาร์บอนไฟเบอร์", custom: "กำหนดเอง" },
  tr: { steel: "Çelik", aluminum: "Alüminyum", titanium: "Titanyum", concrete: "Beton", carbonFiber: "Karbon fiber", custom: "Özel" },
  uk: { steel: "Сталь", aluminum: "Алюміній", titanium: "Титан", concrete: "Бетон", carbonFiber: "Вуглеволокно", custom: "Власне" },
  ur: { steel: "فولاد", aluminum: "ایلومینیم", titanium: "ٹائٹینیم", concrete: "کنکریٹ", carbonFiber: "کاربن فائبر", custom: "حسب ضرورت" },
  vi: { steel: "Thép", aluminum: "Nhôm", titanium: "Titan", concrete: "Bê tông", carbonFiber: "Sợi carbon", custom: "Tùy chỉnh" },
  "zh-tw": { steel: "鋼", aluminum: "鋁", titanium: "鈦", concrete: "混凝土", carbonFiber: "碳纖維", custom: "自訂" },
};

const materialLibrary: Record<string, WorkbenchMaterialLibraryCopy> = {
  en: { title: "Material Library", addMaterial: "Add material", newCustom: "New custom", importMaterials: "Import materials", importHint: "Accepts JSON / CSV material libraries.", show: "Show", hide: "Hide", applySelected: "Apply to selected", applyAll: "Apply to all", deleteMaterial: "Delete material" },
  zh: { title: "材料库", addMaterial: "添加材料", newCustom: "新建自定义", importMaterials: "导入材料库", importHint: "支持 JSON / CSV 材料文件。", show: "显示", hide: "隐藏", applySelected: "赋给当前单元", applyAll: "赋给全部单元", deleteMaterial: "删除材料" },
  ja: { title: "材料ライブラリ", addMaterial: "材料を追加", newCustom: "カスタム作成", importMaterials: "材料ライブラリを読み込む", importHint: "JSON / CSV の材料ライブラリに対応します。", show: "表示", hide: "非表示", applySelected: "選択要素に適用", applyAll: "全要素に適用", deleteMaterial: "材料を削除" },
  es: { title: "Biblioteca de materiales", addMaterial: "Añadir material", newCustom: "Nuevo personalizado", importMaterials: "Importar materiales", importHint: "Acepta bibliotecas JSON / CSV.", show: "Mostrar", hide: "Ocultar", applySelected: "Aplicar a seleccionado", applyAll: "Aplicar a todo", deleteMaterial: "Eliminar material" },
  fr: { title: "Bibliothèque matériaux", addMaterial: "Ajouter matériau", newCustom: "Nouveau personnalisé", importMaterials: "Importer matériaux", importHint: "Accepte les bibliothèques JSON / CSV.", show: "Afficher", hide: "Masquer", applySelected: "Appliquer à la sélection", applyAll: "Appliquer à tout", deleteMaterial: "Supprimer matériau" },
  de: { title: "Materialbibliothek", addMaterial: "Material hinzufügen", newCustom: "Neu benutzerdefiniert", importMaterials: "Materialien importieren", importHint: "Akzeptiert JSON-/CSV-Materialbibliotheken.", show: "Anzeigen", hide: "Ausblenden", applySelected: "Auf Auswahl anwenden", applyAll: "Auf alle anwenden", deleteMaterial: "Material löschen" },
  ko: { title: "재료 라이브러리", addMaterial: "재료 추가", newCustom: "사용자 정의 생성", importMaterials: "재료 가져오기", importHint: "JSON / CSV 재료 라이브러리를 지원합니다.", show: "표시", hide: "숨김", applySelected: "선택 항목에 적용", applyAll: "전체 적용", deleteMaterial: "재료 삭제" },
  ru: { title: "Библиотека материалов", addMaterial: "Добавить материал", newCustom: "Свое", importMaterials: "Импорт материалов", importHint: "Поддерживает библиотеки JSON / CSV.", show: "Показать", hide: "Скрыть", applySelected: "Применить к выбранному", applyAll: "Применить ко всем", deleteMaterial: "Удалить материал" },
  "zh-tw": { title: "材料庫", addMaterial: "新增材料", newCustom: "新增自訂", importMaterials: "匯入材料庫", importHint: "支援 JSON / CSV 材料檔。", show: "顯示", hide: "隱藏", applySelected: "套用到目前單元", applyAll: "套用到全部單元", deleteMaterial: "刪除材料" },
};

const scriptErrors: Record<string, WorkbenchScriptErrorCopy> = {
  en: { macroMissing: "Could not find the requested macro.", linkedContextMissing: "Could not resolve the linked data record context.", linkedVersionMissing: "This record does not have a linked model version.", linkedProjectMissing: "This record does not have a linked project." },
  zh: { macroMissing: "找不到指定的宏动作。", linkedContextMissing: "找不到关联的数据记录上下文。", linkedVersionMissing: "这条记录没有关联模型版本。", linkedProjectMissing: "这条记录没有关联项目。" },
  ja: { macroMissing: "指定されたマクロが見つかりませんでした。", linkedContextMissing: "関連するデータレコード文脈を解決できませんでした。", linkedVersionMissing: "このレコードには関連モデルバージョンがありません。", linkedProjectMissing: "このレコードには関連プロジェクトがありません。" },
  es: { macroMissing: "No se encontró la macro solicitada.", linkedContextMissing: "No se pudo resolver el contexto del registro enlazado.", linkedVersionMissing: "Este registro no tiene una versión de modelo enlazada.", linkedProjectMissing: "Este registro no tiene un proyecto enlazado." },
  fr: { macroMissing: "Macro demandée introuvable.", linkedContextMissing: "Impossible de résoudre le contexte de l'enregistrement lié.", linkedVersionMissing: "Cet enregistrement n'a pas de version de modèle liée.", linkedProjectMissing: "Cet enregistrement n'a pas de projet lié." },
  de: { macroMissing: "Das angeforderte Makro wurde nicht gefunden.", linkedContextMissing: "Der verknüpfte Datensatzkontext konnte nicht aufgelöst werden.", linkedVersionMissing: "Dieser Datensatz hat keine verknüpfte Modellversion.", linkedProjectMissing: "Dieser Datensatz hat kein verknüpftes Projekt." },
  ko: { macroMissing: "요청한 매크로를 찾을 수 없습니다.", linkedContextMissing: "연결된 데이터 레코드 컨텍스트를 확인할 수 없습니다.", linkedVersionMissing: "이 레코드에는 연결된 모델 버전이 없습니다.", linkedProjectMissing: "이 레코드에는 연결된 프로젝트가 없습니다." },
  ru: { macroMissing: "Запрошенный макрос не найден.", linkedContextMissing: "Не удалось разрешить контекст связанной записи.", linkedVersionMissing: "У этой записи нет связанной версии модели.", linkedProjectMissing: "У этой записи нет связанного проекта." },
  "zh-tw": { macroMissing: "找不到指定的巨集動作。", linkedContextMissing: "找不到關聯的資料記錄上下文。", linkedVersionMissing: "這筆記錄沒有關聯模型版本。", linkedProjectMissing: "這筆記錄沒有關聯專案。" },
};

const inspect: Record<string, WorkbenchScriptInspectCopy> = {
  en: { layoutSummary: "Layout Summary", reportedAt: "reported at", failureCode: "failure code", failure: "failure", recovery: "recovery", anchors: "anchors", activeSidebar: "active sidebar", runtimeTabs: "runtime tabs", immersive: "immersive", overviewTab: "overview tab", selection3d: "3d selection", time: "Time" },
  zh: { layoutSummary: "布局摘要", reportedAt: "报告时间", failureCode: "失败代码", failure: "失败原因", recovery: "恢复建议", anchors: "锚点", activeSidebar: "当前侧栏", runtimeTabs: "运行时标签", immersive: "沉浸模式", overviewTab: "概览标签", selection3d: "3D 选择", time: "时间" },
  ja: { layoutSummary: "レイアウト要約", reportedAt: "報告時刻", failureCode: "失敗コード", failure: "失敗", recovery: "復旧", anchors: "アンカー", activeSidebar: "アクティブサイドバー", runtimeTabs: "ランタイムタブ", immersive: "没入", overviewTab: "概要タブ", selection3d: "3D 選択", time: "時刻" },
  es: { layoutSummary: "Resumen de diseño", reportedAt: "reportado", failureCode: "código de fallo", failure: "fallo", recovery: "recuperación", anchors: "anclas", activeSidebar: "barra activa", runtimeTabs: "pestañas runtime", immersive: "inmersivo", overviewTab: "pestaña resumen", selection3d: "selección 3D", time: "Hora" },
  fr: { layoutSummary: "Résumé de disposition", reportedAt: "rapporté à", failureCode: "code d'échec", failure: "échec", recovery: "récupération", anchors: "ancres", activeSidebar: "barre active", runtimeTabs: "onglets runtime", immersive: "immersif", overviewTab: "onglet aperçu", selection3d: "sélection 3D", time: "Heure" },
  de: { layoutSummary: "Layout-Zusammenfassung", reportedAt: "gemeldet um", failureCode: "Fehlercode", failure: "Fehler", recovery: "Wiederherstellung", anchors: "Anker", activeSidebar: "aktive Seitenleiste", runtimeTabs: "Runtime-Tabs", immersive: "immersiv", overviewTab: "Übersichtstab", selection3d: "3D-Auswahl", time: "Zeit" },
  ko: { layoutSummary: "레이아웃 요약", reportedAt: "보고 시각", failureCode: "실패 코드", failure: "실패", recovery: "복구", anchors: "앵커", activeSidebar: "활성 사이드바", runtimeTabs: "런타임 탭", immersive: "몰입", overviewTab: "개요 탭", selection3d: "3D 선택", time: "시간" },
  ru: { layoutSummary: "Сводка компоновки", reportedAt: "время отчета", failureCode: "код сбоя", failure: "сбой", recovery: "восстановление", anchors: "якоря", activeSidebar: "активная панель", runtimeTabs: "вкладки рантайма", immersive: "иммерсивно", overviewTab: "вкладка обзора", selection3d: "3D-выбор", time: "Время" },
  "zh-tw": { layoutSummary: "佈局摘要", reportedAt: "回報時間", failureCode: "失敗代碼", failure: "失敗原因", recovery: "恢復建議", anchors: "錨點", activeSidebar: "目前側欄", runtimeTabs: "執行時分頁", immersive: "沉浸模式", overviewTab: "概覽分頁", selection3d: "3D 選擇", time: "時間" },
};

const protocolAgent: Record<string, Omit<WorkbenchProtocolAgentCopy, "countLabel">> = {
  en: { reachableAgents: "Reachable agents", activeLeases: "Active leases", staleLeases: "Stale leases" },
  zh: { reachableAgents: "可达代理", activeLeases: "活跃租约", staleLeases: "过期租约" },
  ja: { reachableAgents: "到達可能エージェント", activeLeases: "アクティブリース", staleLeases: "期限切れリース" },
  es: { reachableAgents: "Agentes alcanzables", activeLeases: "Reservas activas", staleLeases: "Reservas caducas" },
  fr: { reachableAgents: "Agents joignables", activeLeases: "Baux actifs", staleLeases: "Baux expirés" },
  de: { reachableAgents: "Erreichbare Agents", activeLeases: "Aktive Leases", staleLeases: "Abgelaufene Leases" },
  ko: { reachableAgents: "도달 가능한 에이전트", activeLeases: "활성 임대", staleLeases: "만료 임대" },
  ru: { reachableAgents: "Доступные агенты", activeLeases: "Активные аренды", staleLeases: "Устаревшие аренды" },
  "zh-tw": { reachableAgents: "可達代理", activeLeases: "活躍租約", staleLeases: "過期租約" },
};

const auditEmpty: Record<string, string> = {
  en: "No security events match the current filters.",
  zh: "当前筛选下还没有安全事件。",
  ja: "現在のフィルターに一致するセキュリティイベントはありません。",
  es: "No hay eventos de seguridad para los filtros actuales.",
  fr: "Aucun événement de sécurité ne correspond aux filtres actuels.",
  de: "Keine Sicherheitsereignisse für die aktuellen Filter.",
  ko: "현재 필터와 일치하는 보안 이벤트가 없습니다.",
  ru: "Нет событий безопасности для текущих фильтров.",
  "zh-tw": "目前篩選下沒有安全事件。",
};

const projectFlow: Record<string, WorkbenchProjectFlowCopy> = {
  en: {
    noJobVersion: "This job does not have a linked model version.",
    noResultVersion: "This result does not have a linked model version.",
    noRecordContext: "This record does not have a linked project or version context yet.",
    linkedProjectMissing: "Could not find the linked project.",
    noJobProject: "This job does not have a linked project.",
    noResultProject: "This result does not have a linked project.",
    selectJobFirst: "Select a job record first.",
    missingResultJob: "Could not find the job record linked to this result.",
    skippedSensitivePresets: (count) => `Skipped ${count} sensitive automation preset(s) during project import.`,
  },
  zh: {
    noJobVersion: "这个任务还没有关联模型版本。",
    noResultVersion: "这个结果还没有关联模型版本。",
    noRecordContext: "这条记录还没有可应用的项目或版本上下文。",
    linkedProjectMissing: "找不到关联项目。",
    noJobProject: "这个任务还没有关联项目。",
    noResultProject: "这个结果还没有关联项目。",
    selectJobFirst: "请先选择一条任务记录。",
    missingResultJob: "找不到这条结果对应的任务记录。",
    skippedSensitivePresets: (count) => `项目导入时跳过了 ${count} 个敏感自动化预设。`,
  },
  ja: {
    noJobVersion: "このジョブには関連するモデルバージョンがまだありません。",
    noResultVersion: "この結果には関連するモデルバージョンがまだありません。",
    noRecordContext: "このレコードには適用できる project / version の文脈がまだありません。",
    linkedProjectMissing: "関連プロジェクトが見つかりませんでした。",
    noJobProject: "このジョブには関連プロジェクトがまだありません。",
    noResultProject: "この結果には関連プロジェクトがまだありません。",
    selectJobFirst: "先にジョブレコードを選択してください。",
    missingResultJob: "この結果に対応するジョブレコードが見つかりませんでした。",
    skippedSensitivePresets: (count) => `プロジェクトの取り込み時に機微な automation preset を ${count} 件スキップしました。`,
  },
  es: {
    noJobVersion: "Esta tarea no tiene una versión de modelo enlazada.",
    noResultVersion: "Este resultado no tiene una versión de modelo enlazada.",
    noRecordContext: "Este registro aún no tiene contexto de proyecto o versión.",
    linkedProjectMissing: "No se encontró el proyecto enlazado.",
    noJobProject: "Esta tarea no tiene un proyecto enlazado.",
    noResultProject: "Este resultado no tiene un proyecto enlazado.",
    selectJobFirst: "Selecciona primero un registro de tarea.",
    missingResultJob: "No se encontró la tarea enlazada a este resultado.",
    skippedSensitivePresets: (count) => `Se omitieron ${count} presets sensibles durante la importación del proyecto.`,
  },
  fr: {
    noJobVersion: "Cette tâche n'a pas de version de modèle liée.",
    noResultVersion: "Ce résultat n'a pas de version de modèle liée.",
    noRecordContext: "Cet enregistrement n'a pas encore de contexte projet ou version.",
    linkedProjectMissing: "Projet lié introuvable.",
    noJobProject: "Cette tâche n'a pas de projet lié.",
    noResultProject: "Ce résultat n'a pas de projet lié.",
    selectJobFirst: "Sélectionnez d'abord un enregistrement de tâche.",
    missingResultJob: "Tâche liée à ce résultat introuvable.",
    skippedSensitivePresets: (count) => `${count} presets sensibles ignorés pendant l'import du projet.`,
  },
  "zh-tw": {
    noJobVersion: "這個任務尚未關聯模型版本。",
    noResultVersion: "這個結果尚未關聯模型版本。",
    noRecordContext: "這筆記錄尚無可套用的專案或版本上下文。",
    linkedProjectMissing: "找不到關聯專案。",
    noJobProject: "這個任務尚未關聯專案。",
    noResultProject: "這個結果尚未關聯專案。",
    selectJobFirst: "請先選擇一筆任務記錄。",
    missingResultJob: "找不到這筆結果對應的任務記錄。",
    skippedSensitivePresets: (count) => `專案匯入時跳過了 ${count} 個敏感自動化預設。`,
  },
};

const assistantAudit: Record<string, WorkbenchAssistantAuditCopy> = {
  en: {
    manualRecording: "Recorded from manual UI interaction",
    governanceDriftDetected: (driftLabel) => `Governance drift detected: ${driftLabel}.`,
    transactionRolledBack: "Rolled back the last assistant transaction.",
    planExecuted: "Assistant plan executed.",
  },
  zh: {
    manualRecording: "手动 UI 录制",
    governanceDriftDetected: (driftLabel) => `检测到治理漂移: ${driftLabel}。`,
    transactionRolledBack: "已回滚上一轮助手事务。",
    planExecuted: "助手计划已执行。",
  },
  ja: {
    manualRecording: "手動 UI 操作から記録",
    governanceDriftDetected: (driftLabel) => `ガバナンス偏差を検出しました: ${driftLabel}。`,
    transactionRolledBack: "直前のアシスタント操作をロールバックしました。",
    planExecuted: "アシスタントのプランを実行しました。",
  },
  es: {
    manualRecording: "Grabado desde interacción manual de UI",
    governanceDriftDetected: (driftLabel) => `Deriva de gobierno detectada: ${driftLabel}.`,
    transactionRolledBack: "Se revirtió la última transacción del asistente.",
    planExecuted: "Plan del asistente ejecutado.",
  },
  fr: {
    manualRecording: "Enregistré depuis une interaction UI manuelle",
    governanceDriftDetected: (driftLabel) => `Dérive de gouvernance détectée : ${driftLabel}.`,
    transactionRolledBack: "Dernière transaction assistant annulée.",
    planExecuted: "Plan assistant exécuté.",
  },
  "zh-tw": {
    manualRecording: "手動 UI 錄製",
    governanceDriftDetected: (driftLabel) => `偵測到治理漂移：${driftLabel}。`,
    transactionRolledBack: "已回滾上一輪助手事務。",
    planExecuted: "助手計畫已執行。",
  },
};

function normalize(language?: string) {
  return (language ?? "en").toLowerCase();
}

function resolveRecord<T>(records: Record<string, T>, language?: string): T {
  const key = normalize(language);
  return records[key] ?? records[key.split("-")[0]] ?? records.en;
}

export function getWorkbenchGuardrailCopy(language?: string): WorkbenchGuardrailCopy {
  const key = normalize(language);
  return guardrail[key] ?? guardrail[key.split("-")[0]] ?? guardrail.en;
}

export function getWorkbenchRuntimeAuditCopy(language?: string): RuntimeAuditCopy {
  const key = normalize(language);
  const direct = audit[key] ?? audit[key.split("-")[0]];
  if (direct) return direct;
  const source = getWorkbenchGuardrailCopy(language);
  return {
    sensitive: source.warnings,
    destructive: source.tone.block,
    completed: source.tone.ok,
    cancelled: source.tone.warn,
    failed: source.tone.block,
    prompted: source.tone.warn,
    assistant: source.runtime,
    script: source.workflow,
  };
}

export function getWorkbenchMaterialCopy(language?: string): MaterialCopy {
  return resolveRecord(material, language);
}

export function getWorkbenchMaterialLibraryCopy(language?: string): WorkbenchMaterialLibraryCopy {
  const key = normalize(language);
  const direct = materialLibrary[key] ?? materialLibrary[key.split("-")[0]];
  if (direct) return direct;
  const labels = getWorkbenchMaterialCopy(language);
  const guard = getWorkbenchGuardrailCopy(language);
  return {
    title: `${labels.custom} / ${labels.steel}`,
    addMaterial: `+ ${labels.custom}`,
    newCustom: labels.custom,
    importMaterials: `JSON / CSV`,
    importHint: `${labels.custom} JSON / CSV`,
    show: guard.tone.ok,
    hide: guard.tone.warn,
    applySelected: guard.tone.ok,
    applyAll: guard.tone.ok,
    deleteMaterial: guard.tone.block,
  };
}

export function getWorkbenchScriptErrorCopy(language?: string): WorkbenchScriptErrorCopy {
  const key = normalize(language);
  const direct = scriptErrors[key] ?? scriptErrors[key.split("-")[0]];
  if (direct) return direct;
  const auditCopy = getWorkbenchRuntimeAuditCopy(language);
  return {
    macroMissing: auditCopy.failed,
    linkedContextMissing: auditCopy.failed,
    linkedVersionMissing: auditCopy.failed,
    linkedProjectMissing: auditCopy.failed,
  };
}

export function getWorkbenchScriptInspectCopy(language?: string): WorkbenchScriptInspectCopy {
  const key = normalize(language);
  const direct = inspect[key] ?? inspect[key.split("-")[0]];
  if (direct) return direct;
  const guard = getWorkbenchGuardrailCopy(language);
  const auditCopy = getWorkbenchRuntimeAuditCopy(language);
  return {
    layoutSummary: guard.title,
    reportedAt: auditCopy.completed,
    failureCode: auditCopy.failed,
    failure: auditCopy.failed,
    recovery: guard.tone.warn,
    anchors: guard.blocked,
    activeSidebar: guard.workflow,
    runtimeTabs: guard.runtime,
    immersive: guard.tone.ok,
    overviewTab: guard.workflow,
    selection3d: "3D",
    time: auditCopy.completed,
  };
}

export function getWorkbenchProtocolAgentCopy(language?: string): WorkbenchProtocolAgentCopy {
  const key = normalize(language);
  const copy = protocolAgent[key] ?? protocolAgent[key.split("-")[0]] ?? {
    reachableAgents: getWorkbenchRuntimeAuditCopy(language).assistant,
    activeLeases: getWorkbenchRuntimeAuditCopy(language).completed,
    staleLeases: getWorkbenchRuntimeAuditCopy(language).failed,
  };
  return {
    ...copy,
    countLabel: (agents, activeLeases, staleLeases) =>
      `${agents} ${copy.reachableAgents} · ${activeLeases} ${copy.activeLeases} · ${staleLeases} ${copy.staleLeases}`,
  };
}

export function getWorkbenchRuntimeAuditEmptyLabel(language?: string): string {
  const key = normalize(language);
  return auditEmpty[key] ?? auditEmpty[key.split("-")[0]] ?? getWorkbenchRuntimeAuditCopy(language).failed;
}

export function getWorkbenchProjectFlowCopy(language?: string): WorkbenchProjectFlowCopy {
  const key = normalize(language);
  const direct = projectFlow[key] ?? projectFlow[key.split("-")[0]];
  if (direct) return direct;
  const auditCopy = getWorkbenchRuntimeAuditCopy(language);
  return {
    noJobVersion: auditCopy.failed,
    noResultVersion: auditCopy.failed,
    noRecordContext: auditCopy.failed,
    linkedProjectMissing: auditCopy.failed,
    noJobProject: auditCopy.failed,
    noResultProject: auditCopy.failed,
    selectJobFirst: auditCopy.prompted,
    missingResultJob: auditCopy.failed,
    skippedSensitivePresets: (count) => `${auditCopy.sensitive}: ${count}`,
  };
}

export function getWorkbenchAssistantAuditCopy(language?: string): WorkbenchAssistantAuditCopy {
  const key = normalize(language);
  const direct = assistantAudit[key] ?? assistantAudit[key.split("-")[0]];
  if (direct) return direct;
  const auditCopy = getWorkbenchRuntimeAuditCopy(language);
  return {
    manualRecording: auditCopy.script,
    governanceDriftDetected: (driftLabel) => `${auditCopy.sensitive}: ${driftLabel}`,
    transactionRolledBack: auditCopy.cancelled,
    planExecuted: auditCopy.completed,
  };
}
