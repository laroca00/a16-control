import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type AbOptions, type AfterburnerView, type BatteryView, type GpuView, type PowerOptions } from "@/api";
import { ChargeSection, useHardware } from "@/components/HardwareSections";
import { ModeSwitch } from "@/components/ModeSwitch";
import { Button } from "@/components/ui/button";
import { Stat } from "@/components/ui/card";
import { Section, Sections } from "@/components/ui/section";
import { SwitchRow } from "@/components/ui/switch";
import { usePower } from "@/hooks/usePower";
import { useT, type Key } from "@/i18n";

const LIMITS: { key: "fppt" | "sppt" | "spl"; label: Key }[] = [
  { key: "fppt", label: "limitBurst" },
  { key: "sppt", label: "limitSlow" },
  { key: "spl", label: "limitSustained" },
];

const OPTIONS: { key: keyof PowerOptions; label: Key }[] = [
  { key: "acdcSwitch", label: "optAcdc" },
  { key: "autoGame", label: "optGame" },
];

const errorText = "text-[13px] text-destructive-foreground";

export function PowerPage() {
  const t = useT();
  const { power, setMode, setOption } = usePower();
  const [gpu, setGpu] = useState<GpuView | null>(null);
  const [ab, setAb] = useState<AfterburnerView | null>(null);
  const [battery, setBattery] = useState<BatteryView | null>(null);
  const [watchError, setWatchError] = useState(false);
  const hardware = useHardware();

  useEffect(() => {
    api.getGpu().then(setGpu).catch(() => {});
    api.getAfterburner().then(setAb).catch(() => {});
    api.getBattery().then(setBattery).catch(() => {});
    const un = listen<BatteryView>("battery", (e) => setBattery(e.payload));
    return () => void un.then((f) => f());
  }, []);

  const startWatch = () => {
    setWatchError(false);
    api
      .startWatch()
      .then(setBattery)
      .catch(() => setWatchError(true));
  };

  const changeAb = (patch: Partial<AbOptions>) => {
    if (!ab) return;
    api.setAfterburner({ ...ab.options, ...patch }).then(setAb).catch(() => {});
  };

  const gpuLine = gpu?.state === "d0" ? t("gpuAwake") : gpu?.state === "d3" ? t("gpuAsleep") : t("gpuUnknown");

  return (
    <Sections>
      <Section title={t("modeLabel")}>
        <ModeSwitch power={power} onChange={setMode} size="large" />
        {power?.lastError && <p className={errorText}>{t("powerFailed", { error: power.lastError })}</p>}
      </Section>

      {power?.limits && (
        <Section title={t("powerLimits")} description={t("powerLimitsBody")}>
          <dl className="grid w-full grid-cols-3 gap-2">
            {LIMITS.map((l) => (
              <Stat key={l.key} label={t(l.label)} value={t("wattValue", { value: Math.round(power.limits![l.key] / 1000) })} />
            ))}
          </dl>
        </Section>
      )}

      <Section title={t("powerSwitching")}>
        {OPTIONS.map((o) => (
          <SwitchRow
            key={o.key}
            label={t(o.label)}
            checked={power?.options[o.key] ?? false}
            disabled={!power}
            onCheckedChange={(v) => setOption(o.key, v)}
          />
        ))}
      </Section>

      <ChargeSection hardware={hardware} />

      <Section title={t("gpuTitle")} description={gpuLine}>
        <SwitchRow
          label={t("gpuIgpu")}
          checked={gpu?.igpuOnBattery ?? false}
          disabled={!gpu}
          onCheckedChange={(v) => api.setGpuOption("igpuOnBattery", v).then(setGpu)}
        />
        <SwitchRow
          label={t("gpuReset")}
          checked={gpu?.autoReset ?? false}
          disabled={!gpu}
          onCheckedChange={(v) => api.setGpuOption("autoReset", v).then(setGpu)}
        />
      </Section>

      <Section title={t("watchTitle")} description={t("watchBody")}>
        {battery?.watching ? (
          <>
            <div
              className="h-1.5 w-full overflow-hidden rounded-full bg-muted"
              role="progressbar"
              aria-valuenow={Math.round(battery.progress * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
            >
              <span
                className="block h-full rounded-full bg-primary transition-[width] duration-700"
                style={{ width: `${battery.progress * 100}%` }}
              />
            </div>
            <p className="font-mono text-xs text-muted-foreground tabular-nums">
              {t("watchProgress", { pct: Math.round(battery.progress * 100) })}
            </p>
            <Button variant="secondary" onClick={() => api.cancelWatch().then(setBattery)}>
              {t("watchCancel")}
            </Button>
          </>
        ) : (
          <Button variant="secondary" disabled={!battery} onClick={startWatch}>
            {t("watchStart")}
          </Button>
        )}
        {watchError && <p className={errorText}>{t("watchOnAc")}</p>}
        {battery?.summary && (
          <dl className="grid w-full grid-cols-3 gap-2">
            <Stat
              label={t("watchAvgPower")}
              value={battery.summary.avgWatts == null ? "–" : t("wattValue", { value: battery.summary.avgWatts.toFixed(1) })}
            />
            <Stat label={t("watchAvgCpu")} value={t("pctValue", { value: Math.round(battery.summary.avgCpu) })} />
            <Stat label={t("watchIdle")} value={t("pctValue", { value: Math.round(battery.summary.idlePct) })} />
          </dl>
        )}
        {battery && battery.targets.length > 0 ? (
          <div className="flex w-full flex-col gap-3">
            <h3 className="text-[13px] font-medium">{t("watchTargets")}</h3>
            <ul className="flex flex-wrap gap-2">
              {battery.targets.map((name) => (
                <li key={name} className="rounded-md bg-muted px-2 py-1 font-mono text-xs text-muted-foreground">
                  {name}
                </li>
              ))}
            </ul>
            <SwitchRow
              label={t("ecoToggle")}
              checked={battery.ecoEnabled}
              onCheckedChange={(v) => api.setEcoEnabled(v).then(setBattery)}
            />
          </div>
        ) : (
          battery?.summary && <p className="text-[13px] text-muted-foreground">{t("watchNoTargets")}</p>
        )}
      </Section>

      {ab?.installed && (
        <Section title="MSI Afterburner" description={t("abHint")}>
          <SwitchRow label={t("abEnable")} checked={ab.options.enabled} onCheckedChange={(v) => changeAb({ enabled: v })} />
          {ab.options.enabled && (
            <div className="flex gap-4">
              {(["ocProfile", "stockProfile"] as const).map((key) => (
                <label key={key} className="flex items-center gap-2 text-[13px]">
                  <span>{t(key === "ocProfile" ? "abOc" : "abStock")}</span>
                  <select
                    className="h-8 cursor-pointer rounded-md border border-input bg-background px-2 font-mono text-[13px]"
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
        </Section>
      )}
    </Sections>
  );
}
