import { getCurrentWindow } from "@tauri-apps/api/window";
import { useT } from "../i18n";
import "./TitleBar.css";

// Strokes drawn on a 10×10 grid so the glyphs stay crisp at any scale.
const ICONS = {
  minimize: "M1 5.5h8",
  maximize: "M1.5 1.5h7v7h-7z",
  close: "M1.5 1.5l7 7M8.5 1.5l-7 7",
};

function Glyph({ d }: { d: string }) {
  return (
    <svg viewBox="0 0 10 10" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

/** Replaces the Windows caption so the glass reaches the top edge. */
export function TitleBar() {
  const t = useT();
  const win = getCurrentWindow();
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar__buttons">
        <button type="button" className="titlebar__button" aria-label={t("winMinimize")} onClick={() => win.minimize()}>
          <Glyph d={ICONS.minimize} />
        </button>
        <button type="button" className="titlebar__button" aria-label={t("winMaximize")} onClick={() => win.toggleMaximize()}>
          <Glyph d={ICONS.maximize} />
        </button>
        <button
          type="button"
          className="titlebar__button titlebar__button--close"
          aria-label={t("winClose")}
          onClick={() => win.hide()}
        >
          <Glyph d={ICONS.close} />
        </button>
      </div>
    </header>
  );
}
