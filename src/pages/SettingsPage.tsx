import { useState } from "react";
import { Check, Minus, X } from "lucide-react";
import { api, type SetupStatus } from "@/api";
import { Button } from "@/components/ui/button";
import { Section, Sections } from "@/components/ui/section";
import { Segmented } from "@/components/ui/segmented";
import { useLang, useT, type Key, type Lang } from "@/i18n";
import { cn } from "@/lib/utils";

const CHECKS: { key: Key; ok: (s: SetupStatus) => boolean }[] = [
  { key: "checkElevated", ok: (s) => s.elevated },
  { key: "checkTask", ok: (s) => s.task },
  { key: "checkWmiSet", ok: (s) => s.wmiSet },
  { key: "checkWmiGet", ok: (s) => s.wmiGet },
  { key: "checkKeyboardLight", ok: (s) => !s.keyboardLight },
];

const LANGS: { value: Lang; label: string }[] = [
  { value: "tr", label: "Türkçe" },
  { value: "en", label: "English" },
];

export function SettingsPage({ status }: { status: SetupStatus | null }) {
  const t = useT();
  const [lang, setLang] = useLang();
  const [error, setError] = useState<string | null>(null);

  async function uninstall() {
    try {
      await api.runUninstall();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <Sections>
      <Section title={t("language")} titleId="lang-title">
        <Segmented aria-labelledby="lang-title" options={LANGS} value={lang} onChange={setLang} />
      </Section>

      <Section title={t("setupChecks")}>
        <ul className="flex flex-col gap-2">
          {CHECKS.map((c) => {
            const ok = status ? c.ok(status) : null;
            const Icon = ok === null ? Minus : ok ? Check : X;
            return (
              <li key={c.key} className="flex items-center gap-2.5 text-[13px]">
                <span
                  aria-hidden="true"
                  className={cn(
                    "grid size-5 shrink-0 place-items-center rounded-full",
                    ok === null ? "bg-muted text-muted-foreground" : ok ? "bg-chart-1/15 text-chart-1" : "bg-destructive/15 text-destructive-foreground",
                  )}
                >
                  <Icon className="size-3" strokeWidth={2} />
                </span>
                {t(c.key)}
              </li>
            );
          })}
        </ul>
      </Section>

      <Section title={t("uninstallTitle")} description={t("uninstallBody")}>
        {error && <p className="text-[13px] text-destructive-foreground">{t("errorGeneric", { error })}</p>}
        <Button variant="destructive" onClick={uninstall}>
          {t("uninstallAction")}
        </Button>
      </Section>
    </Sections>
  );
}
