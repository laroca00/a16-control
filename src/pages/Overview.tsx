import { Gauge } from "@/components/Gauge";
import { MaxFanButton } from "@/components/MaxFanButton";
import { Card, CardTitle } from "@/components/ui/card";
import type { useFanControl } from "@/hooks/useFanControl";
import { useT } from "@/i18n";

type Control = ReturnType<typeof useFanControl>;

export function Overview({ control }: { control: Control }) {
  const t = useT();
  const { telemetry, busy, error, unreachable, toggle } = control;
  return (
    <div className="grid min-h-[420px] flex-1 grid-cols-2 grid-rows-2 gap-4">
      <Card className="dial-lg row-span-2 items-center justify-center gap-6 p-6">
        <MaxFanButton telemetry={telemetry} busy={busy} onToggle={toggle} />
        <p className="max-w-[34ch] text-center text-[13px] text-muted-foreground">
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
      </Card>
      <Card className="grid grid-rows-[auto_1fr] items-center">
        <CardTitle>{t("overviewFans")}</CardTitle>
        <div className="grid grid-cols-2 gap-3">
          <Gauge value={telemetry?.rpm1 ?? null} max={6400} label={t("fan1")} unit=" RPM" tone="fan" />
          <Gauge value={telemetry?.rpm2 ?? null} max={6400} label={t("fan2")} unit=" RPM" tone="fan" />
        </div>
      </Card>
      <Card className="grid grid-rows-[auto_1fr] items-center">
        <CardTitle>{t("overviewTemps")}</CardTitle>
        <div className="grid grid-cols-2 gap-3">
          <Gauge value={telemetry?.cpuTemp ?? null} max={100} label={t("cpu")} unit="°C" tone="heat" />
          <Gauge value={telemetry?.gpuTemp ?? null} max={100} label={t("gpu")} unit="°C" tone="heat" />
        </div>
      </Card>
    </div>
  );
}
