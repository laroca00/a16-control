<p align="center"><img src="assets/logo.svg" width="96" alt="GiENEMY logo"></p>

# GiENEMY

GiENEMY puts fan control back on the Gigabyte GAMING A16 (3VH). On this laptop GiMATE does not show its fan settings, so the fans follow the firmware curve and the CPU throttles in long gaming sessions. GiENEMY talks to the same firmware interface directly and adds a Max Fan switch to the tray.

On the test machine, Max Fan took both fans from about 4,400/4,900 RPM to 6,380 RPM, and the CPU dropped from 74 °C to 59 °C within a minute.

## What it does

- Max Fan on and off, from the tray panel or the main window
- Live speed of both fans and CPU and GPU temperature
- Turkish and English interface, picked from the Windows language on first start
- Starts at sign-in without a UAC prompt

Power modes, keyboard lighting and display controls are planned for later versions.

## Requirements

- Gigabyte GAMING A16 3VH on Windows 11. Other Gigabyte laptops expose a similar interface, but only the A16 is tested.
- WebView2, which Windows 11 already includes.

## Install

1. Download `GiENEMY_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it. Windows asks for administrator approval once, during installation.
2. Open GiENEMY from the Start menu. From then on it also starts at sign-in.
3. Click the tray icon to open the panel.

Max Fan stays on until you turn it off. When you quit GiENEMY with Max Fan on, it hands the fans back to the firmware first.

## How it works

The BIOS exposes fan methods over ACPI-WMI (`root\WMI`, classes `GB_WMIACPI_Set` and `GB_WMIACPI_Get`). Max Fan turns the automatic fan governor off, sets a fixed duty of 100 on both fans and locks it. Turning it off releases the lock and switches the governor back on (`SetAutoFanStatus(1)`).

The installer registers the WMI classes only where they are missing, so a fuller definition installed by Gigabyte's software is left alone. It also creates a scheduled task named `GiENEMY` that starts the app with administrator rights at sign-in, which is why no UAC prompt appears afterwards. Because that task runs elevated, GiENEMY installs into Program Files, where only administrators can change the files it starts.

GiENEMY only calls these runtime WMI methods. It does not write to the BIOS, DMI or NVRAM, and it does not change CPU power limits.

If KeyboardLight or another tool that drives the same fans is installed, GiENEMY warns you, because the two will undo each other's settings.

## Uninstall

Uninstall GiENEMY from Windows settings. The uninstaller hands the fans back to the firmware, stops the app and removes the scheduled task. The WMI class definitions stay, because Gigabyte's own software uses them too.

## Build from source

You need Rust (stable), Node.js 22 or newer and pnpm.

```powershell
pnpm install
pnpm test                                        # frontend tests
cargo test --manifest-path src-tauri/Cargo.toml  # backend tests
pnpm tauri build                                 # installer in src-tauri/target/release/bundle/nsis
```

From an elevated terminal, `GiENEMY.exe --probe` prints the current fan readings and `GiENEMY.exe --fan max|auto` switches the fans.

## License

[MIT](LICENSE)

Not affiliated with or endorsed by GIGABYTE. GIGABYTE and AORUS are trademarks of GIGA-BYTE Technology Co., Ltd.

---

## Türkçe

GiENEMY, Gigabyte GAMING A16 (3VH) için fan kontrolü sağlar. Bu modelde GiMATE fan ayarlarını göstermiyor, fanlar firmware eğrisine göre dönüyor ve uzun oyunlarda işlemci ısınıp yavaşlıyor. GiENEMY aynı firmware arayüzüne doğrudan bağlanır ve tray'e bir Max Fan düğmesi ekler.

Kurulum için [Releases](../../releases) sayfasından installer'ı indirip çalıştır. Windows kurulum sırasında bir kez yönetici onayı ister. Sonrasında uygulama oturum açılınca sormadan başlar.

Max Fan sen kapatana kadar açık kalır. Max Fan açıkken uygulamadan çıkarsan fanlar önce otomatik moda döner. GiENEMY BIOS'a, DMI'a ya da işlemci güç limitlerine dokunmaz.
