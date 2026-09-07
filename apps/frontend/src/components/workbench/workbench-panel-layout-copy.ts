// Small shell-control copy stays available before the larger language chunks load.
const COPY: Record<string, readonly [string, string]> = {
  en: ["Drag to resize; double-click to reset this panel.", "Restore layout"],
  zh: ["拖动调整大小；双击恢复此分区。", "恢复默认布局"],
  "zh-tw": ["拖曳調整大小；按兩下還原此分區。", "還原預設版面"],
  ja: ["ドラッグでサイズ変更、ダブルクリックでこのパネルをリセット。", "レイアウトを戻す"],
  es: ["Arrastra para ajustar; haz doble clic para restablecer este panel.", "Restablecer diseño"],
  ar: ["اسحب لتغيير الحجم؛ انقر مرتين لإعادة ضبط هذه اللوحة.", "استعادة التخطيط"],
  bn: ["আকার বদলাতে টানুন; এই প্যানেল রিসেট করতে ডাবল-ক্লিক করুন।", "বিন্যাস পুনরুদ্ধার করুন"],
  cs: ["Tažením změníte velikost; dvojklikem obnovíte tento panel.", "Obnovit rozložení"],
  da: ["Træk for at ændre størrelse; dobbeltklik for at nulstille panelet.", "Gendan layout"],
  de: ["Zum Skalieren ziehen; Doppelklick setzt diesen Bereich zurück.", "Layout zurücksetzen"],
  el: ["Σύρετε για αλλαγή μεγέθους· διπλό κλικ για επαναφορά του πίνακα.", "Επαναφορά διάταξης"],
  fa: ["برای تغییر اندازه بکشید؛ برای بازنشانی این پنل دوبار کلیک کنید.", "بازیابی چیدمان"],
  fi: ["Muuta kokoa vetämällä; palauta paneeli kaksoisnapsauttamalla.", "Palauta asettelu"],
  fr: ["Faites glisser pour redimensionner ; double-cliquez pour réinitialiser ce panneau.", "Rétablir la disposition"],
  he: ["גררו לשינוי הגודל; לחצו פעמיים לאיפוס הלוח הזה.", "שחזור הפריסה"],
  hi: ["आकार बदलने के लिए खींचें; इस पैनल को रीसेट करने के लिए डबल-क्लिक करें।", "लेआउट पुनर्स्थापित करें"],
  id: ["Seret untuk mengubah ukuran; klik dua kali untuk mereset panel ini.", "Pulihkan tata letak"],
  it: ["Trascina per ridimensionare; fai doppio clic per ripristinare questo pannello.", "Ripristina disposizione"],
  ko: ["드래그하여 크기 조절; 두 번 클릭하여 이 패널 초기화.", "레이아웃 복원"],
  ms: ["Seret untuk mengubah saiz; klik dua kali untuk menetapkan semula panel ini.", "Pulihkan susun atur"],
  nl: ["Sleep om te schalen; dubbelklik om dit paneel te herstellen.", "Indeling herstellen"],
  no: ["Dra for å endre størrelse; dobbeltklikk for å tilbakestille panelet.", "Gjenopprett oppsett"],
  pl: ["Przeciągnij, aby zmienić rozmiar; kliknij dwukrotnie, aby zresetować panel.", "Przywróć układ"],
  "pt-br": ["Arraste para redimensionar; clique duas vezes para redefinir este painel.", "Restaurar layout"],
  ro: ["Trageți pentru redimensionare; faceți dublu clic pentru a reseta panoul.", "Restabiliți aspectul"],
  ru: ["Перетащите для изменения размера; двойной щелчок сбрасывает эту панель.", "Восстановить раскладку"],
  sv: ["Dra för att ändra storlek; dubbelklicka för att återställa panelen.", "Återställ layout"],
  sw: ["Buruta kubadili ukubwa; bofya mara mbili kurejesha paneli hii.", "Rejesha mpangilio"],
  ta: ["அளவை மாற்ற இழுக்கவும்; இந்தப் பலகையை மீட்டமைக்க இருமுறை சொடுக்கவும்.", "தளவமைப்பை மீட்டமை"],
  th: ["ลากเพื่อปรับขนาด ดับเบิลคลิกเพื่อรีเซ็ตแผงนี้", "คืนค่าเค้าโครง"],
  tr: ["Boyutlandırmak için sürükleyin; paneli sıfırlamak için çift tıklayın.", "Düzeni geri yükle"],
  uk: ["Перетягніть для зміни розміру; подвійне клацання скидає цю панель.", "Відновити компонування"],
  ur: ["سائز بدلنے کے لیے کھینچیں؛ اس پینل کو ری سیٹ کرنے کے لیے ڈبل کلک کریں۔", "لے آؤٹ بحال کریں"],
  vi: ["Kéo để đổi kích thước; nhấp đúp để đặt lại bảng này.", "Khôi phục bố cục"],
};

export function getWorkbenchPanelLayoutCopy(language = "en") {
  const [resize, reset] = COPY[language.toLowerCase()] ?? COPY.en;
  return { resize, reset };
}
