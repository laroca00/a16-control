import { cn } from "@/lib/utils";

type Option<T extends string | number> = { value: T; label: string };

type SegmentedProps<T extends string | number> = {
  options: Option<T>[];
  value: T | null;
  onChange: (value: T) => void;
  disabled?: boolean;
  /** Stretch the options across the full width. */
  fill?: boolean;
  size?: "default" | "lg";
  className?: string;
} & ({ "aria-label": string } | { "aria-labelledby": string });

/** A radio group drawn as one control; the chosen option gets the gilt edge. */
export function Segmented<T extends string | number>({
  options,
  value,
  onChange,
  disabled,
  fill,
  size = "default",
  className,
  ...aria
}: SegmentedProps<T>) {
  return (
    <div
      role="radiogroup"
      {...aria}
      className={cn("gap-1 rounded-lg bg-muted p-1", fill ? "grid grid-flow-col auto-cols-fr" : "inline-flex self-start", className)}
    >
      {options.map((o) => {
        const on = value === o.value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={on}
            disabled={disabled}
            onClick={() => onChange(o.value)}
            className={cn(
              "cursor-pointer rounded-md px-3 text-[13px] whitespace-nowrap transition-colors disabled:cursor-default",
              size === "lg" ? "h-9" : "h-7",
              on
                ? "bg-accent font-medium text-foreground ring-1 ring-primary/50 ring-inset"
                : "text-muted-foreground enabled:hover:text-foreground",
            )}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
