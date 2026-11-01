import { useState } from "react";
import { api } from "../api";
import { useT } from "../i18n";
import "./Notices.css";

export function SetupNotice() {
  const t = useT();
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function install() {
    setWorking(true);
    setError(null);
    try {
      // On success the app restarts elevated, so nothing more happens here.
      await api.runInstall();
    } catch (e) {
      setError(String(e));
      setWorking(false);
    }
  }

  return (
    <section className="notice" aria-live="polite">
      <h2 className="notice__title">{t("setupTitle")}</h2>
      <p className="notice__body">{t("setupBody")}</p>
      {error && <p className="notice__error">{t("setupFailed", { error })}</p>}
      <button type="button" className="notice__action" onClick={install} disabled={working}>
        {working ? t("setupWorking") : t("setupAction")}
      </button>
    </section>
  );
}

export function UnsupportedNotice() {
  const t = useT();
  return (
    <section className="notice notice--warn">
      <h2 className="notice__title">{t("unsupportedTitle")}</h2>
      <p className="notice__body">{t("unsupportedBody")}</p>
    </section>
  );
}

export function KeyboardLightNotice() {
  const t = useT();
  return (
    <section className="notice notice--warn">
      <h2 className="notice__title">{t("klTitle")}</h2>
      <p className="notice__body">{t("klBody")}</p>
    </section>
  );
}
