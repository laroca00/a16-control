# Firmware methods on the GAMING A16 (3VH)

Read on 2026-11-28 from the BIOS's DSDT (`HKLM\HARDWARE\ACPI\DSDT`, table
`GBTUACPI`). The setter GUID `ABBC0F75-8EA1-11d1-00A0-C90629100000` maps to
object `BD`, so its dispatcher is `WMBD(instance, method id, data)`; the getter
dispatcher is `WMBC`. Each case writes or reads one field in the EC's `PECM`
field list (region `ERAM`), or a CMOS byte through `WXCM`/`RXCM`
(index `XCMI`, data `XCMD`).

Setter and getter IDs match, except that a few exist on only one side.

| ID | Setter | Getter | Field | Values | Verified on the device |
|---|---|---|---|---|---|
| 100 | SetChargePolicy | GetChargePolicy | `BCPS` (4 bits) | 0 standard, 1 stop at `BCPC` | write and read back |
| 101 | SetChargeStop | GetChargeStop | `BCPC` (8 bits) | percent | write and read back |
| 135 | SetPwrLightDisplay | getPwrLightDisplay | `PLED` (inverted in ACPI) | 1 on, 0 off | write and read back |
| 136 | SetBatteryLightDisplay | getBatteryLightDisplay | `BLED` (inverted in ACPI) | 1 on, 0 off | write and read back |
| 163 | SetPostAnimate | getPostAnimate | CMOS `0xD1` | 1 on, 0 off | read only |
| 201 | SetFnKeyLockStatus | getFnKeyLockStatus | `FNKS` (1 bit) | 1 locked | write and read back |
| 202 | SetTouchPadStatus | getTouchPadStatus | `PSON` (1 bit) | 0 touchpad on, 1 off (inferred: reads 0 while the touchpad works) | read only |
| 203 | SetWinKeyLockStatus | none | `WINK` (1 bit) | 1 locked | not yet |
| 230 | SetPEG2orSG2 | GetPEG2orSG2 | CMOS `0xD0` (getter masks 0x7F) | 0 hybrid, 1 dGPU only (reads 1 while NVIDIA drives the panel) | read only |
| 97 | none | GetBatteryHealth | `BHEA` | reads 0 on this unit | |
| 241–243 | SetApuParameter3..1 | none | `ECPT` | **milliwatts**: the BIOS divides by 1000 and writes watts to the EC | |
| 80 | SetFanDuty (not GPU mode) | GetPEGorSG | `FDTY` | fan duty; the getter's name is misleading | |

Notes

- Gigabyte's own `GB_WMIACPI_Set` registration on this machine declared only
  the fan, APU and NVIDIA methods, so the ones above could not be called until
  `gienemy-set.mof` added them.
- The GPU mode is a CMOS byte the BIOS reads at power-on; a change needs a
  restart. Which mode is active is read from the adapter that drives the
  primary display.
- The Windows key lock has no getter, so GiENEMY keeps its own copy and writes
  it again at start-up.
- Battery health comes from Windows (`BatteryStaticData`,
  `BatteryFullChargedCapacity`, `BatteryCycleCount` in `root\WMI`), not `BHEA`.
