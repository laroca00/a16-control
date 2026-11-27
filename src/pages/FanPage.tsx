import { useEffect, useState, type ReactNode } from "react";
import type { Telemetry } from "@/api";
import { FanModeSwitch } from "@/components/FanModeSwitch";
import { MaxFanButton } from "@/components/MaxFanButton";
import { Sparkline } from "@/components/Sparkline";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import type { useFanControl } from "@/hooks/useFanControl";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";

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

const LEGEND = { fan: "bg-chart-1", heat: "bg-chart-2", muted: "bg-chart-muted" } as const;

function ChartGroup({
  title,
  series,
  children,
}: {
  title: string;
  series: { label: string; tone: keyof typeof LEGEND }[];
  children: ReactNode;
}) {
  return (
    <section className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex items-baseline justify-between gap-4">
        <CardTitle>{title}</CardTitle>
        <ul className="flex gap-4 text-xs text-muted-foreground">
          {series.map((s) => (
            <li key={s.label} className="flex items-center gap-2">
              <span className={cn("h-0.5 w-3 rounded-full", LEGEND[s.tone])} />
              {s.label}
            </li>
          ))}
        </ul>
      </div>
      <div className="relative min-h-24 flex-1 rounded-lg border bg-background">{children}</div>
    </section>
  );
}

export function FanPage({ control }: { control: Control }) {
  const t = useT();
  const { telemetry, busy, toggle } = control;
  const history = useHistory(telemetry);

  return (
    <div className="grid min-h-[420px] flex-1 grid-cols-[auto_minmax(0,1fr)] gap-4">
      <Card className="items-center justify-center gap-6 p-6">
        <MaxFanButton telemetry={telemetry} busy={busy} onToggle={toggle} />
        <FanModeSwitch telemetry={telemetry} busy={busy} onChange={toggle} />
      </Card>
      <Card className="gap-6">
        <ChartGroup
          title={t("fanSpeedChart")}
          series={[
            { label: t("fan1"), tone: "fan" },
            { label: t("fan2"), tone: "muted" },
          ]}
        >
          <Sparkline values={history.map((s) => s.rpm1)} max={6400} label={t("fan1")} tone="fan" />
          <Sparkline values={history.map((s) => s.rpm2)} max={6400} label={t("fan2")} tone="muted" />
        </ChartGroup>
        <ChartGroup
          title={t("tempChart")}
          series={[
            { label: t("cpu"), tone: "heat" },
            { label: t("gpu"), tone: "muted" },
          ]}
        >
          <Sparkline values={history.map((s) => s.cpuTemp)} max={100} label={t("cpu")} tone="heat" />
          <Sparkline values={history.map((s) => s.gpuTemp)} max={100} label={t("gpu")} tone="muted" />
        </ChartGroup>
        <CardDescription className="text-xs">{t("fanHistory")}</CardDescription>
      </Card>
    </div>
  );
}
