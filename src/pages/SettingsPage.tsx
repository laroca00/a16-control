import { useEffect, useState } from "react";
import { Check, Minus, X } from "lucide-react";
import { api, type AlertOptions, type SetupStatus } from "@/api";
import { LightsSection } from "@/components/HardwareSections";
import { Button } from "@/components/ui/button";
import { Section, Sections } from "@/components/ui/section";
import { Segmented } from "@/components/ui/segmented";
import { Switch, SwitchRow } from "@/components/ui/switch";
import { autoCheckEnabled, setAutoCheck, useUpdate } from "@/lib/update";
import { useLang, useT, type Key, type Lang } from "@/i18n";
import { cn } from "@/lib/utils";

const CHECKS: { key: Key; ok: (s: SetupStatus) => boolean }[] = [
  { key: "checkElevated", ok: (s) => s.elevated },
  { key: "checkTask", ok: (s) => s.task },
  { key: "checkWmiSet", ok: (s) => s.wmiSet },
  { key: "checkWmiGet", ok: (s) => s.wmiGet },
  { key: "checkWmiFeatures", ok: (s) => s.wmiFeatures },
  { key: "checkKeyboardLight", ok: (s) => !s.keyboardLight },
];

const LANGS: { value: Lang; label: string }[] = [
  { value: "tr", label: "Türkçe" },
  { value: "en", label: "English" },
];

const LIMITS = [80, 85, 90, 95];

const SHORTCUTS: { keys: string[]; label: Key }[] = [
  { keys: ["Ctrl", "Alt", "F"], label: "shortcutFan" },
  { keys: ["Ctrl", "Alt", "P"], label: "shortcutPower" },
  { keys: ["Ctrl", "↑ ↓"], label: "shortcutKb" },
];

function Keys({ keys }: { keys: string[] }) {
  return (
    <span className="flex gap-1">
      {keys.map((k) => (
        <kbd
          key={k}
          className="inline-flex h-6 min-w-6 items-center justify-center rounded-md border bg-muted px-1.5 font-mono text-[11px] text-muted-foreground"
        >
          {k}
        </kbd>
      ))}
    </span>
  );
}

function AlertSection() {
  const t = useT();
  const [alerts, setAlerts] = useState<AlertOptions | null>(null);

  useEffect(() => {
    api.getAlerts().then(setAlerts).catch(() => {});
  }, []);

  const change = (patch: Partial<AlertOptions>) => {
    if (!alerts) return;
    api.setAlerts({ ...alerts, ...patch }).then(setAlerts).catch(() => {});
  };

  return (
    <Section title={t("alertTitle")} description={t("alertBody")}>
      <SwitchRow label={t("alertEnable")} checked={alerts?.enabled ?? false} disabled={!alerts} onCheckedChange={(v) => change({ enabled: v })} />
      {alerts?.enabled && (
        <>
          <div className="flex w-full items-center justify-between gap-4 border-t pt-3 text-[13px]">
            <span id="alert-limit">{t("alertLimit")}</span>
            <Segmented
              aria-labelledby="alert-limit"
              options={LIMITS.map((v) => ({ value: v, label: `${v}°C` }))}
              value={alerts.temp}
              onChange={(v) => change({ temp: v })}
            />
          </div>
          <div className="flex w-full items-center justify-between gap-4 border-t pt-3 text-[13px]">
            <span>{t("alertMaxFan")}</span>
            <Switch label={t("alertMaxFan")} checked={alerts.maxFan} onCheckedChange={(v) => change({ maxFan: v })} />
          </div>
        </>
      )}
    </Section>
  );
}

function UpdateSection() {
  const t = useT();
  const { current, state, check } = useUpdate({ auto: false });
  const [auto, setAuto] = useState(autoCheckEnabled);

  return (
    <Section title={t("updateTitle")} description={current ? t("updateVersion", { version: current }) : undefined}>
      <SwitchRow
        label={t("updateAuto")}
        checked={auto}
        onCheckedChange={(v) => {
          setAutoCheck(v);
          setAuto(v);
        }}
      />
      <div className="flex w-full items-center gap-3 border-t pt-3">
        <Button variant="secondary" disabled={!current || state.status === "checking"} onClick={() => void check()}>
          {state.status === "checking" ? t("updateChecking") : t("updateCheck")}
        </Button>
        {state.status === "latest" && <span className="text-[13px] text-muted-foreground">{t("updateLatest")}</span>}
        {state.status === "error" && (
          <span className="text-[13px] text-destructive-foreground">{t("updateFailed", { error: state.error })}</span>
        )}
        {state.status === "available" && (
          <>
            <span className="text-[13px]">{t("updateAvailable", { version: state.release.version })}</span>
            <Button onClick={() => void api.openReleasePage(state.release.url)}>{t("updateDownload")}</Button>
          </>
        )}
      </div>
    </Section>
  );
}

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
    <Sections>
      <Section title={t("language")} titleId="lang-title">
        <Segmented aria-labelledby="lang-title" options={LANGS} value={lang} onChange={setLang} />
      </Section>

      <AlertSection />

      <LightsSection />

      <Section title={t("shortcutsTitle")} description={t("shortcutsBody")}>
        <ul className="flex w-full flex-col gap-2">
          {SHORTCUTS.map((s) => (
            <li key={s.label} className="flex items-center justify-between gap-4 text-[13px]">
              {t(s.label)}
              <Keys keys={s.keys} />
            </li>
          ))}
        </ul>
      </Section>

      <UpdateSection />

      <Section title={t("setupChecks")}>
        <ul className="flex flex-col gap-2">
          {CHECKS.map((c) => {
            const ok = status ? c.ok(status) : null;
            const Icon = ok === null ? Minus : ok ? Check : X;
            return (
              <li key={c.key} className="flex items-center gap-2.5 text-[13px]">
                <span
                  aria-hidden="true"
                  className={cn(
                    "grid size-5 shrink-0 place-items-center rounded-full",
                    ok === null ? "bg-muted text-muted-foreground" : ok ? "bg-chart-1/15 text-chart-1" : "bg-destructive/15 text-destructive-foreground",
                  )}
                >
                  <Icon className="size-3" strokeWidth={2} />
                </span>
                {t(c.key)}
              </li>
            );
          })}
        </ul>
      </Section>

      <Section title={t("uninstallTitle")} description={t("uninstallBody")}>
        {error && <p className="text-[13px] text-destructive-foreground">{t("errorGeneric", { error })}</p>}
        <Button variant="destructive" onClick={uninstall}>
          {t("uninstallAction")}
        </Button>
      </Section>
    </Sections>
  );
}
