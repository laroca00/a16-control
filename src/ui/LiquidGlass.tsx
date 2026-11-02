import type { CSSProperties, ReactNode } from "react";
import "./glass.css";

export type GlassShape = "rounded" | "pill" | "circle";

type LiquidGlassProps = {
  shape?: GlassShape;
  /** Corner radius in px, used by the rounded shape. */
  radius?: number;
  role?: string;
  className?: string;
  style?: CSSProperties;
  /** Scrolls the children inside the padding instead of scrolling the glass itself. */
  scroll?: boolean;
  /** Drawn behind the scrolling content and stays put while it scrolls. */
  backdrop?: ReactNode;
  children?: ReactNode;
};

// A plain tinted pane. The refracting SVG filter and backdrop blur it once had
// cost each pane its own GPU textures, and over the smooth backdrop they were
// all but invisible.
export function LiquidGlass({
  shape = "rounded",
  radius = 24,
  role,
  className,
  style,
  scroll = false,
  backdrop,
  children,
}: LiquidGlassProps) {
  const classes = ["glass", `glass--${shape}`, scroll && "glass--scroll", className].filter(Boolean).join(" ");
  return (
    <div role={role} className={classes} style={{ ...style, "--glass-radius": `${radius}px` } as CSSProperties}>
      {backdrop}
      {scroll ? <div className="glass__scroll">{children}</div> : children}
    </div>
  );
}
