import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import deAccount from "./locales/de/account.json";
import deCommon from "./locales/de/common.json";
import deEditor from "./locales/de/editor.json";
import deEntries from "./locales/de/entries.json";
import deErrors from "./locales/de/errors.json";
import deProjects from "./locales/de/projects.json";
import deSchema from "./locales/de/schema.json";
import deSettings from "./locales/de/settings.json";
import deShare from "./locales/de/share.json";
import deShortcuts from "./locales/de/shortcuts.json";
import deSync from "./locales/de/sync.json";
import enAccount from "./locales/en/account.json";
import enCommon from "./locales/en/common.json";
import enEditor from "./locales/en/editor.json";
import enEntries from "./locales/en/entries.json";
import enErrors from "./locales/en/errors.json";
import enProjects from "./locales/en/projects.json";
import enSchema from "./locales/en/schema.json";
import enSettings from "./locales/en/settings.json";
import enShare from "./locales/en/share.json";
import enShortcuts from "./locales/en/shortcuts.json";
import enSync from "./locales/en/sync.json";

export const namespaces = [
  "common",
  "entries",
  "schema",
  "editor",
  "projects",
  "account",
  "sync",
  "share",
  "settings",
  "errors",
  "shortcuts",
] as const;

const resources = {
  en: {
    common: enCommon,
    entries: enEntries,
    schema: enSchema,
    editor: enEditor,
    projects: enProjects,
    account: enAccount,
    sync: enSync,
    share: enShare,
    settings: enSettings,
    errors: enErrors,
    shortcuts: enShortcuts,
  },
  de: {
    common: deCommon,
    entries: deEntries,
    schema: deSchema,
    editor: deEditor,
    projects: deProjects,
    account: deAccount,
    sync: deSync,
    share: deShare,
    settings: deSettings,
    errors: deErrors,
    shortcuts: deShortcuts,
  },
} as const;

export function detectLanguage(): "en" | "de" {
  return typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("de")
    ? "de"
    : "en";
}

export function resolveLanguage(language: string): "en" | "de" {
  if (language === "en" || language === "de") {
    return language;
  }
  return detectLanguage();
}

void i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: detectLanguage(),
    fallbackLng: "en",
    supportedLngs: ["en", "de"],
    ns: [...namespaces],
    defaultNS: "common",
    interpolation: { escapeValue: false },
    returnNull: false,
  })
  .then(() => {
    document.documentElement.lang = i18n.language;
  });

i18n.on("languageChanged", (language) => {
  document.documentElement.lang = language;
});

export default i18n;
