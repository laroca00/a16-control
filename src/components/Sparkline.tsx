import { cn } from "@/lib/utils";

type Props = {
  values: number[];
  max: number;
  label: string;
  /** fan and heat lead their chart; muted is the quieter second series. */
  tone?: "fan" | "heat" | "muted";
};

const W = 100;
const H = 30;

const STROKE = { fan: "stroke-chart-1", heat: "stroke-chart-2", muted: "stroke-chart-muted" } as const;

export function sparkPath(values: number[], max: number): string {
  if (values.length === 0) return "";
  const step = values.length > 1 ? W / (values.length - 1) : 0;
  return values
    .map((v, i) => {
      const y = H - (Math.min(Math.max(v, 0), max) / max) * H;
      return `${i === 0 ? "M" : "L"}${(i * step).toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

export function Sparkline({ values, max, label, tone = "fan" }: Props) {
  return (
    <svg
      className="absolute inset-2 h-[calc(100%-16px)] w-[calc(100%-16px)]"
      viewBox={`0 0 ${W} ${H}`}
      preserveAspectRatio="none"
      role="img"
      aria-label={label}
    >
      <path
        d={sparkPath(values, max)}
        vectorEffect="non-scaling-stroke"
        className={cn("fill-none [stroke-linejoin:round] [stroke-width:1.5]", STROKE[tone])}
      />
    </svg>
  );
}
