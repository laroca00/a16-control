import { useCallback, useEffect, useState } from "react";
import { api, SETUP_NEEDED, type FanMode, type SetupStatus } from "../api";
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

  // A getter class that exists without our methods is Gigabyte's own; setup leaves it alone.
  const unsupported = status !== null && !status.wmiGet && status.getClass;
  const needsSetup =
    status !== null && !unsupported && (!status.wmiSet || !status.wmiGet || !status.task);
  const unreachable = telemetry.error && telemetry.error !== SETUP_NEEDED ? telemetry.error : null;
  return { telemetry: telemetry.data, status, needsSetup, unsupported, unreachable, busy, error, toggle };
}
