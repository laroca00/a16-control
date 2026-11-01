import { useEffect } from "react";
import { api } from "../api";
import { MaxFanButton } from "../components/MaxFanButton";
import { KeyboardLightNotice, SetupNotice } from "../components/Notices";
import { useFanControl } from "../hooks/useFanControl";
import { useT } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";
import { ShaderGradient } from "../ui/ShaderGradient";
import "./panel.css";

export function QuickPanel() {
  const t = useT();
  const { telemetry, status, needsSetup, busy, error, toggle } = useFanControl();

  useEffect(() => {
    document.body.classList.add("panel-window");
    return () => document.body.classList.remove("panel-window");
  }, []);

  const crowded = Boolean(status?.keyboardLight);

  return (
    <div className={`panel${crowded ? " panel--crowded" : ""}${needsSetup ? " panel--setup" : ""}`}>
      <ShaderGradient className="panel__backdrop" colors={["#0b1846", "#2f55c8", "#1c9c92"]} brightness={0.85} />
      <LiquidGlass className="panel__card" radius={22}>
        {needsSetup ? (
          <SetupNotice />
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
            {error && <p className="panel__error">{t("errorGeneric", { error })}</p>}
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
