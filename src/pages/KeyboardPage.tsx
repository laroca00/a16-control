import { useEffect, useState, type CSSProperties } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type KeyboardView, type Rgb } from "@/api";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import { SwitchRow } from "@/components/ui/switch";
import { useT, type Key } from "@/i18n";
import { cn } from "@/lib/utils";
import "./KeyboardPage.css";

const PRESET_LABELS: Record<string, Key> = {
  white: "presetWhite",
  red: "presetRed",
  orange: "presetOrange",
  green: "presetGreen",
  cyan: "presetCyan",
  blue: "presetBlue",
  purple: "presetPurple",
  pink: "presetPink",
};

// Key widths per row, in key units, for the backlight preview.
const ROWS = [
  [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2],
  [1.5, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1.5],
  [1.8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2.2],
  [2.3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2.7],
  [1.3, 1.3, 1.3, 6.3, 1.3, 1.3, 1.3],
];

const hex = (rgb: Rgb) => "#" + rgb.map((c) => c.toString(16).padStart(2, "0")).join("");
const fromHex = (h: string): Rgb => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as Rgb;

const swatch =
  "relative size-8 cursor-pointer rounded-full p-0 ring-1 ring-white/20 ring-inset transition-shadow hover:ring-white/50";
// A gap in the card colour, then the gilt ring that marks the chosen colour.
const swatchOn = "shadow-[0_0_0_3px_var(--card),0_0_0_4px_var(--primary)]";

function useKeyboard() {
  const [kb, setKb] = useState<KeyboardView | null>(null);
  useEffect(() => {
    let alive = true;
    api
      .getKeyboard()
      .then((k) => alive && setKb(k))
      .catch(() => {});
    const un = listen<KeyboardView>("keyboard", (e) => setKb(e.payload));
    return () => {
      alive = false;
      void un.then((f) => f());
    };
  }, []);
  return [kb, setKb] as const;
}

export function KeyboardPage() {
  const t = useT();
  const [kb, setKb] = useKeyboard();

  const run = (p: Promise<KeyboardView>) => p.then(setKb).catch(() => {});
  const lit = (kb?.brightness ?? 0) > 0;
  const glow = {
    "--kb-color": kb ? hex(kb.rgb) : "#ffffff",
    "--kb-level": String((kb?.brightness ?? 0) / 100),
  } as CSSProperties;

  return (
    <div className="grid min-h-[360px] flex-1 grid-cols-[minmax(360px,1.3fr)_minmax(280px,1fr)] gap-4">
      <Card className="justify-center gap-4 p-6">
        <div className={cn("kb-preview", kb?.rainbow && "kb-preview--rainbow", !lit && "kb-preview--off")} style={glow} aria-hidden="true">
          {ROWS.map((row, r) => (
            <div key={r} className="kb-preview__row">
              {row.map((w, k) => (
                <span key={k} className="kb-preview__key" style={{ flexGrow: w }} />
              ))}
            </div>
          ))}
        </div>
        <CardDescription className="text-xs">{t("kbHotkeys")}</CardDescription>
      </Card>

      <Card className="items-stretch gap-4 p-6">
        <CardTitle>{t("kbPresets")}</CardTitle>
        <div className="flex flex-wrap gap-3">
          {kb?.presets.map((p) => (
            <button
              key={p.name}
              type="button"
              className={cn(swatch, !kb.rainbow && hex(kb.rgb) === hex(p.rgb) && swatchOn)}
              style={{ background: hex(p.rgb) }}
              aria-label={t(PRESET_LABELS[p.name] ?? "kbCustom")}
              onClick={() => run(api.setKeyboardColor(p.rgb))}
            />
          ))}
          <label className={cn(swatch, "swatch-custom overflow-hidden")} title={t("kbCustom")}>
            <input
              type="color"
              className="absolute inset-0 cursor-pointer opacity-0"
              aria-label={t("kbCustom")}
              value={kb ? hex(kb.rgb) : "#ffffff"}
              onChange={(e) => run(api.setKeyboardColor(fromHex(e.target.value)))}
            />
          </label>
        </div>

        <Slider
          label={t("kbBrightness")}
          min={0}
          max={100}
          step={10}
          value={kb?.brightness ?? 0}
          output={`${kb?.brightness ?? 0}%`}
          disabled={!kb}
          onChange={(v) => run(api.setKeyboardBrightness(v))}
        />

        <SwitchRow
          label={t("kbRainbow")}
          checked={kb?.rainbow ?? false}
          disabled={!kb}
          onCheckedChange={() => run(api.setKeyboardRainbow(!kb?.rainbow))}
        />

        <Button
          variant="secondary"
          className="mt-auto self-start"
          disabled={!kb}
          onClick={() => run(api.setKeyboardBrightness(lit ? 0 : 100))}
        >
          {lit ? t("kbOff") : t("kbOn")}
        </Button>
      </Card>
    </div>
  );
}
