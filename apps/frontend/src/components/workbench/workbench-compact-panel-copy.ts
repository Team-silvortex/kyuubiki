type Copy = { search: string; noMatches: string; filters: string; events: string; facets: string };

const copies: Record<string, Copy> = {
  en: { search: "Search", noMatches: "No matching items.", filters: "Filters", events: "Events", facets: "Breakdown" },
  zh: { search: "搜索", noMatches: "没有匹配的条目。", filters: "筛选", events: "事件", facets: "分类统计" },
  ja: { search: "検索", noMatches: "一致する項目はありません。", filters: "絞り込み", events: "イベント", facets: "内訳" },
  es: { search: "Buscar", noMatches: "No hay elementos coincidentes.", filters: "Filtros", events: "Eventos", facets: "Desglose" },
  ar: { search: "بحث", noMatches: "لا توجد عناصر مطابقة.", filters: "عوامل التصفية", events: "الأحداث", facets: "التوزيع" },
  bn: { search: "খুঁজুন", noMatches: "কোনো মিল পাওয়া যায়নি।", filters: "ফিল্টার", events: "ইভেন্ট", facets: "বিভাগভিত্তিক পরিসংখ্যান" },
  cs: { search: "Hledat", noMatches: "Žádné odpovídající položky.", filters: "Filtry", events: "Události", facets: "Rozdělení" },
  da: { search: "Søg", noMatches: "Ingen matchende elementer.", filters: "Filtre", events: "Hændelser", facets: "Fordeling" },
  de: { search: "Suchen", noMatches: "Keine passenden Einträge.", filters: "Filter", events: "Ereignisse", facets: "Aufschlüsselung" },
  el: { search: "Αναζήτηση", noMatches: "Δεν υπάρχουν στοιχεία που να ταιριάζουν.", filters: "Φίλτρα", events: "Συμβάντα", facets: "Κατανομή" },
  fa: { search: "جستجو", noMatches: "مورد منطبقی یافت نشد.", filters: "فیلترها", events: "رویدادها", facets: "تفکیک آماری" },
  fi: { search: "Hae", noMatches: "Ei vastaavia kohteita.", filters: "Suodattimet", events: "Tapahtumat", facets: "Erittely" },
  fr: { search: "Rechercher", noMatches: "Aucun élément correspondant.", filters: "Filtres", events: "Événements", facets: "Répartition" },
  he: { search: "חיפוש", noMatches: "לא נמצאו פריטים תואמים.", filters: "מסננים", events: "אירועים", facets: "פילוח" },
  hi: { search: "खोजें", noMatches: "कोई मेल खाता आइटम नहीं मिला।", filters: "फ़िल्टर", events: "इवेंट", facets: "वर्गीकरण" },
  id: { search: "Cari", noMatches: "Tidak ada item yang cocok.", filters: "Filter", events: "Peristiwa", facets: "Rincian" },
  it: { search: "Cerca", noMatches: "Nessun elemento corrispondente.", filters: "Filtri", events: "Eventi", facets: "Ripartizione" },
  ko: { search: "검색", noMatches: "일치하는 항목이 없습니다.", filters: "필터", events: "이벤트", facets: "분류 통계" },
  ms: { search: "Cari", noMatches: "Tiada item yang sepadan.", filters: "Penapis", events: "Peristiwa", facets: "Pecahan" },
  nl: { search: "Zoeken", noMatches: "Geen overeenkomende items.", filters: "Filters", events: "Gebeurtenissen", facets: "Uitsplitsing" },
  no: { search: "Søk", noMatches: "Ingen samsvarende elementer.", filters: "Filtre", events: "Hendelser", facets: "Fordeling" },
  pl: { search: "Szukaj", noMatches: "Brak pasujących elementów.", filters: "Filtry", events: "Zdarzenia", facets: "Podział" },
  "pt-br": { search: "Pesquisar", noMatches: "Nenhum item correspondente.", filters: "Filtros", events: "Eventos", facets: "Distribuição" },
  ro: { search: "Caută", noMatches: "Nu există elemente corespunzătoare.", filters: "Filtre", events: "Evenimente", facets: "Distribuție" },
  ru: { search: "Поиск", noMatches: "Совпадающих элементов нет.", filters: "Фильтры", events: "События", facets: "Распределение" },
  sv: { search: "Sök", noMatches: "Inga matchande objekt.", filters: "Filter", events: "Händelser", facets: "Fördelning" },
  sw: { search: "Tafuta", noMatches: "Hakuna vipengee vinavyolingana.", filters: "Vichujio", events: "Matukio", facets: "Mgawanyo" },
  ta: { search: "தேடு", noMatches: "பொருந்தும் உருப்படிகள் இல்லை.", filters: "வடிப்பான்கள்", events: "நிகழ்வுகள்", facets: "வகைப்பாடு" },
  th: { search: "ค้นหา", noMatches: "ไม่พบรายการที่ตรงกัน", filters: "ตัวกรอง", events: "เหตุการณ์", facets: "การแจกแจง" },
  tr: { search: "Ara", noMatches: "Eşleşen öğe yok.", filters: "Filtreler", events: "Olaylar", facets: "Dağılım" },
  uk: { search: "Пошук", noMatches: "Відповідних елементів немає.", filters: "Фільтри", events: "Події", facets: "Розподіл" },
  ur: { search: "تلاش", noMatches: "کوئی مماثل آئٹم نہیں ملا۔", filters: "فلٹر", events: "واقعات", facets: "درجہ وار تفصیل" },
  vi: { search: "Tìm kiếm", noMatches: "Không có mục phù hợp.", filters: "Bộ lọc", events: "Sự kiện", facets: "Phân loại" },
  "zh-tw": { search: "搜尋", noMatches: "沒有符合的項目。", filters: "篩選", events: "事件", facets: "分類統計" },
};

export function getWorkbenchCompactPanelCopy(language: string): Copy {
  const key = language.toLowerCase();
  return Object.hasOwn(copies, key) ? copies[key] : copies.en;
}
