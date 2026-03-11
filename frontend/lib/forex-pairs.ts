export type ForexCategory =
  | "Majors"
  | "Euro Crosses"
  | "Yen Crosses"
  | "Commodity Crosses"
  | "Sterling Crosses"
  | "Nordics"
  | "Emerging";

export interface ForexPair {
  symbol: string;
  base: string;
  quote: string;
  category: ForexCategory;
  session: string;
  volatility: "Core" | "Balanced" | "Fast" | "High";
  note: string;
}

export const FOREX_CATEGORY_ORDER: ForexCategory[] = [
  "Majors",
  "Euro Crosses",
  "Yen Crosses",
  "Commodity Crosses",
  "Sterling Crosses",
  "Nordics",
  "Emerging",
];

export const FOREX_PAIRS: ForexPair[] = [
  { symbol: "EUR/USD", base: "EUR", quote: "USD", category: "Majors", session: "London / New York", volatility: "Core", note: "Thanh khoan cao, benchmark cua forex spot." },
  { symbol: "GBP/USD", base: "GBP", quote: "USD", category: "Majors", session: "London / New York", volatility: "Fast", note: "Nhay voi du lieu UK va USD." },
  { symbol: "AUD/USD", base: "AUD", quote: "USD", category: "Majors", session: "Sydney / Asia", volatility: "Balanced", note: "Hay gan voi risk sentiment va commodities." },
  { symbol: "NZD/USD", base: "NZD", quote: "USD", category: "Majors", session: "Sydney / Asia", volatility: "Balanced", note: "Thanh khoan thap hon AUD/USD, phan ung nhanh voi carry." },
  { symbol: "USD/JPY", base: "USD", quote: "JPY", category: "Majors", session: "Asia / New York", volatility: "Core", note: "Rat nhay voi loi suat va risk-off." },
  { symbol: "USD/CHF", base: "USD", quote: "CHF", category: "Majors", session: "London / New York", volatility: "Balanced", note: "Co tinh chat safe haven cung CHF." },
  { symbol: "USD/CAD", base: "USD", quote: "CAD", category: "Majors", session: "New York", volatility: "Balanced", note: "Lien he chat voi gia dau va du lieu Bac My." },
  { symbol: "EUR/GBP", base: "EUR", quote: "GBP", category: "Euro Crosses", session: "London", volatility: "Balanced", note: "Do chenh suc manh giua khu vuc Euro va UK." },
  { symbol: "EUR/CHF", base: "EUR", quote: "CHF", category: "Euro Crosses", session: "London", volatility: "Balanced", note: "Cross phong thu, nhay voi safe haven demand." },
  { symbol: "EUR/CAD", base: "EUR", quote: "CAD", category: "Euro Crosses", session: "London / New York", volatility: "Balanced", note: "Can bang giua Eurozone va hang hoa." },
  { symbol: "EUR/AUD", base: "EUR", quote: "AUD", category: "Euro Crosses", session: "Asia / London", volatility: "Fast", note: "Hay co nhung con song trend ro." },
  { symbol: "EUR/NZD", base: "EUR", quote: "NZD", category: "Euro Crosses", session: "Asia / London", volatility: "High", note: "Volatility cao, spread lon hon nhom majors." },
  { symbol: "EUR/JPY", base: "EUR", quote: "JPY", category: "Yen Crosses", session: "Asia / London", volatility: "Fast", note: "Ket hop giua risk sentiment va du lieu chau Au." },
  { symbol: "GBP/JPY", base: "GBP", quote: "JPY", category: "Yen Crosses", session: "Asia / London", volatility: "High", note: "Mot trong nhung cap bien dong manh nhat." },
  { symbol: "AUD/JPY", base: "AUD", quote: "JPY", category: "Yen Crosses", session: "Asia", volatility: "Fast", note: "Thuoc do risk-on / risk-off o chau A." },
  { symbol: "NZD/JPY", base: "NZD", quote: "JPY", category: "Yen Crosses", session: "Asia", volatility: "Fast", note: "De theo doi carry trade sentiment." },
  { symbol: "CAD/JPY", base: "CAD", quote: "JPY", category: "Yen Crosses", session: "Asia / New York", volatility: "Fast", note: "Ket hop risk sentiment va gia dau." },
  { symbol: "CHF/JPY", base: "CHF", quote: "JPY", category: "Yen Crosses", session: "Asia / Europe", volatility: "Balanced", note: "Cross safe haven, hay dung de do do risk-off." },
  { symbol: "AUD/CAD", base: "AUD", quote: "CAD", category: "Commodity Crosses", session: "Asia / New York", volatility: "Balanced", note: "Cross hang hoa voi hai dong tien nhay voi commodities." },
  { symbol: "AUD/NZD", base: "AUD", quote: "NZD", category: "Commodity Crosses", session: "Asia", volatility: "Balanced", note: "Cross noi dia chau Dai Duong." },
  { symbol: "NZD/CAD", base: "NZD", quote: "CAD", category: "Commodity Crosses", session: "Asia / New York", volatility: "Balanced", note: "Cross thanh khoan vua, de bat divergence giua dairy va oil sentiment." },
  { symbol: "GBP/CHF", base: "GBP", quote: "CHF", category: "Sterling Crosses", session: "London", volatility: "Fast", note: "Do chenh risk premium giua UK va safe haven CHF." },
  { symbol: "GBP/AUD", base: "GBP", quote: "AUD", category: "Sterling Crosses", session: "Asia / London", volatility: "High", note: "Bien dong lon, hay can trend confirmation." },
  { symbol: "GBP/CAD", base: "GBP", quote: "CAD", category: "Sterling Crosses", session: "London / New York", volatility: "Fast", note: "Anh huong boi du lieu UK va energy market." },
  { symbol: "GBP/NZD", base: "GBP", quote: "NZD", category: "Sterling Crosses", session: "Asia / London", volatility: "High", note: "Spread rong, phu hop bo loc confidence cao." },
  { symbol: "EUR/SEK", base: "EUR", quote: "SEK", category: "Nordics", session: "London", volatility: "Balanced", note: "Theo doi dong krona trong khung chau Au." },
  { symbol: "EUR/NOK", base: "EUR", quote: "NOK", category: "Nordics", session: "London", volatility: "Balanced", note: "Lien quan den oil beta va chenh lech chau Au." },
  { symbol: "USD/SEK", base: "USD", quote: "SEK", category: "Nordics", session: "London / New York", volatility: "Balanced", note: "Do suc manh USD so voi dong krona Thuy Dien." },
  { symbol: "USD/NOK", base: "USD", quote: "NOK", category: "Nordics", session: "London / New York", volatility: "Balanced", note: "Do suc manh USD so voi dong kroner Na Uy." },
  { symbol: "USD/SGD", base: "USD", quote: "SGD", category: "Emerging", session: "Asia", volatility: "Balanced", note: "Mot cap emerging chat luong cao o chau A." },
  { symbol: "USD/HKD", base: "USD", quote: "HKD", category: "Emerging", session: "Asia", volatility: "Core", note: "Thuong on dinh hon do che do neo." },
  { symbol: "USD/CNH", base: "USD", quote: "CNH", category: "Emerging", session: "Asia", volatility: "Fast", note: "Dai dien ky vong ve Trung Quoc va USD Asia." },
  { symbol: "USD/MXN", base: "USD", quote: "MXN", category: "Emerging", session: "New York", volatility: "High", note: "Mot trong cac cap carry pho bien nhat." },
  { symbol: "USD/ZAR", base: "USD", quote: "ZAR", category: "Emerging", session: "London / New York", volatility: "High", note: "Rat nhay voi sentiment va kim loai." },
  { symbol: "USD/TRY", base: "USD", quote: "TRY", category: "Emerging", session: "Europe", volatility: "High", note: "Can theo doi rui ro chinh sach va thanh khoan." },
  { symbol: "USD/PLN", base: "USD", quote: "PLN", category: "Emerging", session: "Europe", volatility: "Balanced", note: "Cross Dong Au co tinh chat macro ro rang." },
  { symbol: "USD/HUF", base: "USD", quote: "HUF", category: "Emerging", session: "Europe", volatility: "Balanced", note: "Theo doi chenh lech loi suat va risk premium." },
  { symbol: "USD/CZK", base: "USD", quote: "CZK", category: "Emerging", session: "Europe", volatility: "Balanced", note: "Cross thin thanh khoan hon, phu hop watchlist chon loc." },
  { symbol: "USD/THB", base: "USD", quote: "THB", category: "Emerging", session: "Asia", volatility: "Balanced", note: "Lien quan den dong von chau A va du lich." },
  { symbol: "USD/KRW", base: "USD", quote: "KRW", category: "Emerging", session: "Asia", volatility: "Fast", note: "Nhạy với tăng trưởng và xuất khẩu châu Á." },
  { symbol: "USD/INR", base: "USD", quote: "INR", category: "Emerging", session: "Asia", volatility: "Balanced", note: "Theo doi dong rupee va can bang can thanh toan." },
  { symbol: "USD/BRL", base: "USD", quote: "BRL", category: "Emerging", session: "New York", volatility: "High", note: "Volatility cao, nhay voi commodities va risk premium." },
];

export function recommendedSourceUrls(symbol: string): string[] {
  const compact = symbol.replace("/", "").toLowerCase();
  const investing = symbol.replace("/", "-").toLowerCase();

  return [
    `https://www.fxstreet.com/rates-charts/${compact}`,
    `https://www.investing.com/currencies/${investing}`,
    `https://www.tradingview.com/symbols/${compact.toUpperCase()}/ideas/`,
  ];
}
