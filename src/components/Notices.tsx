import { useState, type ReactNode } from "react";
import { TriangleAlert } from "lucide-react";
import { api } from "@/api";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

function Notice({
  title,
  warn,
  children,
  live,
}: {
  title: string;
  warn?: boolean;
  children: ReactNode;
  live?: boolean;
}) {
  return (
    <section
      aria-live={live ? "polite" : undefined}
      className={cn("flex w-full gap-3 rounded-lg border bg-card p-4", warn && "border-primary/40")}
    >
      {warn && <TriangleAlert aria-hidden="true" strokeWidth={1.5} className="mt-0.5 size-4 shrink-0 text-primary" />}
      <div className="flex min-w-0 flex-col items-start gap-2">
        <h2 className="text-[13px] leading-5 font-medium">{title}</h2>
        {children}
      </div>
    </section>
  );
}

const body = "text-[13px] leading-5 text-muted-foreground";

export function SetupNotice() {
  const t = useT();
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function install() {
    setWorking(true);
    setError(null);
    try {
      // On success the app restarts elevated, so nothing more happens here.
      await api.runInstall();
    } catch (e) {
      setError(String(e));
      setWorking(false);
    }
  }

  return (
    <Notice title={t("setupTitle")} live>
      <p className={body}>{t("setupBody")}</p>
      {error && <p className="text-[13px] text-destructive-foreground">{t("setupFailed", { error })}</p>}
      <Button onClick={install} disabled={working}>
        {working ? t("setupWorking") : t("setupAction")}
      </Button>
    </Notice>
  );
}

export function UnsupportedNotice() {
  const t = useT();
  return (
    <Notice title={t("unsupportedTitle")} warn>
      <p className={body}>{t("unsupportedBody")}</p>
    </Notice>
  );
}

export function KeyboardLightNotice() {
  const t = useT();
  return (
    <Notice title={t("klTitle")} warn>
      <p className={body}>{t("klBody")}</p>
    </Notice>
  );
}
