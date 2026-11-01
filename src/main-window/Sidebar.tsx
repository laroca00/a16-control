import { useT, type Key } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

export type Page = "overview" | "fan" | "power" | "keyboard" | "display" | "settings";

const PAGES: { id: Page; label: Key }[] = [
  { id: "overview", label: "navOverview" },
  { id: "fan", label: "navFan" },
  { id: "power", label: "navPower" },
  { id: "keyboard", label: "navKeyboard" },
  { id: "display", label: "navDisplay" },
  { id: "settings", label: "navSettings" },
];

export function Sidebar({ page, onSelect }: { page: Page; onSelect: (p: Page) => void }) {
  const t = useT();
  return (
    <LiquidGlass className="sidebar" radius={24}>
      <div className="sidebar__brand">{t("appName")}</div>
      <nav className="sidebar__nav">
        {PAGES.map((p) => (
          <button
            key={p.id}
            type="button"
            className={`sidebar__item${p.id === page ? " sidebar__item--active" : ""}`}
            aria-current={p.id === page ? "page" : undefined}
            onClick={() => onSelect(p.id)}
          >
            {t(p.label)}
          </button>
        ))}
      </nav>
    </LiquidGlass>
  );
}
