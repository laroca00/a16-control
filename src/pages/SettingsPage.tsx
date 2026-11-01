import { useState } from "react";
import { api, type SetupStatus } from "../api";
import { useLang, useT, type Key, type Lang } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

const CHECKS: { key: Key; ok: (s: SetupStatus) => boolean }[] = [
  { key: "checkElevated", ok: (s) => s.elevated },
  { key: "checkTask", ok: (s) => s.task },
  { key: "checkWmiSet", ok: (s) => s.wmiSet },
  { key: "checkWmiGet", ok: (s) => s.wmiGet },
  { key: "checkKeyboardLight", ok: (s) => !s.keyboardLight },
];

const LANGS: { id: Lang; name: string }[] = [
  { id: "tr", name: "Türkçe" },
  { id: "en", name: "English" },
];

export function SettingsPage({ status }: { status: SetupStatus | null }) {
  const t = useT();
  const [lang, setLang] = useLang();
  const [error, setError] = useState<string | null>(null);

  async function uninstall() {
    try {
      await api.runUninstall();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="page page--settings">
      <h1 className="page__title">{t("navSettings")}</h1>
      <div className="settings">
        <LiquidGlass className="settings__card" radius={24}>
          <h2 className="card__title" id="lang-title">
            {t("language")}
          </h2>
          <div className="segmented" role="radiogroup" aria-labelledby="lang-title">
            {LANGS.map((l) => (
              <button
                key={l.id}
                type="button"
                role="radio"
                aria-checked={lang === l.id}
                className={`segmented__item${lang === l.id ? " segmented__item--on" : ""}`}
                onClick={() => setLang(l.id)}
              >
                {l.name}
              </button>
            ))}
          </div>
        </LiquidGlass>

        <LiquidGlass className="settings__card" radius={24}>
          <h2 className="card__title">{t("setupChecks")}</h2>
          <ul className="checks">
            {CHECKS.map((c) => {
              const ok = status ? c.ok(status) : null;
              return (
                <li key={c.key} className={`checks__row checks__row--${ok === null ? "unknown" : ok ? "ok" : "bad"}`}>
                  <span className="checks__mark" aria-hidden="true">
                    {ok === null ? "…" : ok ? "✓" : "✕"}
                  </span>
                  {t(c.key)}
                </li>
              );
            })}
          </ul>
        </LiquidGlass>

        <LiquidGlass className="settings__card" radius={24}>
          <h2 className="card__title">{t("uninstallTitle")}</h2>
          <p className="card__body">{t("uninstallBody")}</p>
          {error && <p className="card__error">{t("errorGeneric", { error })}</p>}
          <button type="button" className="danger-button" onClick={uninstall}>
            {t("uninstallAction")}
          </button>
        </LiquidGlass>
      </div>
    </div>
  );
}
