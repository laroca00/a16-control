import { useEffect } from "react";
import { api } from "../api";

// The click or key that turned the screen off must not turn it back on.
const GRACE_MS = 1500;

/** A plain black window over every screen; any input ends it. */
export function Blackout() {
  useEffect(() => {
    document.body.style.background = "#000";
    document.body.style.cursor = "none";
    const shownAt = Date.now();
    let done = false;
    const wake = () => {
      if (done || Date.now() - shownAt < GRACE_MS) return;
      done = true;
      void api.screenOn();
    };
    const events = ["keydown", "mousedown", "mousemove", "wheel", "touchstart"] as const;
    events.forEach((e) => window.addEventListener(e, wake));
    return () => events.forEach((e) => window.removeEventListener(e, wake));
  }, []);
  return <div style={{ position: "fixed", inset: 0, background: "#000" }} />;
}
