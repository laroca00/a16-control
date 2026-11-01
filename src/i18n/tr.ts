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

  unsupportedTitle: "Bu laptopun sensör arayüzü desteklenmiyor",
  unsupportedBody: "Firmware, GiENEMY'nin ihtiyaç duyduğu fan ve sıcaklık okumalarını sunmuyor.",
  unreachable: "Fanlara şu an ulaşılamıyor: {error}",

  klTitle: "KeyboardLight hâlâ yüklü",
  klBody: "İkisi aynı fanları yönetip birbirinin ayarını bozuyor. KeyboardLight'ı kaldır.",

  winMinimize: "Küçült",
  winMaximize: "Büyüt",
  winClose: "Tray'e gizle",

  navOverview: "Genel",
  navFan: "Fan",
  navPower: "Güç",
  navKeyboard: "Klavye",
  navDisplay: "Ekran",
  navSettings: "Ayarlar",

  overviewFans: "Fanlar",
  overviewTemps: "Sıcaklıklar",
  fan1: "Fan 1",
  fan2: "Fan 2",
  statusMax: "Fanlar tam devirde kilitli. Oyun bitince Max Fan'ı kapat.",
  statusAuto: "Fan hızını sıcaklığa göre firmware ayarlıyor.",

  fanHistory: "Son üç dakika",
  fanSpeedChart: "Fan hızı",
  tempChart: "Sıcaklık",

  soonTitle: "Sonraki sürümde",
  soonPower: "Pil, Auto ve Performans modları, oyun algılama ve pil güç planı sonraki sürümde geliyor.",
  soonKeyboard: "Klavye rengi, parlaklık, gökkuşağı ve kısayollar sonraki sürümde geliyor.",
  soonDisplay: "Parlaklık adımları, yenileme hızı ve ekranı kapatma sonraki sürümde geliyor.",

  language: "Dil",
  setupChecks: "Kurulum",
  checkElevated: "Yönetici yetkisiyle çalışıyor",
  checkTask: "Oturum açılınca sormadan başlıyor",
  checkWmiSet: "Fan kontrol arayüzü hazır",
  checkWmiGet: "Sensör arayüzü hazır",
  checkKeyboardLight: "KeyboardLight yüklü değil",
  uninstallTitle: "GiENEMY'yi kaldır",
  uninstallBody: "Açılış görevini siler ve uygulamayı kapatır. Dosyaları silmek için Windows ayarlarından kaldır.",
  uninstallAction: "Kaldır",
};
