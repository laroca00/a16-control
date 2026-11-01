import { useCallback, useEffect, useState } from "react";
import { api, type FanMode, type SetupStatus } from "../api";
import { useTelemetry } from "./useTelemetry";

/** Shared by the panel and the main window: live readings, setup state and the fan toggle. */
export function useFanControl() {
  const telemetry = useTelemetry();
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.getSetupStatus().then(setStatus).catch(() => {});
  }, []);

  const toggle = useCallback(
    async (next: FanMode) => {
      setBusy(true);
      setError(null);
      try {
        telemetry.update(await api.setFan(next));
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    },
    [telemetry],
  );

  const needsSetup = status !== null && (!status.wmiSet || !status.wmiGet || !status.task);
  return { telemetry: telemetry.data, status, needsSetup, busy, error, toggle };
}
