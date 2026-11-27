import { useEffect } from "react";
import { api } from "@/api";
import { MaxFanButton } from "@/components/MaxFanButton";
import { ModeSwitch } from "@/components/ModeSwitch";
import { KeyboardLightNotice, SetupNotice, UnsupportedNotice } from "@/components/Notices";
import { Button } from "@/components/ui/button";
import { usePower } from "@/hooks/usePower";
import { useFanControl } from "@/hooks/useFanControl";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";

export function QuickPanel() {
  const t = useT();
  const { telemetry, status, needsSetup, unsupported, unreachable, busy, error, toggle } = useFanControl();
  const { power, setMode } = usePower();

  useEffect(() => {
    document.body.classList.add("panel-window");
    return () => document.body.classList.remove("panel-window");
  }, []);

  // With the KeyboardLight warning there is less room, so the dial shrinks and drops its figures.
  const crowded = Boolean(status?.keyboardLight);

  return (
    <div className="h-full p-2 animate-in fade-in slide-in-from-bottom-2 duration-300">
      <div
        className={cn(
          "flex h-full flex-col items-center rounded-xl border bg-card px-4 pb-3",
          crowded ? "gap-3 pt-4" : "gap-4 pt-5",
          crowded ? "quick-panel quick-panel--crowded" : "quick-panel",
        )}
      >
        {needsSetup ? (
          <div className="mt-auto w-full">
            <SetupNotice />
          </div>
        ) : unsupported ? (
          <UnsupportedNotice />
        ) : (
          <>
            <MaxFanButton telemetry={telemetry} busy={busy} onToggle={toggle} />
            <dl className="grid w-full grid-cols-2 divide-x rounded-lg bg-muted">
              {(["cpu", "gpu"] as const).map((k) => (
                <div key={k} className={cn("grid justify-items-center", crowded ? "py-1" : "py-2")}>
                  <dt className="text-xs text-muted-foreground">{t(k)}</dt>
                  <dd className="font-mono text-lg font-medium tabular-nums">
                    {telemetry ? t("tempValue", { value: k === "cpu" ? telemetry.cpuTemp : telemetry.gpuTemp }) : "–"}
                  </dd>
                </div>
              ))}
            </dl>
            <ModeSwitch power={power} onChange={setMode} />
            {error && <p className="text-center text-xs text-destructive-foreground">{t("errorGeneric", { error })}</p>}
            {!error && unreachable && (
              <p className="text-center text-xs text-destructive-foreground">{t("unreachable", { error: unreachable })}</p>
            )}
          </>
        )}
        {status?.keyboardLight && <KeyboardLightNotice />}
        <Button variant="ghost" className="mt-auto" onClick={() => api.openMain()}>
          {t("allSettings")}
        </Button>
      </div>
    </div>
  );
}
