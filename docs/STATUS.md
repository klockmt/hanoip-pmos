# Moto G60 (hanoip) on postmarketOS: where it stands

State on 2026-11-02. The goal is to run postmarketOS on the Moto G60 (hanoip) as a headless server and to publish everything
on GitHub with reproducible instructions and a way to update the kernel. How the phone is run: [`HOMELAB.md`](HOMELAB.md).
What broke and where it is kept from breaking again: [`PITFALLS.md`](PITFALLS.md). The kernel plan: section 5 below; the
parked mainline attempt: [`MAINLINE.md`](MAINLINE.md). Open work: the GitHub issues, roadmap #20.

| Area | State | Since | Evidence |
|---|---|---|---|
| Downstream kernel (4.14, cool585 Cybertron v18) | pmOS edge boots on its own from `boot_b` + `vendor_boot_b`, 104 GB root on `userdata`; package `pkgrel 12` with patches 0001-0010 | 2026-10-24 | `scripts/test-kernel-config`; section 4 below |
| Access | SSH over the USB cable and the tailnet only, keys only | 2026-11-04 | `scripts/test-firewall`; HOMELAB |
| Kernel modules | load (patch 0010); the tree must come from the running kernel's build | 2026-11-01 | `scripts/test-kernel-config`; PITFALLS |
| Wi-Fi (WCN3990) | works: 2.4/5 GHz, WPA3, back after a reboot; the userspace is the `-modem` subpackage. A 24 h hold was not measured | 2026-10-30, packaged 2026-11-01 | `scripts/test-modem-package`; `userspace/modem/README.md` |
| Battery percentage, charge limit, heat | `hanoip-qgd` (Rust), `hanoip-charge-limit` (holds 80-82 % on the cable), `hanoip-thermal` | 2026-10-28 | `scripts/test-qgd`, `test-charge-limit`, `test-thermal`; `BATTERY.md` |
| Cycle counters | restored (`bms/cycle_counts` writable), kept across reboots | 2026-11-01 | `scripts/test-cycle-counts`, `test-restore-cycle-counts` |
| Telegram bot and `/status` | Rust since 2026-11-01; the package no longer needs `python3`. Commands checked on Telegram by the owner (issue #22) | 2026-11-01 | `scripts/test-bot`, `scripts/test-status` |
| A/B slot | `qbootctl -m` at every boot marks slot b successful | 2026-10-24 | PITFALLS (docs only) |
| zram | zstd, 5.3 GB, priority 300 | 2026-11-01 | `scripts/test-kernel-config` |
| Containers | namespaces, `pids`/`devices` cgroups, NAT/bridge; use `crun` (the `runc` of Alpine edge panics on 4.14) | 2026-11-01 | NOTES; `scripts/test-kernel-config` |
| Watchdog | software `/dev/watchdog`; left unfed it restarted the phone in about 45 s; nothing feeds it at boot (issue #17) | 2026-11-01 | NOTES |
| btrfs | built in, mounts with `compress=zstd`; the NAS disk's two partitions, converted with `btrfstune`, mount read-write on the phone and `btrfs scrub` finds no errors (issue #19) | 2026-11-04 | `scripts/test-kernel-config`; `docs/HOMELAB.md` |
| Samba on the tailnet | port 445 answers over `tailscale0` and times out on the LAN; the `cloud` share works from Nautilus on the PC (large copies and a reboot with the disk: #27) | 2026-11-04 | `scripts/test-firewall`; `docs/HOMELAB.md` |
| Front LED as a power indicator | solid with the cable in, a short flash every 3 s on the battery; checked by eye after the fix | 2026-11-04 | `scripts/test-led`; `docs/PITFALLS.md` |
| USB host through a hub | Ethernet, a UAS disk at 24.6 MB/s and PD charging in bypass work (issue #3) | 2026-10-30 | NOTES |
| Not supported | WireGuard (not in 4.14; Tailscale instead), a hardware watchdog (no device node), `CFS_BANDWIDTH` (needs `!SCHED_WALT`), swarm/overlay networks | | NOTES |
| Package update flashes `boot` and `vendor_boot` | boot-deploy's images from the repo's deviceinfo boot; the 2026-10-30 hang was `console=tty0` first in the command line, now `console=ttyMSM0,115200n8` first; a package update flashed both partitions and booted (issue #21, v0.1.13) | 2026-11-02 | `scripts/test-deviceinfo`; NOTES |
| Mainline kernel | parked (2026-11-02, section 5). No boot yet: the ABL only runs header-v3 images, so the early tests were invalid (issue #6) | | [`MAINLINE.md`](MAINLINE.md) |

## 1. Device
| Item | Value |
|---|---|
| Model | Moto G60, codename `hanoip`, XT2135-1 (LATAM, `retbr`) |
| SoC | SM7150 (`qcom,sdmmagpie`, soc_id 365, msm-id `0x16d`). `ro.board.platform=sm6150` is only Motorola's platform name |
| RAM / storage | 4 GB LPDDR4x, 128 GB UFS (Samsung) |
| Bootloader | MBM-3.0, UEFI (`fastboot getvar kernel: uefi`), unlocked (`flashing_unlocked`), verity disabled |
| Partitions | A/B (`boot_a/b`, `vendor_boot_a/b`, `dtbo_a/b`, `vbmeta_a/b`). There is no dedicated `recovery` partition; the recovery lives in the ramdisk. Since 2026-10-23 `adb reboot recovery` enters a LineageOS recovery |
| Android | Originally 12, kernel 4.14.357 (`openela-Cybertron-v18-ksu`, from cool585), KernelSU, root adb through `su`. The installed system turned out to be LineageOS 22.1 (Android 15) with the same kernel |
| Stock boot image | Android header **v3** (kernel ~42 MB, ramdisk ~19.7 MB); cmdline and offsets (`0x8000`/`0x1000000`) come from `vendor_boot` |
| Panel | `tm_ili7807s_fhd_vid`, 1080×2460 |
| Stock serial console | `ttyMSM0`, GPIO44 (TX) / GPIO45 (RX), function `qup12`, 1.8 V, 115200 8N1 (no access to the hardware) |


## 2. Collected material and where it is
- Partition backups (outside the repository, with `SHA256SUMS`): `$HANOIP_BACKUP_DIR/` with `boot_{a,b}`, `vendor_boot_{a,b}`, `dtbo_{a,b}`, `vbmeta_{a,b}`, `persist`, `modemst1/2`, `fsg_{a,b}`, `misc`, `abl_{a,b}`.
- Device data (without identifiers): `docs/device-info/` with `fastboot-getvar.txt`, `android-runtime.txt`, `kernel-android.config`, `running.dtb`, `running.dts`, `SUMMARY.md`.
- Project DTS: `kernel/dts/sm7150-motorola-hanoip.dts`.
- Device package draft (live copy outside the repo): `$PMAPORTS_DIR/device/testing/device-motorola-hanoip/`, with a versioned copy in `pmaports/` of this repo.


## 3. Sources used
- Guide: https://wiki.nura.eco/wiki/SM7150_Mainlining
- Kernel: `sm7150-mainline/linux` (GitHub). Tested: fuyuki's fork, branch `next` (6.6-rc1), tag `v6.13.4` and tag `v7.1_rc3` (the one pmOS packages today).
- pmaports: snapshot of the `sm7150-mainline/pmaports` mirror, later updated to the official `main` of 2026-10-13.
- U-Boot: `sm7150-mainline/u-boot`, branch `sdm845-based` (`qcom_defconfig`).
- Hardware reference: cool585's repository (`android_kernel_motorola_sm6150`, branch `v18`), used to understand the hanoip overlays and as the downstream kernel.
- Wi-Fi firmware package for later: `sm7150-mainline/firmware-qcom-sm7150-ath10k`.


## 4. Downstream path (works)

- **Kernel:** cool585's 4.14 tree (commit `b92cb6cd`) built with clang/lld, packaged as `pmaports/device/downstream/linux-motorola-hanoip` (installs `Image.gz` and the base DTB `qcom/sdmmagpie-hanoi-base.dtb`).
- **Image layout:** `boot.img` header v3 with only the kernel; the pmOS initramfs (LZ4 legacy), cmdline and DTB go in `vendor_boot`; the stock `dtbo` applies the board overlay.
- **Patch needed:** `kernel/downstream-patches/0001-...`, so the loop device re-reads the partition table after `LOOP_SET_BLOCK_SIZE` (UFS has 4096-byte sectors).
- **Rootfs:** a GPT image with 4096-byte sectors on `userdata`, written through the Lineage recovery because Motorola's fastboot refuses `userdata`.
- **Firewall:** nftables with all set backends built in.
- **Boot test sequence:** stock kernel + own initramfs (first signs of life), then the pmOS initramfs, then `fastboot boot` of the full image, then flashing `boot_b`/`vendor_boot_b`/`userdata`. Details and pitfalls in `NOTES.md`.
- **Wi-Fi:** see `userspace/modem/README.md`.


## 5. Kernel plan (2026-11-02)

The phone stays on the downstream 4.14 kernel, and that kernel becomes ours. Nothing of this is done yet.

- **Our own kernel.** Fork the tree we build today (cool585's `android_kernel_motorola_sm6150`, commit `b92cb6cd`) into a
  repository of our own, so the phone no longer depends on cool585's Cybertron releases. `scripts/build-kernel` and the
  `linux-motorola-hanoip` APKBUILD then point at the fork.
- **Our own patches.** `kernel/downstream-patches/0001-0010` become the first commits of the fork; new fixes go there, each
  with its test, as now.
- **Hardened for a server.** Turn off what a headless server does not use and turn on the hardening the 4.14 tree offers
  (candidates to check: stack protector, hardened usercopy, `FORTIFY_SOURCE`, freelist hardening, module signature
  enforcement, `kptr_restrict`/`dmesg_restrict`). Check each option against the running config first and enforce it in
  `scripts/test-kernel-config`.
- **Mainline is parked**, far in the background. [`MAINLINE.md`](MAINLINE.md) stays as the record of what was tried.
