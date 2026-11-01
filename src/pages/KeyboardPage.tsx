import { useEffect, useState, type CSSProperties } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type KeyboardView, type Rgb } from "../api";
import { useT, type Key } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";
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
    <div className="page page--keyboard">
      <h1 className="page__title">{t("navKeyboard")}</h1>
      <div className="keyboard-page">
        <LiquidGlass className="kb-preview-card" radius={28}>
          <div className={`kb-preview${kb?.rainbow ? " kb-preview--rainbow" : ""}${lit ? "" : " kb-preview--off"}`} style={glow} aria-hidden="true">
            {ROWS.map((row, r) => (
              <div key={r} className="kb-preview__row">
                {row.map((w, k) => (
                  <span key={k} className="kb-preview__key" style={{ flexGrow: w }} />
                ))}
              </div>
            ))}
          </div>
          <p className="card__hint">{t("kbHotkeys")}</p>
        </LiquidGlass>

        <LiquidGlass className="settings__card" radius={24}>
          <h2 className="card__title">{t("kbPresets")}</h2>
          <div className="swatches">
            {kb?.presets.map((p) => (
              <button
                key={p.name}
                type="button"
                className={`swatch${!kb.rainbow && hex(kb.rgb) === hex(p.rgb) ? " swatch--on" : ""}`}
                style={{ background: hex(p.rgb) }}
                aria-label={t(PRESET_LABELS[p.name] ?? "kbCustom")}
                onClick={() => run(api.setKeyboardColor(p.rgb))}
              />
            ))}
            <label className="swatch swatch--custom" title={t("kbCustom")}>
              <input
                type="color"
                aria-label={t("kbCustom")}
                value={kb ? hex(kb.rgb) : "#ffffff"}
                onChange={(e) => run(api.setKeyboardColor(fromHex(e.target.value)))}
              />
            </label>
          </div>

          <label className="kb-slider">
            <span>{t("kbBrightness")}</span>
            <input
              type="range"
              min={0}
              max={100}
              step={10}
              aria-label={t("kbBrightness")}
              value={kb?.brightness ?? 0}
              disabled={!kb}
              onChange={(e) => run(api.setKeyboardBrightness(Number(e.target.value)))}
            />
            <output>{kb?.brightness ?? 0}%</output>
          </label>

          <label className="toggle">
            <span>{t("kbRainbow")}</span>
            <button
              type="button"
              role="switch"
              aria-checked={kb?.rainbow ?? false}
              aria-label={t("kbRainbow")}
              className={`toggle__switch${kb?.rainbow ? " toggle__switch--on" : ""}`}
              disabled={!kb}
              onClick={() => run(api.setKeyboardRainbow(!kb?.rainbow))}
            />
          </label>

          <button
            type="button"
            className="ghost-button"
            disabled={!kb}
            onClick={() => run(api.setKeyboardBrightness(lit ? 0 : 100))}
          >
            {lit ? t("kbOff") : t("kbOn")}
          </button>
        </LiquidGlass>
      </div>
    </div>
  );
}
