import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import de from "./locales/de.json";
import fr from "./locales/fr.json";
import es from "./locales/es.json";
import ptBR from "./locales/pt-BR.json";
import ru from "./locales/ru.json";

/** Supported UI locales (BCP 47 tags used in settings + i18next). */
export const SUPPORTED_LOCALES = [
  { id: "en", nativeLabel: "English" },
  { id: "zh-CN", nativeLabel: "简体中文" },
  { id: "zh-TW", nativeLabel: "繁體中文" },
  { id: "ja", nativeLabel: "日本語" },
  { id: "ko", nativeLabel: "한국어" },
  { id: "de", nativeLabel: "Deutsch" },
  { id: "fr", nativeLabel: "Français" },
  { id: "es", nativeLabel: "Español" },
  { id: "pt-BR", nativeLabel: "Português (Brasil)" },
  { id: "ru", nativeLabel: "Русский" },
] as const;

export type SupportedLocaleId = (typeof SUPPORTED_LOCALES)[number]["id"];

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "zh-CN": { translation: zhCN },
    "zh-TW": { translation: zhTW },
    ja: { translation: ja },
    ko: { translation: ko },
    de: { translation: de },
    fr: { translation: fr },
    es: { translation: es },
    "pt-BR": { translation: ptBR },
    ru: { translation: ru },
  },
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
