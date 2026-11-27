import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** A settings page: sections stacked and divided by hairlines. */
export function Sections({ children }: { children: ReactNode }) {
  return <div className="flex w-full max-w-4xl flex-col">{children}</div>;
}

/** One settings section: what it is on the left, its controls on the right. */
export function Section({
  title,
  description,
  titleId,
  className,
  children,
}: {
  title: string;
  description?: ReactNode;
  titleId?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section
      className={cn(
        "grid grid-cols-[minmax(160px,224px)_minmax(0,1fr)] gap-x-8 border-b py-6 first:pt-0 last:border-b-0",
        className,
      )}
    >
      <div className="flex flex-col gap-1">
        <h2 id={titleId} className="text-sm leading-5 font-medium">
          {title}
        </h2>
        {description && <p className="text-[13px] leading-5 text-muted-foreground">{description}</p>}
      </div>
      <div className="flex min-w-0 flex-col items-start gap-3">{children}</div>
    </section>
  );
}
