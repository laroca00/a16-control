import { useEffect, useState } from "react";
import { api, type DisplayView } from "@/api";
import { Button } from "@/components/ui/button";
import { Section, Sections } from "@/components/ui/section";
import { Segmented } from "@/components/ui/segmented";
import { Slider } from "@/components/ui/slider";
import { useT } from "@/i18n";

export function DisplayPage() {
  const t = useT();
  const [d, setD] = useState<DisplayView | null>(null);
  // Brightness writes go through WMI one by one; send only where the drag ends.
  const [draft, setDraft] = useState<number | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .getDisplay()
      .then((v) => alive && setD(v))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  const run = (p: Promise<DisplayView>) => p.then(setD).catch(() => {});

  const commitBrightness = () => {
    if (draft == null) return;
    const value = draft;
    setDraft(null);
    setD((v) => (v ? { ...v, brightness: value } : v));
    run(api.setBrightness(value));
  };

  return (
    <Sections>
      <Section title={t("dispBrightness")}>
        <Slider
          label={t("dispBrightness")}
          hideLabel
          min={5}
          max={100}
          step={5}
          value={draft ?? d?.brightness ?? 50}
          output={`${draft ?? d?.brightness ?? "–"}%`}
          disabled={d?.brightness == null}
          onChange={setDraft}
          onCommit={commitBrightness}
        />
      </Section>

      {d && d.rates.length > 1 && (
        <Section title={t("dispRefresh")} titleId="refresh-title">
          <Segmented
            aria-labelledby="refresh-title"
            options={d.rates.map((hz) => ({ value: hz, label: t("hzValue", { value: hz }) }))}
            value={d.refresh}
            onChange={(hz) => run(api.setRefreshRate(hz))}
          />
        </Section>
      )}

      <Section title={t("dispBlank")} description={t("dispOffHint")}>
        <Button variant="secondary" onClick={() => void api.screenOff()}>
          {t("dispOff")}
        </Button>
      </Section>
    </Sections>
  );
}
