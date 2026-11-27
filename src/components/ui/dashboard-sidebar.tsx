import type { ElementType, ReactNode } from "react";
import { cn } from "@/lib/utils";

export type NavItemData = {
  id: string;
  title: string;
  icon: ElementType;
};

export type NavGroupData = {
  heading?: string;
  items: NavItemData[];
};

function NavItem({
  item,
  active,
  collapsed,
  onSelect,
}: {
  item: NavItemData;
  active: boolean;
  collapsed: boolean;
  onSelect: (id: string) => void;
}) {
  const Icon = item.icon;
  return (
    <button
      type="button"
      aria-current={active ? "page" : undefined}
      title={collapsed ? item.title : undefined}
      onClick={() => onSelect(item.id)}
      className={cn(
        "group flex h-8 w-full shrink-0 cursor-pointer items-center gap-2.5 rounded-md px-2 text-left text-[13px] whitespace-nowrap transition-colors select-none",
        active
          ? "bg-sidebar-accent font-medium text-foreground"
          : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-foreground",
      )}
    >
      <Icon
        aria-hidden="true"
        strokeWidth={1.5}
        className={cn(
          "size-4 shrink-0 transition-colors",
          active ? "text-primary" : "text-muted-foreground/80 group-hover:text-foreground/80",
        )}
      />
      {/* The label stays for screen readers while the rail shows icons only. */}
      <span className={cn("truncate", collapsed && "sr-only")}>{item.title}</span>
    </button>
  );
}

export function SidebarNav({
  groups,
  bottomItems = [],
  activeId,
  onSelect,
  header,
  collapsed = false,
  className,
}: {
  groups: NavGroupData[];
  bottomItems?: NavItemData[];
  activeId: string;
  onSelect: (id: string) => void;
  /** Drawn above the navigation, in place of a workspace switcher. */
  header?: ReactNode;
  /** Narrows the sidebar to a rail of icons. */
  collapsed?: boolean;
  className?: string;
}) {
  return (
    <aside
      data-collapsed={collapsed || undefined}
      className={cn(
        "flex h-full shrink-0 flex-col overflow-hidden border-r border-sidebar-border bg-sidebar p-3 transition-[width] duration-200",
        collapsed ? "w-[57px]" : "w-[240px]",
        className,
      )}
    >
      {header}
      <nav className="flex flex-1 flex-col gap-6 overflow-x-hidden overflow-y-auto [scrollbar-width:none]">
        {groups.map((group, i) => (
          <div key={group.heading ?? i} className="flex flex-col gap-0.5">
            {group.heading && !collapsed && (
              <span className="mb-1 px-2 text-xs whitespace-nowrap text-muted-foreground/70">{group.heading}</span>
            )}
            {group.items.map((item) => (
              <NavItem key={item.id} item={item} active={activeId === item.id} collapsed={collapsed} onSelect={onSelect} />
            ))}
          </div>
        ))}
      </nav>
      {bottomItems.length > 0 && (
        <div className="mt-auto flex flex-col gap-0.5 border-t border-sidebar-border pt-3">
          {bottomItems.map((item) => (
            <NavItem key={item.id} item={item} active={activeId === item.id} collapsed={collapsed} onSelect={onSelect} />
          ))}
        </div>
      )}
    </aside>
  );
}
