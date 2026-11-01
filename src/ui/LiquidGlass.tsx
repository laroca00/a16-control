import { useEffect, useId, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { createDisplacementMap, type DisplacementMap } from "./glassMap";
import "./glass.css";

export type GlassShape = "rounded" | "pill" | "circle";

type LiquidGlassProps = {
  shape?: GlassShape;
  /** Corner radius in px, used by the rounded shape. */
  radius?: number;
  blur?: number;
  /** Multiplier on the refraction offsets. */
  strength?: number;
  role?: string;
  className?: string;
  style?: CSSProperties;
  /** Scrolls the children inside the padding instead of scrolling the glass itself. */
  scroll?: boolean;
  /** Drawn behind the scrolling content and stays put while it scrolls. */
  backdrop?: ReactNode;
  children?: ReactNode;
};

type Size = { width: number; height: number };

// Rebuilding the map costs a few milliseconds per 100k pixels, so wait for a
// window resize to settle; the previous map is stretched meanwhile.
const RESIZE_SETTLE_MS = 120;

export function LiquidGlass({
  shape = "rounded",
  radius = 24,
  blur = 6,
  strength = 1,
  role,
  className,
  style,
  scroll = false,
  backdrop,
  children,
}: LiquidGlassProps) {
  const ref = useRef<HTMLDivElement>(null);
  const filterId = `glass-${useId().replace(/[^\w-]/g, "")}`;
  const [size, setSize] = useState<Size | null>(null);
  const [map, setMap] = useState<DisplacementMap | null>(null);
  const hasMap = useRef(false);

  useEffect(() => {
    const observer = new ResizeObserver(([entry]) => {
      const box = entry.borderBoxSize[0];
      const next = { width: Math.round(box.inlineSize), height: Math.round(box.blockSize) };
      setSize((prev) => (prev?.width === next.width && prev.height === next.height ? prev : next));
    });
    observer.observe(ref.current!);
    return () => observer.disconnect();
  }, []);

  const cornerRadius = !size ? radius : shape === "rounded" ? radius : Math.min(size.width, size.height) / 2;

  useEffect(() => {
    if (!size || size.width === 0 || size.height === 0) return;
    const timer = setTimeout(
      () => {
        hasMap.current = true;
        setMap(createDisplacementMap(size.width, size.height, cornerRadius, strength));
      },
      hasMap.current ? RESIZE_SETTLE_MS : 0,
    );
    return () => clearTimeout(timer);
  }, [size, cornerRadius, strength]);

  const ready = map !== null && size !== null;
  const classes = ["glass", `glass--${shape}`, scroll && "glass--scroll", className].filter(Boolean).join(" ");

  return (
    <div
      ref={ref}
      role={role}
      className={classes}
      style={
        {
          ...style,
          "--glass-radius": `${radius}px`,
          "--glass-blur": `${blur}px`,
          ...(ready && { backdropFilter: `url(#${filterId})` }),
        } as CSSProperties
      }
    >
      {ready && (
        <svg className="glass__defs" aria-hidden="true" focusable="false">
          <filter
            id={filterId}
            x="0"
            y="0"
            width={size.width}
            height={size.height}
            filterUnits="userSpaceOnUse"
            colorInterpolationFilters="sRGB"
          >
            <feGaussianBlur in="SourceGraphic" stdDeviation={blur} edgeMode="duplicate" result="blurred" />
            <feImage
              href={map.href}
              x="0"
              y="0"
              width={size.width}
              height={size.height}
              preserveAspectRatio="none"
              result="map"
            />
            <feDisplacementMap in="blurred" in2="map" scale={map.scale} xChannelSelector="R" yChannelSelector="G" />
          </filter>
        </svg>
      )}
      {backdrop}
      {scroll ? <div className="glass__scroll">{children}</div> : children}
    </div>
  );
}
