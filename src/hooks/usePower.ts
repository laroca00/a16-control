import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type PowerOptions, type PowerView, type UiMode } from "../api";

export function usePower() {
  const [power, setPower] = useState<PowerView | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .getPower()
      .then((p) => alive && setPower(p))
      .catch(() => {});
    const un = listen<PowerView>("power", (e) => setPower(e.payload));
    return () => {
      alive = false;
      void un.then((f) => f());
    };
  }, []);

  const setMode = useCallback(async (mode: UiMode) => {
    // Show the choice straight away; applying takes a moment.
    setPower((p) => (p ? { ...p, mode } : p));
    try {
      setPower(await api.setPowerMode(mode));
    } catch {
      setPower(await api.getPower().catch(() => null));
    }
  }, []);

  const setOption = useCallback(async (key: keyof PowerOptions, value: boolean) => {
    setPower(await api.setPowerOption(key, value));
  }, []);

  return { power, setMode, setOption };
}
