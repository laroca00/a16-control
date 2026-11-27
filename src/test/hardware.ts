import type { HardwareView } from "../api";

/** A machine whose firmware setters are registered. */
export const hwView: HardwareView = {
  supported: true,
  chargeLimit: 100,
  toggles: { fnLock: true, winLock: false, touchpad: true, powerLed: true, batteryLed: true, bootAnimation: true },
  winLockInGames: false,
  gpuMode: "discrete",
  gpuActive: "discrete",
  battery: { design: 76088, full: 58708, cycles: 65 },
};
