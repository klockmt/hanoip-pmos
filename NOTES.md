# hanoip-mainline

postmarketOS port for the **Moto G60 (hanoip, XT2135-1)**: a working downstream-kernel (4.14) build and a mainline-kernel effort. Reply to the user in Portuguese.

## Device

- SoC SM7150 (`qcom,sdmmagpie`, soc_id 365). `ro.board.platform=sm6150` is Motorola's platform name, not the SoC.
- 4 GB RAM, 128 GB UFS, A/B, unlocked bootloader, boot image header v3, Android 12 with KernelSU.
- Verified facts: `docs/device-info/SUMMARY.md`. Read it before touching device info.
- **Never** put an IMEI, serial number or MAC address in any repo file (this one included). The repo is public.
- Partition backups (boot, vendor_boot, dtbo, vbmeta, persist, modemst, fsg, misc, abl + SHA256SUMS): `$HANOIP_BACKUP_DIR/`. They stay out of the repo on purpose.
- Collection of 2026-10-23 (`scripts/coletar-g60.sh`, already fixed for A/B): `$HANOIP_COLLECTION_DIR/` with identity, kernel (config, dmesg, iomem), slot `b` partitions, `/vendor` firmware (102 MB, proprietary) and the device tree. It contains serial/MAC/IMEI and firmware: **never copy it into the repo**. `.gitignore` covers `*.img` and the collection folders.
- `docs/device-info/android-runtime.txt`, `running.dts` and `running.dtb` are **sanitized** (MACs became `00:00:00:00:00:00`, the serial became `X`); sanitize again before committing if you regenerate them.

## Files

- `kernel/dts/sm7150-motorola-hanoip.dts`: our own DTS (mainline).
- `docs/STATUS.md`: project state, test matrix, commands to reproduce.
- `docs/device-info/`: device outputs (getvar, running DTB, Android kernel config).
- `kernel/downstream-patches/`: patches for the cool585 4.14 kernel (loop, `clk.c` for `DEBUG_FS`, `icnss` with BDF/REGDB/`host_cap`).
- `pmaports/`: versioned copy of the `device-motorola-hanoip` and `linux-motorola-hanoip` packages (the live copy is in `$PMAPORTS_DIR`).
- `userspace/modem/`: `qrtr-ns`, `pd-mapper` and `tqftpserv` for the 4.14 kernel, plus the modem/Wi-Fi openrc and udev files.
- `scripts/`: the replication steps documented in `README.md` (`check-host`, `backup-partitions`, `build-kernel`, `setup-pmaports`, `pmb`, `configure-pmbootstrap`, `build-image`, `flash-rootfs`, `flash-boot`, `setup-usb-net`, `hanoip-ssh`, `reboot-bootloader`, `mkbootimg-kernel`), `check-repo` (**run it before every commit and push**), `flash-boot-ssh` (writes boot over the LAN when the phone is on the Ethernet hub and has no USB link to the PC), `setup-console`, `update-apkbuild-sums` (recompute the APKBUILD checksums), the Wi-Fi investigation helpers (`push-firmware`, `mount-vendor`) and `coletar-g60.sh` (runs in Termux on Android). They read `.env`; the ones that change something accept `DRY_RUN=1`. `README.md` says exactly which were run end to end and which only in dry-run, so keep it honest when you change them.
- **Local paths and personal data live in `.env`** (not in git; template in `.env.example`). In docs and scripts use the variables (`$HANOIP_BACKUP_DIR`, `$HANOIP_COLLECTION_DIR`, `$PMAPORTS_DIR`, `$KERNEL_DOWNSTREAM_DIR`, ...), never the real path or the user name. To resolve a variable, read `.env`.
- Commits: small and per topic, in English (`type(scope): summary` + a body explaining why).

## pmOS tools (outside the repo)

- `pmbootstrap` 3.12.1 lives in a venv and is **not on PATH**: `$PMBOOTSTRAP`. Already initialized (`pmbootstrap init`).
- pmaports: `$PMAPORTS_DIR` (local git repo, `origin/main` = official pmaports of 2026-10-13). Device package: `device/testing/device-motorola-hanoip/` (versioned copy in `pmaports/` of this repo).
- Kernel: `$KERNEL_MAINLINE_DIR` (tag `v7.1_rc3` of `sm7150-mainline/linux`, the one pmOS packages). Build in `.output/`. `.../src/linux-6.13.4` and the fuyuki 6.6-rc1 fork in `$HANOIP_SRC_DIR/fuyuki/` also exist.
- U-Boot: `$UBOOT_DIR`.
- `pmbootstrap` asks for the `sudo` password and needs interactive input (not even `!` works). **The user runs it**; hand over ready-made commands and wait for the output.

## pmOS downstream working (2026-10-24)

- **postmarketOS edge boots on its own with SSH** on the downstream kernel (cool585 Cybertron v18, 4.14) with openrc. Root is expanded to 104 GB on `userdata`. Access: `scripts/hanoip-ssh` (uses `HANOIP_USER`, `HANOIP_HOST` and `HANOIP_SSH_KEY` from `.env`; dedicated key without a passphrase, since the user's own key has one and cannot be used non-interactively). `sudo` asks for a password.
- Packages in the local pmaports: `device/downstream/linux-motorola-hanoip` (cool585 source at commit `b92cb6cd`, config from `/proc/config.gz` + `DEVTMPFS`, `DEVTMPFS_MOUNT`, `USB_CONFIGFS_ECM/RNDIS`, `SYSVIPC`, `FHANDLE`; installs `Image.gz` and the base DTB `qcom/sdmmagpie-hanoi-base.dtb`) and `device/testing/device-motorola-hanoip` (`kernel-downstream` subpackage). `pmbootstrap config kernel downstream`, `service_manager openrc`, `ssh_keys True`.
- `deviceinfo`: `header_version 3`, `initfs_compression lz4`, `dtb qcom/sdmmagpie-hanoi-base` (goes in `vendor_boot`; the stock `dtbo` applies the board overlay), `bootimg_custom_args` with base `0`, offsets `0x8000`/`0x1000000`/`0x100`, `dtb_offset 0x1f00000`, `os_version 15.0.0`, `os_patch_level 2025-04`; cmdline with the non-Android options of the stock `vendor_boot`; `rootfs_image_sector_size 4096`.
- With v3, pmOS puts the **initramfs, cmdline and DTB in `vendor_boot`** and only the kernel in `boot.img`. Flash both.
- **Mandatory patch for the 4.14 kernel:** `0001-loop-rescan-partitions-after-block-size-change.patch` (re-read the partition table on `LOOP_SET_BLOCK_SIZE`). Without it, `losetup -P --sector-size 4096` does not create `loopNpX`. With a 512-byte GPT the initramfs `fdisk` cannot find the table (UFS has 4096-byte sectors). Together these two problems prevented mounting the subvolumes.
- Kernel build: tree `$KERNEL_DOWNSTREAM_DIR`, `make O=.output ... DTC=/usr/bin/dtc` (the tree's own `dtc` built with clang 23 segfaults), then `pmbootstrap build --envkernel linux-motorola-hanoip` (without activating envkernel).
- **Motorola's fastboot refuses to flash `userdata`** (`flash permission denied`). Flash the rootfs through the Lineage recovery: restore the stock `vendor_boot_b`, `fastboot reboot recovery`, Enable ADB, `adb exec-in "dd of=/dev/block/by-name/userdata bs=4M" < motorola-hanoip.img`, and verify the hash by reading it back. Writing over the initramfs USB network (`nc`) froze the network midway; do not use it.
- **Firewall solved (24/10):** nftables built into the kernel (`NF_TABLES`, `NF_TABLES_INET/IPV4/IPV6`, `NFT_CT`, `NFT_COUNTER`, `NFT_NAT`, `NFT_MASQ`, `NFT_REJECT*`, `NFT_LIMIT`, `NFT_LOG`, `NFT_CHAIN_NAT_IPV4`) and **the set backends** `NFT_SET_HASH`, `NFT_SET_RBTREE`, `NFT_SET_BITMAP` (without them rules with `{ ... }` fail with "Not supported"). No modules: the kernel package does not install modules, so everything is `=y`. Config in `linux-motorola-hanoip/config-motorola-hanoip.aarch64` (`pkgrel=2`). The `51_usb_inet.nft` rule allows everything coming in over USB.
- Updating only the kernel: `scripts/mkbootimg-kernel` (the pmOS v3 `boot.img` is just the kernel), test with `fastboot boot`, then `fastboot flash boot_b`.
- Pending: `zram` is missing; `systemd-tmpfiles`/`sysusers` fail on 4.14 (`openat2`).
- **Large transfers over the USB (NCM) network bring the device network down** (ARP `INCOMPLETE`, only a reboot recovers it). Limit with `pv -L 1m` (69 MB went through that way); `scripts/push-firmware` does this.
- Rebooting to the bootloader without buttons: `scripts/reboot-bootloader` (the `reboot(RESTART2, "bootloader")` syscall).
- Without `FW_LOADER_USER_HELPER_FALLBACK`, boot-to-SSH drops from ~3 min to seconds (every missing firmware file cost 60 s).

## Modem and Wi-Fi (in progress, 2026-10-24)

Details, files and build in `userspace/modem/README.md`. Summary:
- Kernel with `QCA_CLD_WLAN=y` and `FW_LOADER_USER_HELPER_FALLBACK` off: the package `linux-motorola-hanoip` is now `pkgrel=3` with exactly the config and patches `0001`-`0003` of test kernel #10 and correct `sha512sums` (the old checksum was stale), but it was never built by pmbootstrap itself, only through `scripts/build-kernel` + `--envkernel`; tested only with `fastboot boot` (`$HANOIP_TEST_DIR/pmos-wlan/boot-wlan.img`). `boot_b` still holds build #4, without WLAN.
- Already on the device: firmware in `/lib/firmware`; Alpine's `rmtfs` (read-only, `-r`); `qrtr-ns` v1.0, `pd-mapper` and `tqftpserv` with patches in `/usr/local/bin`; udev and openrc files from `userspace/modem/etc`; `/var/lib/tqftpserv` copied from `persist` (ro,noload). Alpine's `tqftpserv` was removed from the `boot` runlevel.
- Requirements found: a userspace QRTR name server; remoteproc fallback to `/lib/firmware`; `fsg` → `fsg_b` link (without it, `rmts_get_buffer api failed`); IPA up (`echo 1 > /dev/ipa`) and ready (`rmnet_ipa0`) **before** the modem; `restart_level` set to `RELATED`.
- Result: modem `ONLINE`, `wlan_pd` comes up and WLFW connects, but the Wi-Fi firmware crashes 0.7 s later (`early crash indication`, `EX:kernel:0x0:tmr_slave1:0x1d1`), in a loop, before `FW_READY` and without ever asking for `bdwlan`. DT, IPA order and `/readwrite` are already ruled out. `hanoip-modem` is **outside** the default runlevel until this is solved. Every test needs a clean boot (the modem does not restart well within the same session).
- Test kernel #6 (`$HANOIP_TEST_DIR/pmos-wlan/boot-wlan-debugfs.img`) enables `DEBUG_FS` (needs patch `kernel/downstream-patches/0002-...`; previous config in `.output/.config.pre-debugfs`). It showed that the `icnss` QMI handshake finishes without errors (`board_id 0xff`, fw `WLAN.HL.3.2.2-00947`) and the firmware crashes by itself about 650 ms later. Details in the modem README.
- 2026-10-24 (afternoon): `icnss` now sends `host_cap`, REGDB and BDF like `cnss-daemon` (patch `0003`, test kernel #10 `$HANOIP_TEST_DIR/pmos-wlan/boot-wlan-regdb.img`). The firmware accepts everything and **still crashes**. Details and discarded hypotheses in the modem README. Patches `0002` and `0003` are in `kernel/downstream-patches/` and in the pmaports kernel package (the copies in `pmaports/device/downstream/linux-motorola-hanoip/` must stay identical to the ones in `kernel/downstream-patches/`; if you change one, change both and recompute `sha512sums`).
- Never use `pkill -f <text>` inside `ssh ... 'sudo sh -c "..."'`: it matches the command line itself and kills the session (it happened twice). Keep the PID with `$!`.

## Screen console and boot log (2026-10-24)

The screen used to stay on the Motorola logo for the whole boot. What was found and done:
- The **ABL rewrites every `console=` on the command line to `console=null`** (checked with `console=ttyMSM0` in vendor_boot and `console=tty0` in the boot.img header: both arrive as `console=null`). `ignore_loglevel` and `fbcon=` pass through. The DTB only contributes `quiet rcupdate.rcu_expedited=1 rcu_nocbs=0-7 noirqdebug kpti=off mitigations=off`. So the kernel got no console at all. Patch `0004` makes `console=null` mean `tty0`.
- **The MSM fbdev emulation cannot work** with this downstream SDE driver: `msm_fbdev.c` asks for an iova with `priv->kms->aspace`, which SDE never fills (it keeps `sde_kms->aspace[domain]`), hence `failed to get buffer obj iova: -22`. Not worth fixing: the first fbdev modeset would also have to pass the SDE pipeline.
- Patch `0005` adds `hanoip-splashfb`: a plain fbdev on the `cont_splash_region` (`0x9c000000`, 1080x2460, 32 bpp, stride 4320, **a8r8g8b8 / BGRA bytes**, found by reading the buffer) that the bootloader leaves scanning out. It maps the pages write-combined with `vmap` because that region is RAM without `no-map`, which arm64 refuses to `ioremap` (so `simplefb` cannot be used). It does not touch the display pipeline, and only registers if the reserved-memory node exists. fbcon takes it over at 0.35 s: 90x111 characters with the 12x22 Sun font (the largest in this tree; no Terminus 16x32 in 4.14 here). The raw buffer is in debugfs (`/sys/kernel/debug/hanoip_splashfb`): render it with PIL (`Image.frombuffer('RGBA',(1080,2460),data,'raw','BGRA',4320,1)`) to see exactly what is on the screen without a camera.
- Config: `FRAMEBUFFER_CONSOLE`, `FONTS`, `FONT_SUN12x22`, `FONT_10x18`, `FB_HANOIP_SPLASH`; `DRM_FBDEV_EMULATION` off (it only logs the iova error). `scripts/config -e FONT_SUN12x22` does not work (lowercase x): edit `.config` and run `olddefconfig`.
- Command line (in `scripts/mkbootimg-kernel` and `deviceinfo`): `ignore_loglevel fbcon=font:SUN12x22`. `ignore_loglevel` shows the whole boot log but also every `[signal] ... send death sig` line the Motorola kernel prints for each signal, which scribbles over the login prompt, so `hanoip-console.start` (installed in `/etc/local.d` by the device package, or by `scripts/setup-console`) switches it off and sets `dmesg -n 4` at the end of boot. Checked by rendering the buffer before and after.
- State after the framebuffer works: OpenRC boot output (coloured `[ ok ]`), then `Welcome to Nura! ... motorola-hanoip login:` on tty1. Still noisy in that log: `systemd-sysusers`/`tmpfiles` fail on 4.14 ("Failed to chase and open directory").
- **A/B retry counter (found 2026-10-24, important for a server):** each boot of a slot that was never marked successful costs one retry; after ~7 the ABL sets `unbootable=yes` on it and switches the active slot (here to `a`, which holds the old Lineage `boot_a`/`vendor_boot_a`, so nothing useful boots and the phone drops to the bootloader). Android marks the slot successful; pmOS does not. Seen as `fastboot getvar slot-retry-count:b` = 0, `slot-unbootable:b` = yes, `slot-successful:b` = no, `current-slot` = a, after a day of reboots. The fix is the `qbootctl` OpenRC service, which runs `qbootctl -m` at every boot: the device package now depends on `soc-qcom-qbootctl` (as `device-qcom-sm7150` does). A phone installed before that needs it installed by hand: `apk add soc-qcom-qbootctl` (packages `qbootctl`, `qbootctl-openrc` from Alpine edge community; `soc-qcom-qbootctl`, `-openrc` from the pmOS repo, which can be installed offline with `apk add --no-network`), then `qbootctl -m`. On the phone flashed before the dependency existed, `soc-qcom-qbootctl` (signed by the pmOS repo) is rejected as UNTRUSTED because its key is not installed, so install only the two Alpine packages (`qbootctl`, `qbootctl-openrc`) and run `rc-update add qbootctl default`; that is what was done on 2026-10-24. Verified end to end: after `fastboot set_active b` (flags `successful=no`, `retry=7`) a plain reboot left slot b `successful=yes` with no manual step. OpenRC lists the service as `crashed` (it runs one command and exits instead of daemonizing); that is cosmetic, and the boot log shows `SLOT _b: Marked boot successful`.
- Recovery when it happens: `fastboot set_active b` (restores retries and clears `unbootable`; the user allowed it, see the working rules), then `fastboot reboot`. With slot `a` active `fastboot boot` of pmOS images does not work either, because vendor_boot comes from the active slot.
- **Open: not yet seen by eye** (only through the scanout buffer).

## Boot test results (`fastboot boot`, nothing flashed)

- **First boot with signs of life (2026-10-24), downstream path:** header **v3** + stock `vendor_boot` and `dtbo` (slot `b`) + Cybertron kernel (stock or rebuilt) + own ramdisk. The device brought up the **NCM** USB gadget and answered ping at `172.16.42.1`.
- **pmOS initramfs runs (2026-10-24):** rebuilt downstream kernel + pmOS initramfs recompressed as LZ4 legacy + `pmos.debug-shell`, in v3 with stock `vendor_boot`/`dtbo`: ping in 20 s and a shell on `telnet 172.16.42.1 23`. UFS ok (104 partitions), `a600000.dwc3` ok, 3.6 GB RAM free, no `wlan0` yet. Image: `$HANOIP_TEST_DIR/downstream/boot-downstream-pmos-lz4.img`. The ABL cmdline includes the serial and MACs: do not copy its output into the repo.
- **The ramdisk must be LZ4 legacy** (`lz4 -l`, magic `02214c18`), like the stock ones. With gzip or zstd the device stalls on the Motorola logo, with no USB and no reboot. pmOS mkinitfs produces zstd by default: use `deviceinfo_initfs_compression="lz4"` (check that it produces legacy).
- Own test initramfs: `$HANOIP_TEST_DIR/downstream/root/` (**static** busybox, Alpine's `busybox.static`; the dynamic musl busybox makes init die). `init-reboot` (static C) reboots into recovery with `reboot(RESTART2, "recovery")` and proves that init ran.
- On the PC: NetworkManager connection `hanoip-usb` with `172.16.42.2/24` on the USB interface (created with `nmcli`, no sudo). The interface showed up as `enp5s0f3u1` (depends on the USB port).
- Rebuilt downstream kernel: `$KERNEL_DOWNSTREAM_DIR/out/` with the `/proc/config.gz` config + `DEVTMPFS`, `DEVTMPFS_MOUNT`, `USB_CONFIGFS_ECM`, `USB_CONFIGFS_RNDIS`, `SYSVIPC`, `FHANDLE`. With the host's clang 23 you need `HOSTCC="clang -Wno-error=incompatible-pointer-types ..."` and `KCFLAGS="-Wno-error ..."` (clang 16+ turns those warnings into errors). The stock kernel has **no** `DEVTMPFS` (pmOS requires it).

- **Finding of 2026-10-24: the ABL only runs images with header v3.** Test H3 (a kernel whose first instruction is `PSCI SYSTEM_OFF`): with **v0** the device reboots in ~2 s (the kernel never runs); with **v3** it powers off (charging screen). All the v0 tests in the matrix were invalid because of this, and the "reset in ~2 s" symptoms came from the ABL, not from the kernel.
- With v3 the DTB comes from `vendor_boot` (the stock one has 1 DTB, a 1.6 KB vendor ramdisk and the cmdline; the Lineage recovery lives in the `boot` ramdisk). For mainline: generate a v3 `vendor_boot` with our DTB using the same parameters as the stock one (`unpack_bootimg --format mkbootimg` on `vendor_boot_b.img`: pagesize 4096, base 0, kernel `0x8000`, ramdisk `0x1000000`, tags `0x100`, dtb `0x1f00000`).
- **Slot `a` is no good for tests:** even with stock `vendor_boot_a` and `dtbo_a`, H3 in v3 does not run on it (the device falls back to the slot `b` Lineage). The custom `vendor_boot` tests done on slot `a` on 24/10 (our DTB, empty or stock ramdisk, `dtbo` with an empty overlay) are **invalid**; repeat them on slot `b`.
- Android confirmed through `getprop` (24/10): **LineageOS 22.1, Android 15** (`lineage_hanoip-userdebug 15`), slot `b`.
- In the ABL, the base DTB of the stock `vendor_boot` has `board-id <0 0>` and `qcom,msm-name`; the board's `board-id` (`<0x4c 0xb200>`) comes from one of the 3 overlays in `dtbo` (the others are `0xb100` and `0xa100`). `$HANOIP_TEST_DIR/v3/dtbo-hanoip-empty.img` is a valid `dtbo` with an empty overlay for `<0x4c 0xb200>`.
- Test images: `$HANOIP_TEST_DIR/v3/` (`vendor_boot-hanoip.img`, `boot-debug-v3.img`). Diagnostic tests (loop at the entry point, loop in `start_kernel`, `SYSTEM_OFF`) in `$HANOIP_TEST_DIR/clkignore/`.

- Kernel 6.6.0-rc1-sm7150 + our DTS + test initramfs: the device resets and no USB gadget appears.
- U-Boot (sm7150-mainline/u-boot, sdm845-based branch) + our DTB: resets, no fastboot gadget.
- Control with the stock `boot_b.img`: Android boots (USB 22b8:2e82), then slows down and reboots by itself. The reset is **not specific to our images**.
- Full matrix (12 tests) in `docs/STATUS.md`. **All** variations through the official pmOS flow failed the same way: kernels 6.6, 6.13.4 and 7.1_rc3 (pmOS config, `EFI_ZBOOT=y`), header v0 + appended DTB and v3, original and zeroed `dtbo`, `pmbootstrap flasher boot`, U-Boot. `172.16.42.x` and the USB gadget never showed up. Image format and `dtbo` alone do not explain it.
- With header v3, the ABL uses the DTB/cmdline/ramdisk of the stock `vendor_boot` (which DTB the kernel receives is unverified); with v0 the stock `dtbo` corrupts the appended DTB unless it is zeroed. Both combinations were tested.
- Unconfirmed hypotheses: a hang before pstore/USB/framebuffer (watchdog, reserved memory, DTB handed over by the ABL); the ABL choosing the DTB by `qcom,board-id`/`msm-id` and rejecting ours.

## pstore

- Access: `adb shell su -mm -c ...` (KernelSU; plain `su` gets permission denied on `/sys/fs/pstore`).
- `dmesg-ramoops-0.enc.z` is encrypted by Motorola and unreadable. `pmsg-ramoops-0` only has Android logcat.
- No marker from our kernel ever appeared. Do not rely on pstore: use a visible console (UART) or a logger on an unencrypted partition.
- The stock Android **consumes** the records when it boots. Raw RAM does not work either: the stock kernel has no `/dev/mem`, `/dev/kmem` or `/proc/kcore`, and `/proc/iomem` is denied.
- There is no dedicated `recovery` partition (only `boot_*` and `vendor_boot_*`; the recovery lives in the ramdisk). On 2026-10-18 `fastboot reboot recovery` caused a bootloop (cause unproven) and a TWRP image for Android 11 (a valid Android boot image) **did not boot** with `fastboot boot`.
- The recovery is **LineageOS 22.1**'s (`lineage_hanoip-userdebug 15`), and `vendor_boot_b` equals the old backup: the installed Android is probably LineageOS 22.1 (Android 15) with the Cybertron kernel, not the stock Android 12 (the "Android 12" in `fastboot getvar` is stale bootloader information). Confirm with `getprop` on Android.
- That recovery uses the slot's `dtbo`: with `dtbo_b` zeroed it does not come up (this was the cause of the 18/10 bootloop). Restore `dtbo_b` before `fastboot reboot recovery`.
- Reading pstore in the recovery works: adb stays `unauthorized` until you enable **Advanced > Enable ADB** in the menu and restart the adb server on the PC; after that it is root. The recovery kernel has `CONFIG_PSTORE{,_CONSOLE,_PMSG,_RAM}=y`, ramoops at `0xaf000000`, size `0xbf800`, 256 KB console and pmsg, `ecc=0`: the console zone matches the one in our DTS.
- Test of 23/10 (kernel 7.1 zboot): `console-ramoops-0` in the recovery was still from the previous 4.14 kernel (retained RAM, with a few corrupted bits). Our kernel never initialized ramoops, consistent with the ABL being unable to load the zboot.
- **Since 2026-10-23, `adb reboot recovery` enters a LineageOS recovery** (reported by the user). Unverified: how it was installed, its adb access and whether it has `CONFIG_PSTORE_*`. It is the guide's way to read the log: after a failed mainline `fastboot boot`, reboot straight into that recovery (not into Android, which consumes pstore) and read `/sys/fs/pstore/dmesg-ramoops-0`.

## Findings of the previous session (2026-10-18)

- Tapping bootloader > Start dropped the device into the **stock recovery** (USB `18d1:d001`, adb `unauthorized`, fastboot unresponsive). *Reboot system now* in the recovery took it back to Android (slot `_b`, `boot_completed=1`, bootreason `reboot,userrequested,recovery`).
- `dtbo` does **not** contain the recovery (it is only a DT overlay; the recovery lives in the boot/vendor_boot ramdisk). `dtbo_a` and `dtbo_b` match the backups (equal hashes), so they were not the cause.
- Current `misc`: first 16 KB all zeros, no pending command. The `misc` backup starts with `bootonce-bootloader`. The hash differs only because of that. **Do not write the `misc` backup back as it is** (it would drop the device into the bootloader once); zeroing those bytes is safer.
- The cause of the drop into the recovery is **unproven**. Hypothesis: a BCB with a pending recovery command after the tests and the spontaneous reboot. The recovery reboot cleared `misc`, so it cannot be confirmed.
- If the in-use slot's `dtbo` is zeroed, that slot's Android does not boot: restore it before `fastboot continue`.

## pmOS flow: pitfalls already solved (2026-10-18)

- The device package **must not** depend on `device-qcom-sm7150` (a generic package that installs `deviceinfo` and `machine-info`, which conflict). Dependencies that work: `android-tools`, `firmware-qcom-sm7150`, `mkbootimg-osm0sis`, `postmarketos-base`, `soc-qcom`. The old `soc-qcom-sm7150` no longer exists.
- `boot-deploy` requires `android-tools` in the rootfs when `deviceinfo_generate_bootimg="true"`.
- `deviceinfo` (mainline attempts): `console=tty0 loglevel=7 ignore_loglevel`, pagesize 4096, offsets `0x8000`/`0x1000000`/`0x100`, appended DTB `qcom/sm7150-motorola-hanoip`. Without `deviceinfo_header_version` pmOS generates v0.
- **Booting straight through the ABL requires `CONFIG_EFI_ZBOOT` off.** With it on (the default in the pmOS 7.1 config), the APKBUILD packs `vmlinuz.efi` (EFI zboot, `zimg` signature, no `ARMd` at `0x38`) into `boot.img`, and the ABL cannot load it: tests 8 to 11 of the matrix ran with that kernel and are **invalid**. Zboot is for the EFI flow (U-Boot/systemd-boot), which is what the generic `device-qcom-sm7150` package uses. For the ABL: `scripts/config --file .output/.config --disable EFI_ZBOOT`, delete `.output/arch/arm64/boot/vmlinuz*` and build `Image.gz dtbs modules`; the APKBUILD then falls back to `Image.gz`.
- Check the generated `boot.img` before testing: `$PMB_WORK/chroot_rootfs_motorola-hanoip/boot/boot.img`; the kernel must start with `1f8b` (gzip) and have the hanoip DTB appended at the end.
- Build outside envkernel: `make O=.output ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- ...` (needs a clean tree via `make ARCH=arm64 mrproper`), and package **without** activating envkernel: it runs `mrproper` and wipes the build.
- After changing `deviceinfo` or the APKBUILD: `pmbootstrap checksum`, `build --force device-motorola-hanoip`, `install`. On `umount ... exit 32`, repeat `shutdown` or use `zap`.
- `git clone` from GitHub fails on this host (`SSL_read unexpected eof`); use `curl` with tarballs from `codeload.github.com`.

## UART

- Stock serial console (`ttyMSM0`, 115200 8N1): GPIO44 (TX) and GPIO45 (RX), function `qup12`, **1.8 V**.
- The user has **no physical access** to the board, so UART is ruled out for now.
- Our DTS uses `serial0 = &uart8` (inherited from sweet). It is **not confirmed** that this matches GPIO44/45; check before counting on UART on mainline.

## Current state

Active slot `b`. **Lineage was replaced by pmOS** (the user's choice on 24/10): `userdata` = pmOS rootfs (hash verified), `vendor_boot_b` = pmOS `vendor_boot`. `boot_b` = pmOS `boot.img` (flashed on 24/10): **pmOS boots on its own** (SSH about 3 min after power-on). `dtbo_a/b` and `vendor_boot_a` match the backups. Factory backups remain in `$HANOIP_BACKUP_DIR/`.

## Next steps

1. Update the kernel on the device: new `boot.img` → `fastboot flash boot_b ...`; initramfs/cmdline/DTB changes → also `fastboot flash vendor_boot_b ...`.
2. (done 24/10) `boot_b` flashed with the firewall kernel (build #4): autonomous boot with `nftables` active.
3. Wi-Fi: find out why the Wi-Fi firmware dies right after WLFW connects (see "Modem and Wi-Fi"). Untested ideas: run Android's `cnss-daemon`/`pm-service` in a minimal Android environment; compare the QMI handshake with Android's; ask in `#sm7150-mainline`.
4. Server tuning: zram, passwordless `sudo` only if the user asks, whatever services the user wants to run.
5. Repository: published on GitHub (private for now); update `STATUS.md` as the work progresses.
6. Mainline (long term): v3 `vendor_boot` with our own DTB on slot `b` (the slot `a` tests do not count). Mainline Wi-Fi uses `ath10k`, which works on sweet/sunfish with this same chip.

## Working rules

- Do not flash partitions beyond what the user asked for, and never wipe or factory reset, unless the user asks.
- **Allowed without asking (the user authorized it on 2026-10-24):** `fastboot set_active b` and `fastboot reboot`, to recover the A/B slot when the bootloader made slot `b` unbootable (see "A/B retry counter"). Check the slot flags first and say what was done.
- Before flashing or overwriting anything, inspect the target and compare with the backups (SHA256SUMS).
- When zeroing the in-use slot's `dtbo` for a test, restore the backup right afterwards, before `fastboot continue` or any reboot.
- Report test results as they are, failures included.
