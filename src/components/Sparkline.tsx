type Props = {
  values: number[];
  max: number;
  label: string;
  tone?: "turquoise" | "bole" | "gilt";
};

const W = 100;
const H = 30;

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

export function Sparkline({ values, max, label, tone = "turquoise" }: Props) {
  return (
    <svg className={`sparkline sparkline--${tone}`} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={label}>
      <path d={sparkPath(values, max)} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
