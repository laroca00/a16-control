import { useEffect, useState } from "react";
import { api, type DisplayView } from "../api";
import { useT } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

export function DisplayPage() {
  const t = useT();
  const [d, setD] = useState<DisplayView | null>(null);
  // Brightness writes go through WMI one by one; send only where the drag ends.
  const [draft, setDraft] = useState<number | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .getDisplay()
      .then((v) => alive && setD(v))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  const run = (p: Promise<DisplayView>) => p.then(setD).catch(() => {});

  const commitBrightness = () => {
    if (draft == null) return;
    const value = draft;
    setDraft(null);
    setD((v) => (v ? { ...v, brightness: value } : v));
    run(api.setBrightness(value));
  };

  return (
    <div className="page page--display">
      <h1 className="page__title">{t("navDisplay")}</h1>
      <div className="settings">
        <LiquidGlass className="settings__card" radius={24}>
          <label className="kb-slider">
            <span>{t("dispBrightness")}</span>
            <input
              type="range"
              min={5}
              max={100}
              step={5}
              aria-label={t("dispBrightness")}
              value={draft ?? d?.brightness ?? 50}
              disabled={d?.brightness == null}
              onChange={(e) => setDraft(Number(e.target.value))}
              onPointerUp={commitBrightness}
              onKeyUp={commitBrightness}
              onBlur={commitBrightness}
            />
            <output>{draft ?? d?.brightness ?? "–"}%</output>
          </label>
        </LiquidGlass>

        {d && d.rates.length > 1 && (
          <LiquidGlass className="settings__card" radius={24}>
            <h2 className="card__title" id="refresh-title">
              {t("dispRefresh")}
            </h2>
            <div className="segmented" role="radiogroup" aria-labelledby="refresh-title">
              {d.rates.map((hz) => (
                <button
                  key={hz}
                  type="button"
                  role="radio"
                  aria-checked={d.refresh === hz}
                  className={`segmented__item${d.refresh === hz ? " segmented__item--on" : ""}`}
                  onClick={() => run(api.setRefreshRate(hz))}
                >
                  {t("hzValue", { value: hz })}
                </button>
              ))}
            </div>
          </LiquidGlass>
        )}

        <LiquidGlass className="settings__card" radius={24}>
          <p className="card__body">{t("dispOffHint")}</p>
          <button type="button" className="ghost-button" onClick={() => void api.screenOff()}>
            {t("dispOff")}
          </button>
        </LiquidGlass>
      </div>
    </div>
  );
}
