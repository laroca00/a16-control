import type { en } from "./en";

export const tr: Record<keyof typeof en, string> = {
  appName: "GiENEMY",
  loading: "Fanlar okunuyor…",
  errorGeneric: "Bir şeyler ters gitti: {error}",

  fanMax: "Max Fan",
  fanOn: "Açık",
  fanOff: "Kapalı",
  fanWorking: "Uygulanıyor…",
  fanRpm: "{rpm1} / {rpm2} RPM",
  fanToggleHint: "Max Fan'ı aç ya da kapat",
  cpu: "CPU",
  gpu: "GPU",
  tempValue: "{value}°C",
  allSettings: "Tüm ayarlar",

  setupTitle: "Kurulum gerekli",
  setupBody: "GiENEMY'nin fanlara ulaşması için bir kez yönetici onayı gerekiyor. Sonrasında hiç sormadan açılır.",
  setupAction: "Kur",
  setupWorking: "Kuruluyor…",
  setupFailed: "Kurulum tamamlanmadı: {error}",

  klTitle: "KeyboardLight hâlâ yüklü",
  klBody: "İkisi aynı fanları yönetip birbirinin ayarını bozuyor. KeyboardLight'ı kaldır.",
};
