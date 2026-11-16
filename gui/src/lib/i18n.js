import { writable, derived } from "svelte/store";
import { normalizeLocale, translate } from "./translations.js";

export const LOCALE_KEY = "djimic.locale";
let initialLocale = "en";
try {
  initialLocale = normalizeLocale(localStorage.getItem(LOCALE_KEY));
} catch {
  // Storage may be unavailable; English remains the default.
}
export const locale = writable(initialLocale);
export const t = derived(locale, (value) => (key, params) => translate(value, key, params));
export function selectLocale(value) {
  const next = normalizeLocale(value);
  locale.set(next);
  try { localStorage.setItem(LOCALE_KEY, next); } catch { /* session-only choice */ }
}
