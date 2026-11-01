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

export type UiMode = "battery" | "auto" | "performance";
export type ProfileId = "battery" | "balancedAc" | "balancedDc" | "performance";
export type Reason = "userBattery" | "userPerformance" | "autoAc" | "autoDc" | "autoLowBattery" | "autoGame";
export type PowerOptions = { autoGame: boolean; acdcSwitch: boolean };

export type Rgb = [number, number, number];
export type KeyboardView = {
  rgb: Rgb;
  brightness: number;
  rainbow: boolean;
  presets: { name: string; rgb: Rgb }[];
};

export type DisplayView = { brightness: number | null; refresh: number | null; rates: number[]; screenOff: boolean };

export type PowerView = {
  mode: UiMode;
  profile: ProfileId | null;
  reason: Reason | null;
  /** APU limits in milliwatts. */
  limits: { fppt: number; sppt: number; spl: number } | null;
  options: PowerOptions;
  lastError: string | null;
};

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
  getPower: () => invoke<PowerView>("get_power"),
  setPowerMode: (mode: UiMode) => invoke<PowerView>("set_power_mode", { mode }),
  setPowerOption: (key: keyof PowerOptions, value: boolean) => invoke<PowerView>("set_power_option", { key, value }),
  getKeyboard: () => invoke<KeyboardView>("get_keyboard"),
  setKeyboardColor: (rgb: Rgb) => invoke<KeyboardView>("set_keyboard_color", { rgb }),
  setKeyboardBrightness: (value: number) => invoke<KeyboardView>("set_keyboard_brightness", { value }),
  setKeyboardRainbow: (on: boolean) => invoke<KeyboardView>("set_keyboard_rainbow", { on }),
  getDisplay: () => invoke<DisplayView>("get_display"),
  setBrightness: (value: number) => invoke<DisplayView>("set_brightness", { value }),
  setRefreshRate: (hz: number) => invoke<DisplayView>("set_refresh_rate", { hz }),
  screenOff: () => invoke<void>("screen_off"),
  screenOn: () => invoke<void>("screen_on"),
};
