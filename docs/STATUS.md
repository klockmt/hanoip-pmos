# Moto G60 (hanoip) on postmarketOS: context, attempts and errors

State on 2026-10-24. The goal is to run postmarketOS on the Moto G60 (hanoip) as a headless server, and to publish everything on GitHub with reproducible instructions and a way to update the kernel.

**Result so far:**

- **Downstream kernel (4.14, cool585 Cybertron): working.** postmarketOS edge boots on its own from `boot_b` + `vendor_boot_b` with a 104 GB root on `userdata`, SSH over the USB network gadget and an nftables firewall. See the "Downstream path" section and `NOTES.md`.
- **Wi-Fi: not working yet.** The modem comes up and the Wi-Fi firmware connects to the kernel (`icnss`), then crashes about 0.7 s later. See `userspace/modem/README.md`.
- **Mainline kernel: no boot yet.** The DTS builds and the kernel and the pmOS package are generated, but no temporary boot reached the pmOS USB network. The cause is the ABL only running header-v3 images (found on 24/10), so every v0 test was invalid; the v3 `vendor_boot` with our DTB is still to be tested on slot `b`.

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

## 4. What was done (mainline attempt)

1. **Backup** of the critical partitions and extraction of the running DTB (`/sys/firmware/fdt`) and of the Android kernel config.
2. **hanoip DTS.** fuyuki's fork stub did not compile (a nonexistent `tz_memmpss_mem` label and a syntax error). It was rewritten in `kernel/dts/sm7150-motorola-hanoip.dts`: identity and `board-id` from the stock DTB, peripherals adapted from `sweet` (UFS, USB, WCN3990 Wi-Fi, SD, regulators), reserved memory overriding only what differs from upstream (`tz_mem`, `cont_splash`, ramoops, wdog, tzlog, mmi), 1080×2460 simplefb with `clocks`, `gpio-reserved-ranges = <59 4>` and `dr_mode = "peripheral"`. It builds without warnings of our own on the 6.6, 6.13.4 and 7.1_rc3 trees.
3. **Kernels.** 6.6-rc1, 6.13.4 and 7.1_rc3 were built (the latter with the current pmOS config, `EFI_ZBOOT=y`, modules and `vmlinuz.efi`).
4. **pmbootstrap** (3.12.1, in a venv): initialized, local pmaports turned into a git repository with `origin/main`, `device-motorola-hanoip` package created from `device-xiaomi-surya`, kernel packaged with `pmbootstrap build --envkernel` and rootfs generated with `pmbootstrap install`.
5. **U-Boot** from the `sm7150-mainline` fork built with the hanoip DTB and `bootcmd = fastboot usb 0`.

## 5. Test matrix (mainline; all with `fastboot boot`, nothing flashed to `boot`/`vendor_boot`)

| # | Payload | Header | `dtbo` | Result |
|---|---|---|---|---|
| 1 | Kernel 6.6-rc1 + test ramdisk (busybox, telnet on 172.16.42.1) | v3 | original (`b`) | Motorola logo, device vanishes from USB and reboots |
| 2 | U-Boot (v3, `bootcmd` without fastboot) | v3 | original | Disconnects, no gadget |
| 3 | U-Boot with `bootcmd = fastboot usb 0` | v3 | original | No `18d1:d00d` gadget |
| 4 | **Control:** original `boot_b.img` | v3 | original | Android booted, slowed down and rebooted by itself; stable afterwards |
| 5 | Kernel 6.6 (DTS with `peripheral`) | v3 | `dtbo_a` **zeroed** (slot `a`) | `Status read failed`, no USB; slot `a` marked `unbootable` |
| 6 | Kernel 6.13.4 | v0 + appended DTB | original | Logo, then back to fastboot |
| 7 | Official pmOS package (labeled 7.1, 6.13.4 code) | v0 | original | Screen flashed and went back to the bootloader |
| 8 | Kernel 7.1_rc3 (official package) | v0 | original | Flashes and goes back to the bootloader |
| 9 | Kernel 7.1_rc3 | v3 (`bootimg_analyze`) | original | Bootlogo and vanishes from USB in < 1 min |
| 10 | Kernel 7.1_rc3, `console=tty0` | v3 | `dtbo_b` **zeroed** | Back to the bootloader |
| 11 | Kernel 7.1_rc3, `console=tty0` | v0 + appended DTB | `dtbo_b` **zeroed** | No boot |
| 12 | U-Boot (`fastboot usb 0`) again | v0 | original | No gadget |

The pmOS `172.16.42.x` USB interface or the U-Boot gadget never appeared in any test.

**Finding of 24/10:** the ABL only runs images with header **v3**. A kernel that calls `PSCI SYSTEM_OFF` on its first instruction reboots in v0 (it does not run) and powers off in v3 (it runs). The v0 tests never executed our kernel. Next step: v3 `vendor_boot` with our DTB.

**Correction of 23/10:** tests 8 to 11 are invalid. The pmOS config for 7.1 enables `CONFIG_EFI_ZBOOT`, and the APKBUILD packed `vmlinuz.efi` (EFI zboot, `zimg` signature) into `boot.img`. The ABL only loads `Image`/`Image.gz`. The combination **header v0 + real `Image.gz` + zeroed `dtbo`** has not been tested. Reading pstore through the LineageOS recovery (test with the zboot kernel) showed only the old 4.14 kernel log, so our kernel never reached ramoops.

## 6. Downstream path (works)

- **Kernel:** cool585's 4.14 tree (commit `b92cb6cd`) built with clang/lld, packaged as `pmaports/device/downstream/linux-motorola-hanoip` (installs `Image.gz` and the base DTB `qcom/sdmmagpie-hanoi-base.dtb`).
- **Image layout:** `boot.img` header v3 with only the kernel; the pmOS initramfs (LZ4 legacy), cmdline and DTB go in `vendor_boot`; the stock `dtbo` applies the board overlay.
- **Patch needed:** `kernel/downstream-patches/0001-...`, so the loop device re-reads the partition table after `LOOP_SET_BLOCK_SIZE` (UFS has 4096-byte sectors).
- **Rootfs:** a GPT image with 4096-byte sectors on `userdata`, written through the Lineage recovery because Motorola's fastboot refuses `userdata`.
- **Firewall:** nftables with all set backends built in.
- **Boot test sequence:** stock kernel + own initramfs (first signs of life), then the pmOS initramfs, then `fastboot boot` of the full image, then flashing `boot_b`/`vendor_boot_b`/`userdata`. Details and pitfalls in `NOTES.md`.
- **Wi-Fi:** see `userspace/modem/README.md`.

## 7. Attempts to get a kernel log

- **pstore through Android:** the stock Android consumes the records when it boots. Only a `dmesg` of the stock kernel itself showed up (panic in `SMP: failed to stop secondary CPUs`) and a `dmesg-ramoops-0.enc.z` encrypted by Motorola. No marker from our kernel (`6.13`, `7.1`, `hanoip`, `sm7150`).
- **pstore through the recovery (18/10):** there is no dedicated `recovery` partition. `fastboot reboot recovery` caused a bootloop and a `twrp` image supplied by the user did not boot through `fastboot boot`. **Update of 23/10:** `adb reboot recovery` now enters a LineageOS recovery; reading pstore through it works (adb and `CONFIG_PSTORE_*` are confirmed in `NOTES.md`).
- **Raw RAM:** the stock kernel has no `/dev/mem`, `/dev/kmem` or `/proc/kcore`, and `/proc/iomem` is denied.
- **Screen:** only the unlocked-bootloader warning and the bootlogo show; nothing from the kernel.
- **UART:** only possible with hardware, on GPIO44/45 at 1.8 V (see the table above). No access to the opened device.

## 8. Errors found and how they were solved

| Problem | Cause | Solution |
|---|---|---|
| `git clone` from GitHub failed with `SSL_read unexpected eof` | A git transport problem on this host (`curl` works) | Download tarballs with `curl` from `codeload.github.com` |
| fuyuki's DTS did not compile | Nonexistent `tz_memmpss_mem` label and an unclosed brace | DTS rewritten |
| `delete-node &reserved_memory` broke the build | `sm7150.dtsi` references `smem_mem`, `adsp_mem`, etc. | Keep upstream's `reserved_memory` and override only what differs |
| `mkbootimg` v2 required a DTB section | A DTB appended to the kernel does not count as a section | Use header v0 like surya (`append_dtb`) |
| `pmbootstrap init` failed with `EOF` | Run through `!`, without interactive input | Run in a normal terminal |
| pmaports without `.git` / `origin/main` | It came as a tarball | `git init -b master`, official remote and `refs/remotes/origin/main` |
| `Strict mode ... abuild needs building` | Local pmaports older than the binary repo | Build `abuild` manually, then update pmaports to the official `main` |
| Chroots with channel `edge` vs `systemd-edge` | Channel change when updating pmaports | `pmbootstrap zap` |
| `soc-qcom-sm7150 (no such package)` | Package renamed in current pmaports | Dependencies rewritten |
| Conflict in `etc/machine-info` / `deviceinfo` | `device-qcom-sm7150` is a generic device package that installs the same files | Do not depend on it; use `firmware-qcom-sm7150`, `soc-qcom`, `mkbootimg-osm0sis` |
| `deviceinfo is missing in checksums` | Empty checksums | `pmbootstrap checksum` |
| `Image.gz` not found when packaging | envkernel only packages; its `mrproper` cleans the tree | Build outside envkernel with `O=.output`, then package |
| The 7.1 package asked for `vmlinuz.efi` and `modules.order` | The current pmOS config enables `EFI_ZBOOT` | Build `vmlinuz.efi dtbs modules` (later: turn zboot off for the ABL) |
| `boot-deploy` required `android-tools` | Dependency missing in the device package | Add `android-tools` to the APKBUILD |
| `sudo umount ... exit 32` on shutdown/build | Mount point already unmounted | Repeat `shutdown` or `zap` |
| Slot `a` became `unbootable` | Consecutive failed boots on slot `a` | Go back to slot `b`; the flag was cleared later |
| Bootloop when asking for the recovery (18/10) | No working recovery at the time (cause unproven) | Restore `dtbo_b` and `fastboot continue`; on 23/10 a LineageOS recovery started working |

## 9. Unconfirmed hypotheses (mainline)

- A very early hang in boot (before pstore, USB or framebuffer), possibly due to the watchdog, reserved memory, or the DTB that the ABL hands to the kernel.
- The ABL may choose the DTB by `qcom,board-id`/`msm-id` and not accept ours, although they equal the stock DTB.
- Differences against the devices that do boot (surya, sweet, davinci, sunfish) have not been compared point by point.

## 10. Suggested next steps

1. Wi-Fi on the downstream kernel: see `userspace/modem/README.md` for what was ruled out. Ask in `#sm7150-mainline` (OFTC / Matrix) with the `icnss` log.
2. Mainline: v3 `vendor_boot` with our DTB, tested on slot `b` (the slot `a` tests are invalid). Compare `deviceinfo`, `board-id` and bring-up commits of the SM7150 devices that already boot.
3. If physical access is ever possible: UART on GPIO44/45 (1.8 V) to see the kernel and U-Boot logs.
4. Read pstore through the LineageOS recovery after a failed mainline boot (the guide's way), going straight into it and not through Android.

## 11. Commands used to reproduce

```
# Running DTB
adb exec-out su -c 'cat /sys/firmware/fdt' > running.dtb
dtc -f -I dtb -O dts -o running.dts running.dtb

# Kernel 7.1_rc3 (out of tree) and the pmOS package
make O=.output ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- olddefconfig
make O=.output ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- -j16 Image.gz vmlinuz.efi dtbs modules
pmbootstrap build --envkernel linux-postmarketos-qcom-sm7150
pmbootstrap checksum device-motorola-hanoip
pmbootstrap build --force device-motorola-hanoip
pmbootstrap install
pmbootstrap flasher boot            # with the device in the bootloader

# Restore dtbo
fastboot flash dtbo_b $HANOIP_BACKUP_DIR/dtbo_b.img
```
