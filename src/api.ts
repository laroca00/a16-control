import { invoke } from "@tauri-apps/api/core";

/** quiet holds the fans near 2400-2700 RPM and hands them to the firmware when hot. */
/** curve moves between quiet, auto and max by temperature, see FanCurve. */
export type FanMode = "max" | "auto" | "quiet" | "curve";

/** °C: quiet below quietBelow, max from maxFrom, the firmware in between. */
export type FanCurve = { quietBelow: number; maxFrom: number };

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
  /** Firmware methods for the charge limit, locks, lights and GPU mode. */
  wmiFeatures: boolean;
  keyboardLight: boolean;
};

export type Config = { lang: string | null };

/** Heat warning: notify when the hotter chip stays at or over `temp` °C. */
export type AlertOptions = { enabled: boolean; temp: number; maxFan: boolean };

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

export type GpuView = { state: "d0" | "d3" | "other" | null; igpuOnBattery: boolean; autoReset: boolean };

export type AbOptions = { enabled: boolean; ocProfile: number; stockProfile: number };
export type AfterburnerView = { installed: boolean; options: AbOptions };

export type BatterySummary = { avgWatts: number | null; avgCpu: number; idlePct: number; targets: string[]; minutes: number };
export type BatteryView = { watching: boolean; progress: number; summary: BatterySummary | null; ecoEnabled: boolean; targets: string[] };

export type PowerView = {
  mode: UiMode;
  profile: ProfileId | null;
  reason: Reason | null;
  /** APU limits in milliwatts. */
  limits: { fppt: number; sppt: number; spl: number } | null;
  options: PowerOptions;
  lastError: string | null;
};

export type Toggle = "fnLock" | "winLock" | "touchpad" | "powerLed" | "batteryLed" | "bootAnimation";
export type GpuMode = "hybrid" | "discrete";
export type HardwareView = {
  /** The firmware setters are registered; false until setup runs again. */
  supported: boolean;
  /** 100 means charging is not stopped early. */
  chargeLimit: number | null;
  toggles: Partial<Record<Toggle, boolean>>;
  winLockInGames: boolean;
  /** The mode the next boot uses. */
  gpuMode: GpuMode | null;
  /** The mode this boot runs in. */
  gpuActive: GpuMode | null;
  /** Capacities in mWh. */
  battery: { design: number; full: number; cycles: number | null } | null;
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
  getAfterburner: () => invoke<AfterburnerView>("get_afterburner"),
  setAfterburner: (options: AbOptions) => invoke<AfterburnerView>("set_afterburner", { options }),
  getBattery: () => invoke<BatteryView>("get_battery"),
  startWatch: () => invoke<BatteryView>("start_watch"),
  cancelWatch: () => invoke<BatteryView>("cancel_watch"),
  setEcoEnabled: (on: boolean) => invoke<BatteryView>("set_eco_enabled", { on }),
  getGpu: () => invoke<GpuView>("get_gpu"),
  setGpuOption: (key: "igpuOnBattery" | "autoReset", value: boolean) => invoke<GpuView>("set_gpu_option", { key, value }),
  getDisplay: () => invoke<DisplayView>("get_display"),
  setBrightness: (value: number) => invoke<DisplayView>("set_brightness", { value }),
  setRefreshRate: (hz: number) => invoke<DisplayView>("set_refresh_rate", { hz }),
  screenOff: () => invoke<void>("screen_off"),
  screenOn: () => invoke<void>("screen_on"),
  getAlerts: () => invoke<AlertOptions>("get_alerts"),
  setAlerts: (options: AlertOptions) => invoke<AlertOptions>("set_alerts", { options }),
  getHardware: () => invoke<HardwareView>("get_hardware"),
  setChargeLimit: (limit: number) => invoke<HardwareView>("set_charge_limit", { limit }),
  setHwToggle: (toggle: Toggle, on: boolean) => invoke<HardwareView>("set_hw_toggle", { toggle, on }),
  setWinLockInGames: (on: boolean) => invoke<HardwareView>("set_win_lock_in_games", { on }),
  setGpuMode: (mode: GpuMode) => invoke<HardwareView>("set_gpu_mode", { mode }),
  restartNow: () => invoke<void>("restart_now"),
  getFanCurve: () => invoke<FanCurve>("get_fan_curve"),
  setFanCurve: (curve: FanCurve) => invoke<FanCurve>("set_fan_curve", { curve }),
  openReleasePage: (url: string) => invoke<void>("open_release_page", { url }),
};
