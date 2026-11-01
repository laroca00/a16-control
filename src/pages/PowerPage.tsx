import type { PowerOptions } from "../api";
import { ModeSwitch } from "../components/ModeSwitch";
import { usePower } from "../hooks/usePower";
import { useT, type Key } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

const LIMITS: { key: "fppt" | "sppt" | "spl"; label: Key }[] = [
  { key: "fppt", label: "limitBurst" },
  { key: "sppt", label: "limitSlow" },
  { key: "spl", label: "limitSustained" },
];

const OPTIONS: { key: keyof PowerOptions; label: Key }[] = [
  { key: "acdcSwitch", label: "optAcdc" },
  { key: "autoGame", label: "optGame" },
];

export function PowerPage() {
  const t = useT();
  const { power, setMode, setOption } = usePower();

  return (
    <div className="page page--power">
      <h1 className="page__title">{t("navPower")}</h1>
      <div className="settings">
        <LiquidGlass className="settings__card" radius={24}>
          <h2 className="card__title">{t("modeLabel")}</h2>
          <ModeSwitch power={power} onChange={setMode} size="large" />
          {power?.lastError && <p className="card__error">{t("powerFailed", { error: power.lastError })}</p>}
        </LiquidGlass>

        {power?.limits && (
          <LiquidGlass className="settings__card" radius={24}>
            <h2 className="card__title">{t("powerLimits")}</h2>
            <dl className="limits">
              {LIMITS.map((l) => (
                <div key={l.key}>
                  <dt>{t(l.label)}</dt>
                  <dd>{t("wattValue", { value: Math.round(power.limits![l.key] / 1000) })}</dd>
                </div>
              ))}
            </dl>
          </LiquidGlass>
        )}

        <LiquidGlass className="settings__card" radius={24}>
          {OPTIONS.map((o) => {
            const on = power?.options[o.key] ?? false;
            return (
              <label key={o.key} className="toggle">
                <span>{t(o.label)}</span>
                <button
                  type="button"
                  role="switch"
                  aria-checked={on}
                  aria-label={t(o.label)}
                  className={`toggle__switch${on ? " toggle__switch--on" : ""}`}
                  disabled={!power}
                  onClick={() => setOption(o.key, !on)}
                />
              </label>
            );
          })}
        </LiquidGlass>
      </div>
    </div>
  );
}
