import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { api } from "../api";
import { en } from "./en";
import { tr } from "./tr";

export type Lang = "tr" | "en";
export type Key = keyof typeof en;
type Vars = Record<string, string | number>;

const dictionaries: Record<Lang, Record<Key, string>> = { en, tr };

export function format(template: string, vars?: Vars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in vars ? String(vars[name]) : match));
}

function systemLang(): Lang {
  return navigator.language.toLowerCase().startsWith("tr") ? "tr" : "en";
}

type I18n = { lang: Lang; setLang: (l: Lang) => void };

const I18nContext = createContext<I18n>({ lang: "en", setLang: () => {} });

export function I18nProvider({ children, initial }: { children: ReactNode; initial?: Lang }) {
  const [lang, setLangState] = useState<Lang>(initial ?? systemLang());

  useEffect(() => {
    if (initial) return;
    api
      .getConfig()
      .then((c) => {
        if (c.lang === "tr" || c.lang === "en") setLangState(c.lang);
      })
      .catch(() => {});
  }, [initial]);

  const setLang = useCallback((l: Lang) => {
    setLangState(l);
    api.setLang(l).catch(() => {});
  }, []);

  return <I18nContext.Provider value={{ lang, setLang }}>{children}</I18nContext.Provider>;
}

export function useT(): (key: Key, vars?: Vars) => string {
  const { lang } = useContext(I18nContext);
  return useCallback((key: Key, vars?: Vars) => format(dictionaries[lang][key], vars), [lang]);
}

export function useLang(): [Lang, (l: Lang) => void] {
  const { lang, setLang } = useContext(I18nContext);
  return [lang, setLang];
}
