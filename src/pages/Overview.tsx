import { Gauge } from "../components/Gauge";
import { MaxFanButton } from "../components/MaxFanButton";
import type { useFanControl } from "../hooks/useFanControl";
import { useT } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

type Control = ReturnType<typeof useFanControl>;

export function Overview({ control }: { control: Control }) {
  const t = useT();
  const { telemetry, busy, error, unreachable, toggle } = control;
  return (
    <div className="page page--overview">
      <h1 className="page__title">{t("navOverview")}</h1>
      <div className="overview">
        <LiquidGlass className="overview__hero" radius={28}>
          <MaxFanButton telemetry={telemetry} busy={busy} onToggle={toggle} />
          <p className="overview__status">
            {error
              ? t("errorGeneric", { error })
              : unreachable
                ? t("unreachable", { error: unreachable })
                : telemetry?.fanMode === "max"
                  ? t("statusMax")
                  : telemetry?.fanMode === "quiet"
                    ? t("statusQuiet")
                    : t("statusAuto")}
          </p>
        </LiquidGlass>
        <LiquidGlass className="overview__card" radius={24}>
          <h2 className="card__title">{t("overviewFans")}</h2>
          <div className="gauges">
            <Gauge value={telemetry?.rpm1 ?? null} max={6400} label={t("fan1")} unit=" RPM" tone="turquoise" />
            <Gauge value={telemetry?.rpm2 ?? null} max={6400} label={t("fan2")} unit=" RPM" tone="turquoise" />
          </div>
        </LiquidGlass>
        <LiquidGlass className="overview__card" radius={24}>
          <h2 className="card__title">{t("overviewTemps")}</h2>
          <div className="gauges">
            <Gauge value={telemetry?.cpuTemp ?? null} max={100} label={t("cpu")} unit="°C" tone="bole" />
            <Gauge value={telemetry?.gpuTemp ?? null} max={100} label={t("gpu")} unit="°C" tone="bole" />
          </div>
        </LiquidGlass>
      </div>
    </div>
  );
}
