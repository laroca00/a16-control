import { useEffect, useState } from "react";
import type { Telemetry } from "../api";
import { MaxFanButton } from "../components/MaxFanButton";
import { Sparkline } from "../components/Sparkline";
import type { useFanControl } from "../hooks/useFanControl";
import { useT } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

type Control = ReturnType<typeof useFanControl>;

// 120 samples at the 1.5 s tick is three minutes.
const HISTORY = 120;

function useHistory(telemetry: Telemetry | null): Telemetry[] {
  const [samples, setSamples] = useState<Telemetry[]>([]);
  useEffect(() => {
    if (telemetry) setSamples((s) => [...s.slice(-(HISTORY - 1)), telemetry]);
  }, [telemetry]);
  return samples;
}

export function FanPage({ control }: { control: Control }) {
  const t = useT();
  const { telemetry, busy, toggle } = control;
  const history = useHistory(telemetry);

  return (
    <div className="page page--fan">
      <h1 className="page__title">{t("navFan")}</h1>
      <div className="fan-page">
        <LiquidGlass className="fan-page__control" radius={28}>
          <MaxFanButton telemetry={telemetry} busy={busy} onToggle={toggle} />
        </LiquidGlass>
        <LiquidGlass className="fan-page__charts" radius={24}>
          <p className="card__hint">{t("fanHistory")}</p>
          <h2 className="card__title">{t("fanSpeedChart")}</h2>
          <div className="chart">
            <Sparkline values={history.map((s) => s.rpm1)} max={6400} label={t("fan1")} />
            <Sparkline values={history.map((s) => s.rpm2)} max={6400} label={t("fan2")} tone="gilt" />
          </div>
          <h2 className="card__title">{t("tempChart")}</h2>
          <div className="chart">
            <Sparkline values={history.map((s) => s.cpuTemp)} max={100} label={t("cpu")} tone="bole" />
            <Sparkline values={history.map((s) => s.gpuTemp)} max={100} label={t("gpu")} tone="gilt" />
          </div>
        </LiquidGlass>
      </div>
    </div>
  );
}
