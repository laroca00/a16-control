import { useEffect } from "react";
import { api } from "../api";
import { MaxFanButton } from "../components/MaxFanButton";
import { ModeSwitch } from "../components/ModeSwitch";
import { usePower } from "../hooks/usePower";
import { KeyboardLightNotice, SetupNotice, UnsupportedNotice } from "../components/Notices";
import { useFanControl } from "../hooks/useFanControl";
import { useT } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";
import "./panel.css";

export function QuickPanel() {
  const t = useT();
  const { telemetry, status, needsSetup, unsupported, unreachable, busy, error, toggle } = useFanControl();
  const { power, setMode } = usePower();

  useEffect(() => {
    document.body.classList.add("panel-window");
    return () => document.body.classList.remove("panel-window");
  }, []);

  const crowded = Boolean(status?.keyboardLight);

  return (
    <div className={`panel${crowded ? " panel--crowded" : ""}${needsSetup ? " panel--setup" : ""}`}>
      <div className="panel__backdrop backdrop" />
      <LiquidGlass className="panel__card" radius={22}>
        {needsSetup ? (
          <SetupNotice />
        ) : unsupported ? (
          <UnsupportedNotice />
        ) : (
          <>
            <MaxFanButton telemetry={telemetry} busy={busy} onToggle={toggle} />
            <dl className="panel__temps">
              <div>
                <dt>{t("cpu")}</dt>
                <dd>{telemetry ? t("tempValue", { value: telemetry.cpuTemp }) : "–"}</dd>
              </div>
              <div>
                <dt>{t("gpu")}</dt>
                <dd>{telemetry ? t("tempValue", { value: telemetry.gpuTemp }) : "–"}</dd>
              </div>
            </dl>
            <ModeSwitch power={power} onChange={setMode} />
            {error && <p className="panel__error">{t("errorGeneric", { error })}</p>}
            {!error && unreachable && <p className="panel__error">{t("unreachable", { error: unreachable })}</p>}
          </>
        )}
        {status?.keyboardLight && <KeyboardLightNotice />}
        <button type="button" className="panel__more" onClick={() => api.openMain()}>
          {t("allSettings")}
        </button>
      </LiquidGlass>
    </div>
  );
}
