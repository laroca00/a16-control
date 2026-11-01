import type { PowerView, Reason, UiMode } from "../api";
import { useT, type Key } from "../i18n";
import "./ModeSwitch.css";

const MODES: { id: UiMode; label: Key }[] = [
  { id: "battery", label: "modeBattery" },
  { id: "auto", label: "modeAuto" },
  { id: "performance", label: "modePerformance" },
];

const REASONS: Record<Reason, Key> = {
  userBattery: "reasonUserBattery",
  userPerformance: "reasonUserPerformance",
  autoAc: "reasonAutoAc",
  autoDc: "reasonAutoDc",
  autoLowBattery: "reasonAutoLowBattery",
  autoGame: "reasonAutoGame",
};

type Props = {
  power: PowerView | null;
  onChange: (mode: UiMode) => void;
  size?: "compact" | "large";
};

export function ModeSwitch({ power, onChange, size = "compact" }: Props) {
  const t = useT();
  const current = power?.mode ?? null;
  return (
    <div className={`mode-switch mode-switch--${size}`}>
      <div className="mode-switch__track" role="radiogroup" aria-label={t("modeLabel")}>
        {MODES.map((m) => (
          <button
            key={m.id}
            type="button"
            role="radio"
            aria-checked={current === m.id}
            disabled={!power}
            className={`mode-switch__item mode-switch__item--${m.id}${current === m.id ? " mode-switch__item--on" : ""}`}
            onClick={() => onChange(m.id)}
          >
            {t(m.label)}
          </button>
        ))}
      </div>
      {power?.reason && <p className="mode-switch__reason">{t(REASONS[power.reason])}</p>}
    </div>
  );
}
