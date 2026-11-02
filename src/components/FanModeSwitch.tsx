import type { FanMode, Telemetry } from "../api";
import { useT, type Key } from "../i18n";
import "./ModeSwitch.css";

const MODES: { id: FanMode; label: Key }[] = [
  { id: "quiet", label: "fanModeQuiet" },
  { id: "auto", label: "fanModeAuto" },
  { id: "max", label: "fanModeMax" },
];

type Props = {
  telemetry: Telemetry | null;
  busy: boolean;
  onChange: (mode: FanMode) => void;
};

export function FanModeSwitch({ telemetry, busy, onChange }: Props) {
  const t = useT();
  const current = telemetry?.fanMode ?? null;
  return (
    <div className="mode-switch mode-switch--large mode-switch--fan">
      <div className="mode-switch__track" role="radiogroup" aria-label={t("fanModeLabel")}>
        {MODES.map((m) => (
          <button
            key={m.id}
            type="button"
            role="radio"
            aria-checked={current === m.id}
            disabled={!telemetry || busy}
            className={`mode-switch__item mode-switch__item--fan-${m.id}${current === m.id ? " mode-switch__item--on" : ""}`}
            onClick={() => current !== m.id && onChange(m.id)}
          >
            {t(m.label)}
          </button>
        ))}
      </div>
    </div>
  );
}
