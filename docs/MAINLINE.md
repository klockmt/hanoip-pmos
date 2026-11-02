# The mainline kernel attempt

The history of trying to boot a mainline kernel (6.6, 6.13.4, 7.1-rc3) on the hanoip, kept as it was recorded on
2026-10-24. **It does not boot yet**: the bootloader (ABL) only runs header-v3 boot images, which made the early tests
invalid (see the correction under the matrix). The phone runs the downstream 4.14 kernel meanwhile; what works today is in
[`STATUS.md`](STATUS.md) and the roadmap is GitHub issue #6. The pitfalls of the pmOS flow itself are in
[`PITFALLS.md`](PITFALLS.md).

**Parked on 2026-11-02.** The plan now is our own hardened 4.14 kernel, forked from the downstream one
([`STATUS.md`](STATUS.md), section 5). This file is kept as the record of the attempt.

## 1. What was done
1. **Backup** of the critical partitions and extraction of the running DTB (`/sys/firmware/fdt`) and of the Android kernel config.
2. **hanoip DTS.** fuyuki's fork stub did not compile (a nonexistent `tz_memmpss_mem` label and a syntax error). It was rewritten in `kernel/dts/sm7150-motorola-hanoip.dts`: identity and `board-id` from the stock DTB, peripherals adapted from `sweet` (UFS, USB, WCN3990 Wi-Fi, SD, regulators), reserved memory overriding only what differs from upstream (`tz_mem`, `cont_splash`, ramoops, wdog, tzlog, mmi), 1080×2460 simplefb with `clocks`, `gpio-reserved-ranges = <59 4>` and `dr_mode = "peripheral"`. It builds without warnings of our own on the 6.6, 6.13.4 and 7.1_rc3 trees.
3. **Kernels.** 6.6-rc1, 6.13.4 and 7.1_rc3 were built (the latter with the current pmOS config, `EFI_ZBOOT=y`, modules and `vmlinuz.efi`).
4. **pmbootstrap** (3.12.1, in a venv): initialized, local pmaports turned into a git repository with `origin/main`, `device-motorola-hanoip` package created from `device-xiaomi-surya`, kernel packaged with `pmbootstrap build --envkernel` and rootfs generated with `pmbootstrap install`.
5. **U-Boot** from the `sm7150-mainline` fork built with the hanoip DTB and `bootcmd = fastboot usb 0`.


## 2. Test matrix
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


## 3. Attempts to get a kernel log
- **pstore through Android:** the stock Android consumes the records when it boots. Only a `dmesg` of the stock kernel itself showed up (panic in `SMP: failed to stop secondary CPUs`) and a `dmesg-ramoops-0.enc.z` encrypted by Motorola. No marker from our kernel (`6.13`, `7.1`, `hanoip`, `sm7150`).
- **pstore through the recovery (18/10):** there is no dedicated `recovery` partition. `fastboot reboot recovery` caused a bootloop and a `twrp` image supplied by the user did not boot through `fastboot boot`. **Update of 23/10:** `adb reboot recovery` now enters a LineageOS recovery; reading pstore through it works (adb and `CONFIG_PSTORE_*` are confirmed in `NOTES.md`).
- **Raw RAM:** the stock kernel has no `/dev/mem`, `/dev/kmem` or `/proc/kcore`, and `/proc/iomem` is denied.
- **Screen:** only the unlocked-bootloader warning and the bootlogo show; nothing from the kernel.
- **UART:** only possible with hardware, on GPIO44/45 at 1.8 V (see the device table in `STATUS.md`, section 1). No access to the opened device.


## 4. Errors of this attempt and how they were solved

The errors of the pmOS flow that were not specific to the mainline attempt are in [`PITFALLS.md`](PITFALLS.md).

| Problem | Cause | Solution |
|---|---|---|
| fuyuki's DTS did not compile | Nonexistent `tz_memmpss_mem` label and an unclosed brace | DTS rewritten |
| `delete-node &reserved_memory` broke the build | `sm7150.dtsi` references `smem_mem`, `adsp_mem`, etc. | Keep upstream's `reserved_memory` and override only what differs |
| `mkbootimg` v2 required a DTB section | A DTB appended to the kernel does not count as a section | Superseded: this ABL only runs header v3, so the v0 tests never ran the kernel (see below) |
| `pmbootstrap init` failed with `EOF` | Run through `!`, without interactive input | Run in a normal terminal |
| pmaports without `.git` / `origin/main` | It came as a tarball | `git init -b master`, official remote and `refs/remotes/origin/main` |
| `Strict mode ... abuild needs building` | Local pmaports older than the binary repo | Build `abuild` manually, then update pmaports to the official `main` |
| Chroots with channel `edge` vs `systemd-edge` | Channel change when updating pmaports | `pmbootstrap zap` |
| `soc-qcom-sm7150 (no such package)` | Package renamed in current pmaports | Dependencies rewritten |
| `deviceinfo is missing in checksums` | Empty checksums | `pmbootstrap checksum` |
| The 7.1 package asked for `vmlinuz.efi` and `modules.order` | The current pmOS config enables `EFI_ZBOOT` | Build `vmlinuz.efi dtbs modules` (later: turn zboot off for the ABL) |
| Slot `a` became `unbootable` | Consecutive failed boots on slot `a` | Go back to slot `b`; the flag was cleared later |
| Bootloop when asking for the recovery (18/10) | No working recovery at the time (cause unproven) | Restore `dtbo_b` and `fastboot continue`; on 23/10 a LineageOS recovery started working |

## 5. Unconfirmed hypotheses
- A very early hang in boot (before pstore, USB or framebuffer), possibly due to the watchdog, reserved memory, or the DTB that the ABL hands to the kernel.
- The ABL may choose the DTB by `qcom,board-id`/`msm-id` and not accept ours, although they equal the stock DTB.
- Differences against the devices that do boot (surya, sweet, davinci, sunfish) have not been compared point by point.


## 6. Next steps (mainline)

2. Mainline: v3 `vendor_boot` with our DTB, tested on slot `b` (the slot `a` tests are invalid). Compare `deviceinfo`, `board-id` and bring-up commits of the SM7150 devices that already boot.
3. If physical access is ever possible: UART on GPIO44/45 (1.8 V) to see the kernel and U-Boot logs.
4. Read pstore through the LineageOS recovery after a failed mainline boot (the guide's way), going straight into it and not through Android.

## 7. Commands used to reproduce
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
