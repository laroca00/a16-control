import { useEffect, useState } from "react";
import { api, type AbOptions, type AfterburnerView, type GpuView, type PowerOptions } from "../api";
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

function Toggle({ label, on, disabled, onChange }: { label: string; on: boolean; disabled: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="toggle">
      <span>{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={on}
        aria-label={label}
        className={`toggle__switch${on ? " toggle__switch--on" : ""}`}
        disabled={disabled}
        onClick={() => onChange(!on)}
      />
    </label>
  );
}

export function PowerPage() {
  const t = useT();
  const { power, setMode, setOption } = usePower();
  const [gpu, setGpu] = useState<GpuView | null>(null);
  const [ab, setAb] = useState<AfterburnerView | null>(null);

  useEffect(() => {
    api.getGpu().then(setGpu).catch(() => {});
    api.getAfterburner().then(setAb).catch(() => {});
  }, []);

  const changeAb = (patch: Partial<AbOptions>) => {
    if (!ab) return;
    api.setAfterburner({ ...ab.options, ...patch }).then(setAb).catch(() => {});
  };

  const gpuLine = gpu?.state === "d0" ? t("gpuAwake") : gpu?.state === "d3" ? t("gpuAsleep") : t("gpuUnknown");

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
          {OPTIONS.map((o) => (
            <Toggle
              key={o.key}
              label={t(o.label)}
              on={power?.options[o.key] ?? false}
              disabled={!power}
              onChange={(v) => setOption(o.key, v)}
            />
          ))}
        </LiquidGlass>

        <LiquidGlass className="settings__card" radius={24}>
          <h2 className="card__title">{t("gpuTitle")}</h2>
          <p className="card__body">{gpuLine}</p>
          <Toggle
            label={t("gpuIgpu")}
            on={gpu?.igpuOnBattery ?? false}
            disabled={!gpu}
            onChange={(v) => api.setGpuOption("igpuOnBattery", v).then(setGpu)}
          />
          <Toggle
            label={t("gpuReset")}
            on={gpu?.autoReset ?? false}
            disabled={!gpu}
            onChange={(v) => api.setGpuOption("autoReset", v).then(setGpu)}
          />
        </LiquidGlass>

        {ab?.installed && (
          <LiquidGlass className="settings__card" radius={24}>
            <h2 className="card__title">MSI Afterburner</h2>
            <Toggle label={t("abEnable")} on={ab.options.enabled} disabled={false} onChange={(v) => changeAb({ enabled: v })} />
            {ab.options.enabled && (
              <div className="ab-profiles">
                {(["ocProfile", "stockProfile"] as const).map((key) => (
                  <label key={key} className="ab-profile">
                    <span>{t(key === "ocProfile" ? "abOc" : "abStock")}</span>
                    <select
                      aria-label={t(key === "ocProfile" ? "abOc" : "abStock")}
                      value={ab.options[key]}
                      onChange={(e) => changeAb({ [key]: Number(e.target.value) })}
                    >
                      {[1, 2, 3, 4, 5].map((n) => (
                        <option key={n} value={n}>
                          {n}
                        </option>
                      ))}
                    </select>
                  </label>
                ))}
              </div>
            )}
            <p className="card__hint">{t("abHint")}</p>
          </LiquidGlass>
        )}
      </div>
    </div>
  );
}
