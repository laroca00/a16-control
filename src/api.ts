import { invoke } from "@tauri-apps/api/core";

export type FanMode = "max" | "auto";

export type Telemetry = {
  rpm1: number;
  rpm2: number;
  cpuTemp: number;
  gpuTemp: number;
  cpuDuty: number;
  gpuDuty: number;
  fanMode: FanMode;
};

export type SetupStatus = {
  elevated: boolean;
  task: boolean;
  wmiSet: boolean;
  wmiGet: boolean;
  /** The getter class exists; if it still lacks our methods, setup cannot fix it. */
  getClass: boolean;
  keyboardLight: boolean;
};

export type Config = { lang: string | null };

/** Error text the backend uses when the firmware interface is not set up. */
export const SETUP_NEEDED = "setup";

export const api = {
  getSetupStatus: () => invoke<SetupStatus>("get_setup_status"),
  getTelemetry: () => invoke<Telemetry>("get_telemetry"),
  setFan: (mode: FanMode) => invoke<Telemetry>("set_fan", { mode }),
  runInstall: () => invoke<void>("run_install"),
  runUninstall: () => invoke<void>("run_uninstall"),
  getConfig: () => invoke<Config>("get_config"),
  setLang: (lang: string) => invoke<void>("set_lang", { lang }),
  openMain: () => invoke<void>("open_main"),
  quitApp: () => invoke<void>("quit_app"),
};
