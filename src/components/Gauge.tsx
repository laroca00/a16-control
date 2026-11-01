type Props = {
  value: number | null;
  max: number;
  label: string;
  unit: string;
  tone: "turquoise" | "bole";
};

// A 180° arc; long enough to read at a glance, short enough to sit in a row.
const R = 40;
const LENGTH = Math.PI * R;

export function Gauge({ value, max, label, unit, tone }: Props) {
  const filled = value === null ? 0 : LENGTH * Math.min(Math.max(value, 0) / max, 1);
  return (
    <figure className={`gauge gauge--${tone}`}>
      <svg viewBox="0 0 100 56" aria-hidden="true">
        <path className="gauge__track" d={`M10,50 A${R},${R} 0 0 1 90,50`} />
        <path className="gauge__level" d={`M10,50 A${R},${R} 0 0 1 90,50`} strokeDasharray={`${filled} 1000`} />
      </svg>
      <figcaption>
        <span className="gauge__value">
          {value ?? "–"}
          <small>{unit}</small>
        </span>
        <span className="gauge__label">{label}</span>
      </figcaption>
    </figure>
  );
}
