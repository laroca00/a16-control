import { useCallback, useEffect, useState } from "react";
import { Fan, Keyboard, LayoutDashboard, Monitor, Settings, Zap } from "lucide-react";
import type { FanMode } from "@/api";
import { TitleBar } from "@/components/TitleBar";
import { KeyboardLightNotice, SetupNotice, UnsupportedNotice, UpdateNotice } from "@/components/Notices";
import { SidebarNav, type NavGroupData, type NavItemData } from "@/components/ui/dashboard-sidebar";
import { useFanControl } from "@/hooks/useFanControl";
import { useT, type Key } from "@/i18n";
import { useUpdate } from "@/lib/update";
import { cn } from "@/lib/utils";
import { FanPage } from "@/pages/FanPage";
import { PowerPage } from "@/pages/PowerPage";
import { KeyboardPage } from "@/pages/KeyboardPage";
import { DisplayPage } from "@/pages/DisplayPage";
import { Overview } from "@/pages/Overview";
import { SettingsPage } from "@/pages/SettingsPage";

export type Page = "overview" | "fan" | "power" | "keyboard" | "display" | "settings";

const PAGES: { id: Page; label: Key; icon: NavItemData["icon"] }[] = [
  { id: "overview", label: "navOverview", icon: LayoutDashboard },
  { id: "fan", label: "navFan", icon: Fan },
  { id: "power", label: "navPower", icon: Zap },
  { id: "keyboard", label: "navKeyboard", icon: Keyboard },
  { id: "display", label: "navDisplay", icon: Monitor },
  { id: "settings", label: "navSettings", icon: Settings },
];

const MODE_LABEL: Record<FanMode, Key> = { max: "fanMax", auto: "fanModeAuto", quiet: "fanModeQuiet" };

/** The product name over the live fan mode; the dot is bole while Max Fan holds the fans. */
function Brand({ mode }: { mode: FanMode | null }) {
  const t = useT();
  return (
    <div className="mb-4 flex items-center gap-3 py-2" data-tauri-drag-region>
      <svg viewBox="0 0 32 32" className="size-8 shrink-0 rounded-md bg-secondary p-1.5 text-primary" aria-hidden="true">
        <g fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" transform="rotate(135 16 16)">
          <circle cx="16" cy="16" r="12" strokeDasharray="56.5 100" />
          <circle cx="16" cy="16" r="6.5" strokeDasharray="30.6 100" />
        </g>
      </svg>
      <div className="flex min-w-0 flex-col gap-1 overflow-hidden whitespace-nowrap" data-tauri-drag-region>
        <span className="text-[13px] leading-none font-medium">{t("appName")}</span>
        <span className="flex items-center gap-1.5 text-[11px] leading-none text-muted-foreground">
          <span
            className={cn("size-1.5 rounded-full", mode === "max" ? "bg-chart-2" : mode ? "bg-chart-1" : "bg-chart-muted")}
          />
          {mode ? t(MODE_LABEL[mode]) : "–"}
        </span>
      </div>
    </div>
  );
}

const SIDEBAR_KEY = "gienemy.sidebarCollapsed";

/** Whether the sidebar is a rail of icons; remembered between launches, Ctrl+B flips it. */
function useSidebarCollapsed() {
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem(SIDEBAR_KEY) === "1");
  const toggle = useCallback(() => setCollapsed((c) => !c), []);

  useEffect(() => {
    localStorage.setItem(SIDEBAR_KEY, collapsed ? "1" : "0");
  }, [collapsed]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "b") {
        e.preventDefault();
        toggle();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggle]);

  return [collapsed, toggle] as const;
}

export function MainWindow() {
  const t = useT();
  const [page, setPage] = useState<Page>("overview");
  const [collapsed, toggleSidebar] = useSidebarCollapsed();
  const control = useFanControl();
  const update = useUpdate({ auto: true });
  const release = update.state.status === "available" ? update.state.release : null;

  const nav = (ids: Page[]): NavItemData[] =>
    PAGES.filter((p) => ids.includes(p.id)).map((p) => ({ id: p.id, title: t(p.label), icon: p.icon }));
  const groups: NavGroupData[] = [{ items: nav(["overview", "fan", "power", "keyboard", "display"]) }];
  const current = PAGES.find((p) => p.id === page)!;

  return (
    <div className="flex h-full">
      <SidebarNav
        groups={groups}
        bottomItems={nav(["settings"])}
        activeId={page}
        onSelect={(id) => setPage(id as Page)}
        collapsed={collapsed}
        header={<Brand mode={control.telemetry?.fanMode ?? null} />}
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <TitleBar title={t(current.label)} sidebarCollapsed={collapsed} onToggleSidebar={toggleSidebar} />
        <main className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-6">
          {(control.needsSetup || control.unsupported || control.status?.keyboardLight || release) && (
            <div className="grid gap-2">
              {control.needsSetup && <SetupNotice />}
              {control.unsupported && <UnsupportedNotice />}
              {control.status?.keyboardLight && <KeyboardLightNotice />}
              {release && <UpdateNotice version={release.version} url={release.url} />}
            </div>
          )}
          <div key={page} className="flex flex-1 flex-col animate-in fade-in slide-in-from-bottom-1 duration-200">
            {page === "overview" && <Overview control={control} />}
            {page === "fan" && <FanPage control={control} />}
            {page === "power" && <PowerPage />}
            {page === "keyboard" && <KeyboardPage />}
            {page === "display" && <DisplayPage />}
            {page === "settings" && <SettingsPage status={control.status} />}
          </div>
        </main>
      </div>
    </div>
  );
}
