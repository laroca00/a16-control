import type { FanMode, Telemetry } from "../api";
import { useT } from "../i18n";
import "./MaxFanButton.css";

// Both fans top out at about 6380 RPM on the A16.
const RPM_CEILING = 6400;
// The rings are 270° gauges open at the bottom.
const SWEEP = 0.75;

type Props = {
  telemetry: Telemetry | null;
  busy: boolean;
  onToggle: (next: FanMode) => void;
};

function Ring({ r, rpm }: { r: number; rpm: number }) {
  const length = 2 * Math.PI * r * SWEEP;
  const filled = length * Math.min(rpm / RPM_CEILING, 1);
  return (
    <>
      <circle className="fan-dial__track" cx="100" cy="100" r={r} strokeDasharray={`${length} 1000`} />
      <circle className="fan-dial__level" cx="100" cy="100" r={r} strokeDasharray={`${filled} 1000`} />
    </>
  );
}

export function MaxFanButton({ telemetry, busy, onToggle }: Props) {
  const t = useT();
  const on = telemetry?.fanMode === "max";
  const state = busy ? t("fanWorking") : on ? t("fanOn") : t("fanOff");

  return (
    <button
      type="button"
      className={`fan-dial${on ? " fan-dial--on" : ""}${busy ? " fan-dial--busy" : ""}`}
      aria-pressed={on}
      title={t("fanToggleHint")}
      disabled={!telemetry || busy}
      onClick={() => onToggle(on ? "auto" : "max")}
    >
      <svg className="fan-dial__rings" viewBox="0 0 200 200" aria-hidden="true">
        <g transform="rotate(135 100 100)">
          <Ring r={86} rpm={telemetry?.rpm1 ?? 0} />
          <Ring r={70} rpm={telemetry?.rpm2 ?? 0} />
        </g>
      </svg>
      <span className="fan-dial__face">
        <span className="fan-dial__name">{t("fanMax")}</span>
        <span className="fan-dial__state">{state}</span>
        {telemetry && (
          <span className="fan-dial__rpm">{t("fanRpm", { rpm1: telemetry.rpm1, rpm2: telemetry.rpm2 })}</span>
        )}
      </span>
    </button>
  );
}
