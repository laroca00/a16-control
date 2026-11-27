import type { PowerView, Reason, UiMode } from "@/api";
import { useT, type Key } from "@/i18n";
import { cn } from "@/lib/utils";
import { Segmented } from "@/components/ui/segmented";

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
  const large = size === "large";
  return (
    <div className="flex w-full flex-col gap-2">
      <Segmented
        aria-label={t("modeLabel")}
        fill
        size={large ? "lg" : "default"}
        options={MODES.map((m) => ({ value: m.id, label: t(m.label) }))}
        value={power?.mode ?? null}
        disabled={!power}
        onChange={onChange}
      />
      {power?.reason && (
        <p className={cn("text-xs text-muted-foreground", large ? "text-[13px]" : "text-center")}>
          {t(REASONS[power.reason])}
        </p>
      )}
    </div>
  );
}
