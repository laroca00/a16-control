import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, PanelLeftClose, PanelLeftOpen, Square, X } from "lucide-react";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";

const button =
  "grid h-8 w-10 cursor-pointer place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground [&_svg]:size-3.5";

type Props = {
  title?: string;
  /** Shows the sidebar toggle when given. */
  sidebarCollapsed?: boolean;
  onToggleSidebar?: () => void;
};

/** Replaces the Windows caption: sidebar toggle and page name on the left, window buttons on the right. */
export function TitleBar({ title, sidebarCollapsed, onToggleSidebar }: Props) {
  const t = useT();
  const win = getCurrentWindow();
  return (
    <header className="flex h-12 shrink-0 items-center justify-between gap-2 border-b pr-2 pl-3" data-tauri-drag-region>
      <div className="flex min-w-0 items-center gap-2" data-tauri-drag-region>
        {onToggleSidebar && (
          <button
            type="button"
            className={cn(button, "w-8 [&_svg]:size-[18px]")}
            aria-label={sidebarCollapsed ? t("sidebarExpand") : t("sidebarCollapse")}
            aria-keyshortcuts="Control+B"
            title={`${sidebarCollapsed ? t("sidebarExpand") : t("sidebarCollapse")} (Ctrl+B)`}
            onClick={onToggleSidebar}
          >
            {sidebarCollapsed ? <PanelLeftOpen strokeWidth={1.5} aria-hidden="true" /> : <PanelLeftClose strokeWidth={1.5} aria-hidden="true" />}
          </button>
        )}
        {title && (
          <h1 className="truncate text-sm font-medium" data-tauri-drag-region>
            {title}
          </h1>
        )}
      </div>
      <div className="flex gap-0.5">
        <button type="button" className={button} aria-label={t("winMinimize")} onClick={() => win.minimize()}>
          <Minus strokeWidth={1.5} aria-hidden="true" />
        </button>
        <button type="button" className={button} aria-label={t("winMaximize")} onClick={() => win.toggleMaximize()}>
          <Square strokeWidth={1.5} aria-hidden="true" className="!size-3" />
        </button>
        <button
          type="button"
          className={cn(button, "hover:bg-destructive hover:text-white")}
          aria-label={t("winClose")}
          onClick={() => win.close()}
        >
          <X strokeWidth={1.5} aria-hidden="true" />
        </button>
      </div>
    </header>
  );
}
