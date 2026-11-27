import { useEffect, useState } from "react";
import { api, type GpuMode, type HardwareView, type Toggle } from "@/api";
import { Button } from "@/components/ui/button";
import { Stat } from "@/components/ui/card";
import { Section } from "@/components/ui/section";
import { Segmented } from "@/components/ui/segmented";
import { SwitchRow } from "@/components/ui/switch";
import { useT, type Key } from "@/i18n";

const errorText = "text-[13px] text-destructive-foreground";

/** The firmware switches; every page that shows some of them reads them once. */
export function useHardware() {
  const [hw, setHw] = useState<HardwareView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .getHardware()
      .then((v) => alive && setHw(v))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  const run = (p: Promise<HardwareView>) => {
    setError(null);
    p.then(setHw).catch((e) => setError(String(e)));
  };

  return { hw, error, run };
}

type Hardware = ReturnType<typeof useHardware>;

function SetupHint() {
  const t = useT();
  return (
    <div className="flex flex-col items-start gap-3">
      <p className="text-[13px] text-muted-foreground">{t("hwSetupNeeded")}</p>
      <Button variant="secondary" onClick={() => void api.runInstall()}>
        {t("setupAction")}
      </Button>
    </div>
  );
}

function ToggleRows({ hardware, toggles }: { hardware: Hardware; toggles: { id: Toggle; label: Key }[] }) {
  const t = useT();
  const { hw, run } = hardware;
  return (
    <>
      {toggles
        .filter((x) => hw?.toggles[x.id] !== undefined)
        .map((x) => (
          <SwitchRow
            key={x.id}
            label={t(x.label)}
            checked={hw!.toggles[x.id]!}
            onCheckedChange={(on) => run(api.setHwToggle(x.id, on))}
          />
        ))}
    </>
  );
}

const CHARGE_LIMITS = [60, 80, 100];

export function ChargeSection({ hardware }: { hardware: Hardware }) {
  const t = useT();
  const { hw, error, run } = hardware;
  if (!hw) return null;
  const b = hw.battery;
  return (
    <>
      <Section title={t("chargeTitle")} titleId="charge-title" description={t("chargeBody")}>
        {hw.supported ? (
          <Segmented
            aria-labelledby="charge-title"
            options={CHARGE_LIMITS.map((v) => ({ value: v, label: v === 100 ? t("chargeFull") : t("pctValue", { value: v }) }))}
            value={hw.chargeLimit}
            onChange={(limit) => run(api.setChargeLimit(limit))}
          />
        ) : (
          <SetupHint />
        )}
        {error && <p className={errorText}>{t("errorGeneric", { error })}</p>}
      </Section>
      {b && (
        <Section title={t("batteryHealth")} description={t("batteryHealthBody")}>
          <dl className="grid w-full grid-cols-3 gap-2">
            <Stat label={t("batteryHealthNow")} value={t("pctValue", { value: Math.round((b.full / b.design) * 100) })} />
            <Stat label={t("batteryCapacity")} value={t("whValue", { full: Math.round(b.full / 1000), design: Math.round(b.design / 1000) })} />
            <Stat label={t("batteryCycles")} value={b.cycles == null ? "–" : String(b.cycles)} />
          </dl>
        </Section>
      )}
    </>
  );
}

const LOCKS: { id: Toggle; label: Key }[] = [
  { id: "winLock", label: "toggleWinLock" },
  { id: "fnLock", label: "toggleFnLock" },
  { id: "touchpad", label: "toggleTouchpad" },
];

/** Card body for the keyboard page, which lays out cards rather than sections. */
export function LocksPanel({ hardware }: { hardware: Hardware }) {
  const t = useT();
  const { hw, error, run } = hardware;
  if (!hw) return null;
  if (!hw.supported) return <SetupHint />;
  return (
    <div className="flex flex-col gap-3">
      <ToggleRows hardware={hardware} toggles={LOCKS.slice(0, 1)} />
      <SwitchRow
        label={t("toggleWinLockGames")}
        checked={hw.winLockInGames}
        onCheckedChange={(on) => run(api.setWinLockInGames(on))}
      />
      <ToggleRows hardware={hardware} toggles={LOCKS.slice(1)} />
      {error && <p className={errorText}>{t("errorGeneric", { error })}</p>}
    </div>
  );
}

const GPU_MODES: { value: GpuMode; label: Key }[] = [
  { value: "hybrid", label: "gpuHybrid" },
  { value: "discrete", label: "gpuDiscrete" },
];

export function GpuModeSection() {
  const t = useT();
  const hardware = useHardware();
  const { hw, error, run } = hardware;
  const [confirm, setConfirm] = useState(false);
  if (!hw) return null;
  const pending = hw.gpuMode != null && hw.gpuActive != null && hw.gpuMode !== hw.gpuActive;
  return (
    <Section title={t("gpuModeTitle")} titleId="gpu-mode-title" description={t("gpuModeBody")}>
      {hw.supported ? (
        <Segmented
          aria-labelledby="gpu-mode-title"
          options={GPU_MODES.map((m) => ({ value: m.value, label: t(m.label) }))}
          value={hw.gpuMode}
          onChange={(mode) => run(api.setGpuMode(mode))}
        />
      ) : (
        <SetupHint />
      )}
      {pending && (
        <div className="flex flex-wrap items-center gap-3">
          <p className="text-[13px] text-muted-foreground">{t("gpuRestartNeeded")}</p>
          <Button variant={confirm ? "default" : "secondary"} onClick={() => (confirm ? void api.restartNow() : setConfirm(true))}>
            {confirm ? t("gpuRestartConfirm") : t("gpuRestart")}
          </Button>
        </div>
      )}
      {error && <p className={errorText}>{t("errorGeneric", { error })}</p>}
    </Section>
  );
}

const LIGHTS: { id: Toggle; label: Key }[] = [
  { id: "powerLed", label: "togglePowerLed" },
  { id: "batteryLed", label: "toggleBatteryLed" },
  { id: "bootAnimation", label: "toggleBootAnimation" },
];

export function LightsSection() {
  const t = useT();
  const hardware = useHardware();
  const { hw, error } = hardware;
  if (!hw) return null;
  return (
    <Section title={t("lightsTitle")}>
      {hw.supported ? <ToggleRows hardware={hardware} toggles={LIGHTS} /> : <SetupHint />}
      {error && <p className={errorText}>{t("errorGeneric", { error })}</p>}
    </Section>
  );
}
