import { cn } from "@/lib/utils";

type SliderProps = {
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
  /** The figure shown at the end of the row. */
  output: string;
  disabled?: boolean;
  onChange: (value: number) => void;
  /** Called where a drag or key press ends, for writes that should not repeat. */
  onCommit?: () => void;
  /** Keeps the label for screen readers only, when a heading already names the slider. */
  hideLabel?: boolean;
};

export function Slider({ label, min, max, step, value, output, disabled, onChange, onCommit, hideLabel }: SliderProps) {
  return (
    <label
      className={cn(
        "grid w-full items-center gap-3 text-[13px]",
        hideLabel ? "grid-cols-[1fr_4ch]" : "grid-cols-[auto_1fr_4ch]",
      )}
    >
      <span className={cn(hideLabel && "sr-only")}>{label}</span>
      <input
        type="range"
        className="w-full cursor-pointer accent-primary disabled:cursor-default"
        min={min}
        max={max}
        step={step}
        aria-label={label}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        onPointerUp={onCommit}
        onKeyUp={onCommit}
        onBlur={onCommit}
      />
      <output className="text-right font-mono text-xs text-muted-foreground tabular-nums">{output}</output>
    </label>
  );
}
