import { cn } from "@/lib/utils";

type Props = {
  value: number | null;
  max: number;
  label: string;
  unit: string;
  tone: "fan" | "heat";
};

// A 180° arc; long enough to read at a glance, short enough to sit in a row.
const R = 40;
const LENGTH = Math.PI * R;
const ARC = `M10,50 A${R},${R} 0 0 1 90,50`;

export function Gauge({ value, max, label, unit, tone }: Props) {
  const filled = value === null ? 0 : LENGTH * Math.min(Math.max(value, 0) / max, 1);
  return (
    <figure className="m-0 grid justify-items-center">
      <svg viewBox="0 0 100 56" aria-hidden="true" className="w-full max-w-[150px] fill-none [stroke-linecap:round] [stroke-width:6]">
        <path d={ARC} className="stroke-muted" />
        <path
          d={ARC}
          strokeDasharray={`${filled} 1000`}
          className={cn("transition-[stroke-dasharray] duration-700", tone === "fan" ? "stroke-chart-1" : "stroke-chart-2")}
        />
      </svg>
      <figcaption className="-mt-5 grid justify-items-center gap-0.5">
        <span className="font-mono text-xl font-medium tabular-nums">
          {value ?? "–"}
          <small className="ml-0.5 font-sans text-xs font-normal text-muted-foreground">{unit}</small>
        </span>
        <span className="text-xs text-muted-foreground">{label}</span>
      </figcaption>
    </figure>
  );
}
