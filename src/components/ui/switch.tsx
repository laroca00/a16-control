import { cn } from "@/lib/utils";

type SwitchProps = {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onCheckedChange: (checked: boolean) => void;
};

export function Switch({ checked, disabled, label, onCheckedChange }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
      className={cn(
        "relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors disabled:cursor-default disabled:opacity-50",
        checked ? "bg-primary" : "bg-input",
      )}
    >
      <span
        className={cn(
          "pointer-events-none block size-4 rounded-full transition-[transform,background-color]",
          checked ? "translate-x-[18px] bg-primary-foreground" : "translate-x-0.5 bg-foreground",
        )}
      />
    </button>
  );
}

/** A labelled row with a switch on the right; rows divide with a hairline. */
export function SwitchRow(props: SwitchProps) {
  return (
    <div className="flex w-full items-center justify-between gap-4 text-[13px] [&+&]:border-t [&+&]:pt-3">
      <span>{props.label}</span>
      <Switch {...props} />
    </div>
  );
}
