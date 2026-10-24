# Moto G60 (hanoip, XT2135-1) — facts verified on the device

Collected with `fastboot getvar`, `adb shell su` (KernelSU) and `/sys/firmware/fdt`. No IMEI/serial/MAC here.

| Item | Value | Source |
|---|---|---|
| SoC | SM7150 (`qcom,sdmmagpie`, soc_id 365, msm-id `0x16d`) | fastboot `cpu`, DTB `compatible` |
| Motorola platform name | `sm6150` (`ro.board.platform`) — not the SoC, just Motorola naming | getprop |
| RAM | 4 GB LPDDR4x | fastboot `ram` |
| Storage | UFS, 128 GB Samsung KM5V8001DM-B622 (`androidboot.bootdevice=1d84000.ufshc`) | fastboot / cmdline |
| Bootloader | MBM-3.0 (ABL/UEFI), `securestate: flashing_unlocked`, verity disabled | fastboot |
| A/B slots | Current slot `b`; `_a` has `slot-successful: no` | fastboot |
| Panel | `tm_ili7807s_fhd_vid` / `dsi_ili7807s_tm_678_vid_display` (video mode) | fastboot / cmdline |
| Android (as reported at collection time) | 12 (`S2RI32.32-20-9-9-2`), custom kernel 4.14.357 (`openela-Cybertron-v18-ksu`); later found to be LineageOS 22.1 (Android 15) with the same kernel | getprop / uname |
| Board-id | `qcom,board-id = <0x4c 0xb200>` | DTB |
| Boot image | Android boot header **v3** (kernel ~42 MB, ramdisk ~19.7 MB) | boot_a.img |
| vendor_boot | Present (`VNDRBOOT`, header v3), holds the cmdline | vendor_boot_a.img |
| dtbo | DTBO format (`d7b7ab1e`) | dtbo_a.img |

## Files in this directory
- `fastboot-getvar.txt` — bootloader variables (sanitized)
- `android-runtime.txt` — cmdline and version of the running Android kernel
- `kernel-android.config` — `/proc/config.gz` of the running Android kernel (6123 lines)
- `running.dtb` / `running.dts` — full downstream DTB (~21.7k lines) decompiled with `dtc -f`

## Backups (outside the repository)
`$HANOIP_BACKUP_DIR/` — `boot_{a,b}`, `vendor_boot_{a,b}`, `dtbo_{a,b}`, `vbmeta_{a,b}`, `persist`, `modemst1`, `modemst2`, `fsg_{a,b}`, `misc`, `abl_{a,b}`. Checksums in `SHA256SUMS`.
