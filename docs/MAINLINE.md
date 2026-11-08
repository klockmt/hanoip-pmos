# The mainline kernel attempt

The mainline kernel on the hanoip. The first attempt (6.6, 6.13.4, 7.1-rc3, booted straight from the ABL) never ran a
kernel, as recorded on 2026-10-24 (sections 1 to 6). **Since 2026-11-07 Linux 7.3 runs on this phone RAM-only**, through
a U-Boot built for it (section 7); it is not a server yet. The phone runs the downstream 4.14 kernel meanwhile; what works
today is in [`STATUS.md`](STATUS.md) and the roadmap is GitHub issue #6. The pitfalls of the pmOS flow itself are in
[`PITFALLS.md`](PITFALLS.md).

**Resumed on 2026-11-07** ([`STATUS.md`](STATUS.md), section 5), after being parked on 2026-11-02. The new start is
Linux 7.3 with this project's patches (`kernel/mainline`), booted through our U-Boot, RAM-only. Sections 1 to 6 and 8 are the record of the first attempt; section 7 is the new start.

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

## 7. 2026-11-07: Linux 7.3 runs on this phone, RAM-only, through U-Boot

Nothing was written to the phone. The images were the first validated set (`artefatos-base-20261031`), checked against its
`base-images.sha256`.

1. `scripts/reboot-bootloader`; the ABL reported `MBM-3.0-hanoip_retail-9522caba9cf-23`, slot `b` active and successful,
   retry counts 6/6.
2. `fastboot boot u-boot-sm7150-motorola-hanoip-cam-tres.img` (header v3, DTB built into U-Boot). Its `bootcmd` is
   `run fastboot`: it boots nothing on its own. About 4 s later its fastboot answered (`product: motorola-hanoip`,
   `version: 0.4`); `oem console` shows it started the watchdog at `17c10000` (U-Boot stops it before the kernel).
3. `fastboot boot` of a header-v2 image: that set's `Image.gz` (7.3.0-sm7150 #64) and DTB, the first cpio of its ramdisk (the
   modules, `.ko.zst`) and our own busybox initramfs instead of its Kali one. That Kali initramfs would mount `userdata`
   read-write as the root, and on this phone `userdata` is the server. Ours locks every block device read-only (and keeps
   locking new ones every second), mounts nothing from the disk, brings up an NCM + ACM gadget with a shell on
   `172.16.42.1:2323` (`nc`, USB only) and reboots itself after 10 minutes unless someone logs in.

What worked:

| Area | Result |
|---|---|
| Boot | the gadget was on the PC about 2 s after the `fastboot boot`; `uname -r` is `7.3.0-sm7150` |
| CPU | 8 cores, cpufreq with `schedutil`, cpuidle (WFI and two sleep states) |
| Thermal, RTC, Type-C | 25 thermal zones at about 37 °C, `rtc-pm8xxx`, `qcom,pm6150-typec` sees the PC as partner |
| Battery (modules `qcom-spmi-adc5`, `nvmem_qcom-spmi-sdam`, `qcom_qg`, `qcom_smbx`) | `qcom_qg`: 81 %, 4.215 V, 28.8 °C, -28 mA, "Not charging"; the charger monitor: online, 5.08 V and 294 mA in, health Good. The `qcom_smbx` driver writes nothing to the PMIC ("configuration preserved"), so on the cable the phone runs from USB and the battery holds, like the 4.14 charge limit's hold |
| UFS (`ufshcd-core`, `ufshcd-pltfrm`, `ufs-qcom`) | 78 partitions, all read-only; `sha256sum` of `boot_b` read on 7.3 equals the backup |
| Back to the server | `reboot -f` went through the ABL to slot `b`; SSH answered about 20 s later |

Not there yet: `CONFIG_QCOM_WDT` is off in that config (no hardware watchdog), UFS, battery, remoteprocs and Wi-Fi are
modules its Kali initramfs loads, and the LCDB regulators switch the panel off after about 30 s (fine for a server).
The kernel received only the boot image's command line, not the `bootargs` of the U-Boot DTB, so `panic=` has to go in
the boot image.

**Our own build, same day.** Linux 7.3 from kernel.org (`linux-7.3.tar.xz`, checked against `sha256sums.asc`; its git
tree is `30fd2ef8`, the tree of tag v7.3) plus the 195 patches (`kernel/mainline/patches`) gives tree `276fbce0`,
the one that booted. `config-7.3-hanoip` plus `QCOM_WDT=y` and `WATCHDOG_SYSFS=y`, built with the host's
`aarch64-linux-gnu-gcc` 16.2 (the first set used 13.3), with no errors. The DTB came out byte-identical to the first set's (`900e11eb`).
Booted the same way with our modules in the initramfs and `panic=5 oops=panic` on the command line:

| Check | Result |
|---|---|
| U-Boot `max-download-size` | `0x07000000` (112 MiB): the upper bound of a RAM-only image |
| Modules | `modprobe ufs_qcom` and the battery modules load from our `modules.dep`; 78 partitions, all read-only; battery 81 %, "Not charging" |
| Hardware watchdog | `qcom_wdt` on `/dev/watchdog0`, timeout 30 s (1 to 32). `echo 1 > /dev/watchdog` armed it with nobody feeding it: the phone reset 31 s later, the ABL booted slot `b` and the server answered SSH 42 s after the reset, with nobody touching the phone |
| Slots | before and after: slot `b` active and successful, retry counts 6/6 (the RAM boots do not use up retries) |

How to repeat it (the RAM-boot U-Boot image is the first set's, `artefatos-base-20261031`; check it against its
`base-images.sha256`, or build one with `scripts/build-mainline-uboot`). `scripts/build-mainline-kernel` was run end to end; `scripts/mainline-ram-image`
builds the same image as the one booted above (same kernel, DTB, init and command line), but its own output has not
been booted yet.

```
scripts/build-mainline-kernel            # Linux 7.3 + kernel/mainline/patches, checked tree, kernel/mainline/server.config
scripts/mainline-ram-image               # $HANOIP_MAINLINE_DIR/out/boot-ram-test.img
scripts/reboot-bootloader
fastboot boot u-boot-sm7150-motorola-hanoip-cam-tres.img
fastboot boot "$HANOIP_MAINLINE_DIR/out/boot-ram-test.img"
sudo nmcli device set <usb interface> managed no        # see docs/PITFALLS.md
sudo ip addr add 172.16.42.2/24 dev <usb interface>
printf 'touch /tmp/keep\n' | socat -t 5 - TCP:172.16.42.1:2323   # stops the 10-minute dead man
# ... back to the server: printf 'reboot -f\n' | socat -t 3 - TCP:172.16.42.1:2323; nmcli device set <usb interface> managed yes
```

## 7b. 2026-11-07: a persistent mainline root in super's free region

The user approved one place for a mainline root that survives reboots: the unallocated region of `super` (`sdb23`)
after `product_a`, where no `system_a` exists (read with `lpdump` from the backup). It is used at offset 1120 MiB,
2400 MiB (AGENTS.md), ext4 labelled `hanoip_ml`. The LP metadata, the Android volumes and every other partition are
never written, and the server on `userdata` is untouched.

- `scripts/mainline-make-root` writes it from the running 4.14 server: a loop device with `--sizelimit`, `mkfs.ext4`
  only if the region holds nothing, then `apk --root` with Alpine edge plus the pmOS repository (144 packages,
  125 MB: OpenRC, eudev, OpenSSH, chrony, unudhcpd, sudo), our 7.3 modules, the server's user, key and SSH host keys,
  and three `local.d` guards: every other disk read-only, a reboot back to the server 15 minutes after boot unless
  `/run/hanoip-ml-keep` exists, and a reboot back to the server under 50 % battery or 3.6 V. It hashes `super` outside
  the region before and after; two runs left those hashes equal to the backup's.
- `INIT=super-root scripts/mainline-ram-image` packs `kernel/mainline/super-root-init` (stage 1): it locks every block
  device read-only except `super` and its disk, feeds the hardware watchdog, opens the region only through
  `bounded-loop` (busybox `losetup` has no size limit), checks the offset, the size limit and the label, mounts it and
  switches to OpenRC. If anything fails it falls back to the RAM test's shell and dead man.

First boot (U-Boot and kernel still through `fastboot boot`): SSH answered 73 s after the `fastboot boot`, with the PC
getting `172.16.42.2` by DHCP and `scripts/hanoip-ssh` working unchanged. `/` is `/dev/loop7`; the only writable
block devices are `loop7`, `sdb` and `sdb23`; `sshd`, `chronyd` and `udev` run; `/dev/watchdog0` is active; the
battery reads "Not charging". Stage 1 waits about 50 s for the UFS LUNs. The first try failed to mount with I/O
errors: the locking loop had run while the LUNs probed and locked `super` and its disk before their names showed up,
so nothing was written; it now starts only once `super` is known (`scripts/test-mainline-ram-image`).

Not there yet: the firmware (ADSP, CDSP, modem, IPA, Wi-Fi, Bluetooth, GPU, video) is not in the root, so no Wi-Fi or
modem; udev loads display, touch, audio and Bluetooth drivers a headless server does not need; the clock is not set
without a network; and the boot is still two `fastboot boot`s from the PC.

## 7c. 2026-11-07: the root in super becomes a server base

Same boot (two `fastboot boot`s from the PC), with `scripts/mainline-make-root` now copying from the server, on the
phone, what a server needs, and `kernel/mainline/root-overlay` holding the rest:

- **Modem and Wi-Fi** (the recipe validated on 2026-10-25): the
  vendor firmware (`adsp`, `modem`, `ipa_fws`, `wlanmdsp.mbn`) where the DT's `firmware-name` asks for it
  (`qcom/sm7150/motorola/hanoip/`), `bdwlan_hanoip_ipa.bin` as the ath10k `board.bin`, and
  `/etc/init.d/hanoip-ml-modem`: it copies the EFS (`modemst1`, `modemst2`, `fsg`, `fsc`) to RAM and serves it with
  `rmtfs -o /run/efs -r` (read-only: those partitions are never written), starts `tqftpserv`, loads
  `hanoip_mmi_unit_info` (SMEM item 134, with the generic values of the 4.14 command line: device, carrier, baseband,
  radio, hardware revision; no serial), `qcom_pd_mapper`, `qcom_q6v5_pas`, `ipa` and `ath10k_snoc`, and starts the
  modem with its recovery off. udev does not load those drivers on its own (`/etc/modprobe.d/hanoip-ml.conf`), nor the
  display, touch, video, camera and Bluetooth ones.
- **Network:** the server's NetworkManager profiles and its Wi-Fi MAC (ath10k picks a random one), the Tailscale state
  (the same node, `100.72.42.100`) and the nftables rules. usb0 stays the server's `172.16.42.1` with DHCP for the PC.
- **Clock:** the PMIC RTC does not keep the time (it reads 1974), so `swclock` replaces `hwclock`, and a NetworkManager
  dispatcher script restarts chrony when a connection comes up.
- **Charge limit:** `hanoip-ml-charge` switches the kernel's opt-in charging (`/sys/kernel/debug/hanoip-charge`, `kernel/mainline/patches`) on
  under 75 % and off at 80 %, never over 4.30 V or 40 °C, and off when it stops (`scripts/test-mainline-charge`).

Checked on the phone, with nothing done by hand after the `fastboot boot`: SSH over USB after 72 s and over the
tailnet after 133 s; ADSP and modem running, `wlan0` connected with the server's MAC, internet and DNS, the clock set
by chrony, Tailscale up; port 22 closed from the LAN and open over the tailnet; the charge limit idle at 78 %; only
`loop7`, `sdb` and `sdb23` writable; 198 MB of RAM used, no errors in `dmesg`, which needs root. The modem started by
hand the first time, before the service did it. With the laptop's USB port (a 500 mA SDP) the input sits at its limit
and the battery drains about 55 mA with charging off, so a real charger matters.

Three things broke on the way and have tests (`scripts/test-mainline-root`): there is no `fsg` link on mainline (only
`fsg_a` and `fsg_b`, identical here), the modem does not start by itself (the ADSP does), and chrony started before the
Wi-Fi had no source, which left the clock in 1974 and Tailscale logged out.

Not there yet: the server's own services (AdGuard Home, Samba, Syncthing, the Telegram bot, `hanoip-status`), the USB
host for the NAS disk, and a boot that does not need the PC.

## 7d. 2026-11-08: U-Boot in `boot_a` boots the mainline root

`boot_a` now holds our U-Boot at its start, built by `scripts/build-mainline-uboot`, and 16 MiB in the kernel
image of `INIT=super-root scripts/mainline-ram-image`; `scripts/mainline-boot-a-image` packs the two. The ABL starts that
U-Boot as it starts any kernel. U-Boot scans the UFS, finds `boot_a` by name (LUN 4, partition 22), reads 32 MiB from
16 MiB in and boots it with the DTB built into U-Boot, as `fastboot boot` does. If that fails it resets, and the ABL
counts a failed boot of slot a. It cannot flash.

- **Watchdog:** it starts with U-Boot, nothing in U-Boot feeds it, and the handoff no longer stops it
  (`kernel/mainline/uboot/0001-leave-the-watchdog-running-for-the-kernel.patch`): the kernel's `qcom_wdt` finds it
  running and pings it until stage 1 opens `/dev/watchdog`. U-Boot programs 30 s; it bit about 50 s after U-Boot
  started. The Volume Down menu stops it first (not tried).
- **Boot deadline:** stage 1 reboots a root that has not written `/run/hanoip-ml-up` (local.d's last step) 5 minutes
  after `switch_root`, and leaves the OpenRC services and the kernel log in `/var/log/hanoip-ml-deadline.log`.
- **Slot a marks itself:** after a complete boot from slot a, `98-hanoip-ml-slot.start` runs `qbootctl -m a`, which
  marks every partition of slot a and writes back the GPTs of LUN 4 and LUN 2 (`xbl_a`), as the 4.14 does for slot b
  at every boot; the guard lends it only those whole disks (approved by the user on 2026-11-08). The slot comes from
  the ABL's command line, which U-Boot leaves in the DT as `u-boot,abl-bootargs`; a boot from the PC says `_b` and
  marks nothing.
- **Toolchain:** a container with the toolchain of the first validated set (Ubuntu 24.04, GCC 13.3) rebuilt its
  `u-boot.bin` and its image byte for byte (the payload is deflate level 6). `scripts/build-mainline-uboot` uses the
  host's GCC 16.2, and that build is the one that booted below.

How it was found, with `fastboot boot` from the ABL and slot b active up to S4 (nothing flashed except `boot_a`):

| Run | What booted | What happened |
|---|---|---|
| first try | the persistent U-Boot | sat at its `=>` prompt for good (the user saw it); power key held |
| D1, D3, D2 | debug builds with fastboot or the serial gadget, watchdog unfed | no gadget; reset by the watchdog |
| T3 | the RAM-boot base config, only the watchdog unfed | fastboot at 3.2 s. U-Boot's console (`fastboot oem console`): `stop autoboot:  0 ^H^H^H 0`, a key already pending, then `Timeout waiting for command`, which its `bootretry=1` recovers from and our `-1` did not |
| D2 again | `bootdelay=-2`, `scsi dev` before `scsi read` | UFS at gear 3 with 6 LUNs, `boot_a` = LUN 4 partition 22, 8192 blocks read, CRC `acab4da4` equal to the image, then the root up to SSH |
| S1 | the server build, U-Boot never touching USB | kernel and USB gadget up in 4 s, then the root stalled before `local.d` (no `usb0`, no Wi-Fi) for 10 minutes; power key held |
| S2, S3 | the same with the boot deadline (S3: the root fixed as below) | SSH over USB in 84 s, then the tailnet; chrony in sync; S3 with no log flood and its whole boot in `dmesg` |
| S4 | the same after an unclean reboot out of mainline, no `e2fsck` | SSH over USB in 93 s; the kernel replayed the journal |
| S5 | slot a itself: `fastboot set_active a`, `fastboot reboot`, no PC after that | USB gadget at 20 s, SSH over USB at 113 s, the tailnet, chrony in sync; back in the ABL `current-slot` was `a` and slot a's retry count had gone from 7 to 6. Then `set_active b`: the 4.14 server came back with every service, and `qbootctl -m` marked b successful again (`set_active` clears it) |
| S6 | RAM boot with slot b active, the slot marking in the root | the ABL's slot read as `_b`, nothing marked, the dead man on; `qbootctl` with no arguments tried to write the GPT and the guard stopped it |
| S7 | slot a with the slot marking | SSH at 113 s, no dead man. `boot_a`'s attributes went from `0x32` (retry 6) to `0x77` (active, retry 6, successful) and the ABL then said `slot-successful:_a: yes`; read back on both sides, the GPTs of LUN 4 and LUN 2 kept valid CRCs and backups with only A/B bits changed, and `boot`, `vendor_boot` and `dtbo` of both slots kept their hashes. Back on slot b, both slots are successful |

Fixed on the way, each with a test (`docs/PITFALLS.md`): the pending key at autoboot, `scsi read` on LUN 0, a watchdog
that U-Boot fed and then stopped, a root stall nothing reset, `mainline-make-root` failing on an unclean root (the 4.14
kernel cannot mount an ext4 whose orphan file holds inodes), and chrony's `rtcsync`, which made the kernel retry the RTC
write about 30 times a second.

Still open:
- The mainline root is not the server yet: AdGuard Home, Samba, Syncthing, the Telegram bot and `hanoip-status` run
  only on the 4.14, so slot b is active again.
- The cause of the S1 stall. It did not happen in S2 to S4; the deadline now reboots it and leaves a log.
- LUN 0 (16 MB) answers slowly under mainline: its partitions show up 27 s into the boot, and stage 1 finds super at
  33 to 47 s.
- `tailscale ping` gets no reply from this node; SSH over the tailnet works.

## 7e. 2026-11-08: what Duranium needs on this phone (research only)

Duranium is Nura's image-based variant ([blog](https://nura.eco/blog/2026/03/17/introducing-duranium/),
[wiki](https://wiki.nura.eco/wiki/Duranium_(Immutable_Nura)),
[DESIGN.md](https://gitlab.nura.eco/nura/duranium-build/-/blob/main/DESIGN.md); the wiki and GitLab sit behind a
bot check, so they were read through a browser). In short:

- **Build:** mkosi, through `build-image.py device-<name> ui-<ui> --release=edge` in `duranium-build`. It resolves the
  device package, its kernel subpackage and its `deviceinfo` from pmaports, so a Duranium for this phone needs a
  pmaports device package with a mainline kernel. Ours is the downstream 4.14 one. Everything is systemd, the initramfs
  too.
- **Boot:** EFI. On phones U-Boot gives it, and systemd-boot on the ESP starts a UKI holding kernel, initramfs, command
  line and DTBs; systemd-stub picks the DTB whose `compatible` matches what U-Boot passes. A new image that fails twice
  falls back to the previous one. Our U-Boot already has the EFI loader, `bootefi`, the EFI boot manager, `blkmap` and
  FAT.
- **Disk:** the image ships an ESP and `/usr` slot A with its dm-verity partition; on first boot systemd-repart adds
  slot B and its verity partition and the root on LUKS2, which fills the rest. Each `/usr` slot is 5 GB (EROFS), and
  16 GB is the recommended minimum. On a phone the image is flashed into `userdata`, which U-Boot's `blkmap` presents
  as a disk (`fastboot flash userdata *.raw`). That erases what is there.
- **Encryption:** LUKS2 is always on, but repart creates it with a blank passphrase and it unlocks by itself; the
  first-boot wizard (f0rmz) offers to set one. It also creates the user on the screen, so a headless image needs that
  step done another way.
- **Images:** built ones exist only for fairphone-fp5 and fp6, google-sargo, oneplus-enchilada and fajita,
  lenovo-21bx, google-x64cros and the generic x86_64 and loongarch64 ([duranium.nura.eco](https://duranium.nura.eco/)).
  Nothing for SM7150.

On this phone `userdata` is 116 GB and holds the 4.14 server, whose root uses 4.2 GB of its 104 GB; super's free region
is 2.4 GB. So the choice is where Duranium lives: super's region for a first boot with smaller `/usr` slots (it would
replace the Alpine development root, which `mainline-make-root` writes again at will), a region cut from the end of
`userdata` after shrinking the server's root, or all of `userdata` in place of the 4.14. The device package, the
kernel package and systemd units for the phone's own pieces (the modem start, the charge limit, the guard) are needed
in every case; systemd's own watchdog support would replace the stage-1 daemon.

## 8. Commands used to reproduce (first attempt)
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
