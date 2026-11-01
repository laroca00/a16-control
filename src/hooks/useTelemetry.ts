import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Telemetry } from "../api";

export function useTelemetry(): { data: Telemetry | null; error: string | null; update: (t: Telemetry) => void } {
  const [data, setData] = useState<Telemetry | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    // Show something immediately instead of waiting for the first tick.
    api
      .getTelemetry()
      .then((t) => alive && (setData(t), setError(null)))
      .catch((e) => alive && setError(String(e)));
    const ok = listen<Telemetry>("telemetry", (e) => {
      setData(e.payload);
      setError(null);
    });
    const bad = listen<string>("telemetry-error", (e) => setError(e.payload));
    return () => {
      alive = false;
      void ok.then((f) => f());
      void bad.then((f) => f());
    };
  }, []);

  return { data, error, update: setData };
}
