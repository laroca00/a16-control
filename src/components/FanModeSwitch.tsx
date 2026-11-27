import type { FanMode, Telemetry } from "@/api";
import { useT, type Key } from "@/i18n";
import { Segmented } from "@/components/ui/segmented";

const MODES: { id: FanMode; label: Key }[] = [
  { id: "quiet", label: "fanModeQuiet" },
  { id: "auto", label: "fanModeAuto" },
  { id: "curve", label: "fanModeCurve" },
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
    <Segmented
      aria-label={t("fanModeLabel")}
      fill
      size="lg"
      className="w-full"
      options={MODES.map((m) => ({ value: m.id, label: t(m.label) }))}
      value={current}
      disabled={!telemetry || busy}
      onChange={(m) => current !== m && onChange(m)}
    />
  );
}
