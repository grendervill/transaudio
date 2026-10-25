import type { ModelCard } from "../types";

export const LANGUAGE_NAMES: Record<string, string> = {
  ru: "русский",
  en: "английский",
  de: "немецкий",
  fr: "французский",
  es: "испанский",
  it: "итальянский",
  pt: "португальский",
  uk: "украинский",
  pl: "польский",
  nl: "нидерландский",
  tr: "турецкий",
  ar: "арабский",
  zh: "китайский",
  ja: "японский",
  ko: "корейский",
  yue: "кантонский",
  bg: "болгарский",
  hr: "хорватский",
  cs: "чешский",
  da: "датский",
  et: "эстонский",
  fi: "финский",
  el: "греческий",
  hu: "венгерский",
  lv: "латышский",
  lt: "литовский",
  mt: "мальтийский",
  ro: "румынский",
  sk: "словацкий",
  sl: "словенский",
  sv: "шведский",
  vi: "вьетнамский",
};

export function languageName(code: string): string {
  return LANGUAGE_NAMES[code] ?? code;
}

export function supportsLanguage(card: ModelCard, language: "ru" | "en"): boolean {
  return card.languages.includes(language);
}

export function isAsian(card: ModelCard): boolean {
  return card.languages.some((code) => ["zh", "ja", "ko", "yue"].includes(code));
}

/** Five-step speed scale from the CPU real-time factor. */
export function speedScore(realtime: number): number {
  if (realtime >= 25) return 5;
  if (realtime >= 12) return 4;
  if (realtime >= 6) return 3;
  if (realtime >= 3) return 2;
  return 1;
}

/** Best WER among the languages this card supports; lower is better. */
export function qualityWer(card: ModelCard): number | null {
  const values = [card.quality_ru, card.quality_en].filter(
    (value): value is number => typeof value === "number",
  );
  return values.length ? Math.min(...values) : null;
}

export function qualityScore(card: ModelCard): number | null {
  const wer = qualityWer(card);
  if (wer === null) return null;
  if (wer <= 4.5) return 5;
  if (wer <= 5.5) return 4;
  if (wer <= 6.5) return 3;
  if (wer <= 8) return 2;
  return 1;
}

export function languageSummary(card: ModelCard): string {
  const names = card.languages.map(languageName);
  if (card.language_count <= 1) {
    return names[0] ?? "—";
  }
  if (card.language_count <= 6) {
    return names.join(", ");
  }
  return `${card.language_count} языков · ${names.slice(0, 4).join(", ")}…`;
}

export function recommendationLabel(card: ModelCard): string | null {
  if (card.recommended_for.includes("ru")) return "Рекомендую · русский";
  if (card.recommended_for.includes("en")) return "Рекомендую · английский";
  return null;
}

export function werSummary(card: ModelCard): string | null {
  const parts: string[] = [];
  if (typeof card.quality_en === "number") {
    parts.push(`EN ${card.quality_en.toFixed(1).replace(".", ",")}%`);
  }
  if (typeof card.quality_ru === "number") {
    parts.push(`RU ${card.quality_ru.toFixed(1).replace(".", ",")}%`);
  }
  return parts.length ? parts.join(" · ") : null;
}
