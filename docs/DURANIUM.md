# Duranium on the hanoip: the plan

Hand-off for the work that follows the persistent mainline boot (2026-11-08). Read `AGENTS.md`, `docs/STATUS.md` and
`docs/MAINLINE.md` (sections 7 to 7e) first; `docs/PITFALLS.md` before touching U-Boot, the mainline root, the slots
or a flashing script. Steps are in order. Each one ends with a test (`scripts/test-*`), a commit on `beta` and, when
it touches the phone, evidence written into `docs/MAINLINE.md`.

## What the user decided

- **2026-11-07:** mainline is the goal. Its base (the kernel patches, configs and U-Boot board diffs) is this
  project's own and lives in `kernel/mainline`; never name anyone as its author.
- **2026-11-08:** Nura's Duranium becomes the server, **in all of `userdata`, in place of the 4.14 server**, after a
  verified backup of the 4.14 root. Duranium comes before moving the services to the Alpine mainline root.
- **2026-11-08:** slot a must stay a working second slot: after a complete boot from slot a, `qbootctl -m a` may write
  the GPTs of LUN 4 and LUN 2 (`98-hanoip-ml-slot.start`, the exception in `AGENTS.md`).
- **Before `userdata` is written, ask the user again**, with the backup verified and the image tested as far as it
  can be without `userdata`. Until then the 4.14 on slot b is the server, and every mainline boot keeps `userdata`
  read-only.

## Where things stand (2026-11-08, `beta`)

| Piece | State |
|---|---|
| Slot b | Active and successful: the 4.14 server (`boot_b` = LUN 5, `sdf22`), every service up |
| Slot a | Successful: `boot_a` (LUN 4, `sde22`) holds our U-Boot at 0 and the kernel image (`INIT=super-root`) at 16 MiB; the ABL boots it with no PC, SSH in about 113 s |
| Mainline root | Alpine edge + pmOS repo, OpenRC, in super's approved region (`sdb23`, offset 1174405120, size 2516582400); written by `scripts/mainline-make-root` from the 4.14 |
| `userdata` | `sdb24` (LUN 1), 116 GB: a GPT image with 4096-byte sectors holding the 4.14's boot (503 MB) and root (115 GB, 4.2 GB used) |
| U-Boot | `scripts/build-mainline-uboot` (sm7150-mainline/u-boot `tauchgang` at `f8c04469`, the board diffs in `kernel/mainline/uboot/base`, tree `a38f1757`), already with the EFI loader, `bootefi`, the EFI boot manager, `blkmap` and FAT |
| Kernel | `scripts/build-mainline-kernel`: kernel.org 7.3 + the 195 patches in `kernel/mainline/patches`, trees `30fd2ef8` (v7.3) and `0cf5808c` (patched), `kernel/mainline/config-7.3-hanoip` plus `kernel/mainline/server.config`; it already has `EFI_STUB`, `EFI_ZBOOT`, dm-verity, EROFS, dm-crypt, overlayfs and efivarfs |

## What Duranium is (`docs/MAINLINE.md`, section 7e)

- Built with mkosi by [`duranium-build`](https://gitlab.nura.eco/nura/duranium-build) (`scripts/build-image.py
  device-<name> ui-<ui> --release=edge`). It clones Nura's pmaports and resolves the device package, its first
  `device-<name>-kernel-*` subpackage and its `deviceinfo` (architecture, sector size, `deviceinfo_dtb`) from there.
  `git clone` works; the web pages sit behind a bot check (Anubis), so read them in a browser.
- systemd throughout, the initramfs too. Boot: U-Boot's EFI, systemd-boot on the ESP, a UKI (kernel, initramfs,
  command line, DTBs as `.dtbauto`; systemd-stub picks the one whose `compatible` matches what U-Boot passes). A new
  image that fails twice falls back to the previous one.
- Disk (`mkosi.repart`): ESP 1 GB, `/usr` verity 400 MB, `/usr` 5 GB (EROFS) in the image; on first boot
  systemd-repart adds slot B (5.4 GB) and the root on LUKS2 over the rest. 16 GB minimum. On phones the raw image goes
  into `userdata` and U-Boot's `blkmap` presents `userdata` as the disk.
- LUKS2 is created with a blank passphrase and unlocks by itself. The first-boot wizard (f0rmz) creates the user and
  offers a passphrase **on the screen**: a headless image must do that step another way.
- Built images exist only for other devices (fairphone-fp5/fp6, google-sargo, oneplus-enchilada/fajita, x86_64...).
  The kernel package must put `vmlinuz` in `/boot`, the DTBs in `/boot/dtbs/`, the modules in `/usr/lib/modules/`
  (`mkosi.prepare` moves them into the UKI). `duranium-build` was read at commit `cdaf2da0`.

## The steps

### 1. A pmaports package of our 7.3 kernel

**State (2026-11-08, `beta`):** written and tested without the phone. Package `linux-motorola-hanoip-mainline` in
`pmaports/device/testing/`; `scripts/test-mainline-kernel-package` checks its copies, its trees, its tarball list and
its install paths, and its mutations were each caught. **Not built yet.** Still open, in order:
- Build (the user runs it, `pmbootstrap` needs sudo): `scripts/setup-pmaports`, then
  `scripts/pmb build --force linux-motorola-hanoip-mainline`. Do not activate envkernel.
- Compare with `$HANOIP_SRC_DIR/hanoip-mainline-7.3/out`: the DTB must come out `900e11eb…` (it held across GCC 13.3
  and 16.2). `Image` will differ, since Alpine's cross GCC is not the host's: compare the module list and the `.config`
  without the compiler lines.
- Boot evidence before step 5 depends on it: one RAM-only `fastboot boot` of the built `Image` (the method in
  `docs/MAINLINE.md`, section 7). Not done.
- Not run: the package's `makedepends` on a clean chroot, and `depmod` with the zstd modules on the phone's `kmod`.
- Step 2 changes `kernel_mainline` in `device-motorola-hanoip`, which still depends on `linux-postmarketos-qcom-sm7150`.
  The mainline subpackage must come first: `pmaports.py` takes the first `device-<name>-kernel-*`.
- Duranium's `mkosi.prepare` takes `/boot/vmlinuz` (moved into `usr/lib/modules/*/`) and the DTB from `/boot/dtbs`:
  the package installs both where it looks. Check this again when the image is built, not before.

`linux-motorola-hanoip-mainline`, next to the 4.14's `pmaports/device/downstream/linux-motorola-hanoip`, modelled on
Nura's `device/community/linux-postmarketos-qcom-sm7150/APKBUILD`:
- sources: `linux-7.3.tar.xz` from kernel.org and, from this repository, the 195 patches in `kernel/mainline/patches`,
  `kernel/mainline/config-7.3-hanoip` and `kernel/mainline/server.config` (copied into the package directory, or
  referenced from it). The checksums with `pmbootstrap checksum`.
- `prepare()`: `git am` the patches (one has a rename, none is binary) and check both trees, as
  `scripts/build-mainline-kernel` does; merge the config with `merge_config.sh` and `olddefconfig`.
- `build()`/`package()`: `Image`, modules and `qcom/sm7150-motorola-hanoip.dtb`; the out-of-tree
  `hanoip_mmi_unit_info` (`kernel/mainline/extra/`, SMEM item 134, needed by the modem); install like the Nura
  package (`modules_install zinstall dtbs_install`, the hanoip DTB under `/boot/dtbs/qcom/`).
- Decided (2026-11-08): from source, not `--envkernel`. envkernel bind-mounts the source tree and needs the kbuild output
  inside it, and it forces `pkgrel=0`; a source package builds from this repository's copies alone.
- Test: a `scripts/test-*` that the package pins the same commit and trees as `scripts/build-mainline-kernel`; build
  it and compare `Image`, the DTB and the module list with `$HANOIP_MAINLINE_DIR/out`.

### 2. The device package for mainline

**State (2026-11-08, `beta`):** done in commits f5281e7 and 284fe19. The subpackage `kernel_mainline` now comes first and depends
on `linux-motorola-hanoip-mainline`; `deviceinfo` has the mainline overrides (`deviceinfo_dtb_mainline`,
`deviceinfo_kernel_cmdline_mainline`, `deviceinfo_generate_bootimg_mainline="false"`,
`deviceinfo_flash_kernel_on_update_mainline="false"`). Firmware stays local (gitignored, from `$HANOIP_COLLECTION_DIR`).
- Test: `scripts/test-deviceinfo` extended for the mainline overrides (commit 284fe19).

### 3. systemd units for the phone's own pieces

**State (2026-11-08, `beta`):** the pieces are packaged, still as OpenRC. `device-motorola-hanoip-mainline`
(commits `2ea3e8d`, `59a2196`, `f81c1c9`) carries the thirteen files of `kernel/mainline/root-overlay` as copies, each
identical to the overlay and in git, which `scripts/test-device-mainline-subpackage` checks together with the install
paths and modes; `amove` in `mainline()` keeps them out of the 4.14 server's package. What ships is what the Alpine
root in super runs today: `/etc/init.d` and `/etc/local.d` scripts, the two binaries, the module blacklist,
NetworkManager's configuration and chrony's dispatcher. Duranium is systemd, so the units are still to write. In order:
- Decide what survives. `05-hanoip-ml-guard` (every disk read-only except super), `20-hanoip-deadman` (back to the
  server after 15 minutes) and `99-hanoip-ml-up` (stage 1's boot deadline) exist because the Alpine root in super is a
  development root that must never write `userdata`; Duranium lives in `userdata`, so the guard's rule, and the matching
  line of `AGENTS.md`, ends at the switch, the boot deadline is stage 1's, and the battery needs only the charge limit,
  not the reboot under 50 %.
- Write the units: `hanoip-efs` (the modem's EFS copied to `/run/efs`), `rmtfs` and `tqftpserv` (served read-only),
  `hanoip-modem` (SMEM item 134, the remoteprocs, IPA and ath10k, `Before=NetworkManager`), `hanoip-charge`
  (`Restart=always`, the modules and the debugfs mount), `hanoip-usb-net` (the configfs gadget and `unudhcpd`),
  `hanoip-slot-mark` (oneshot after sshd, `systemctl is-active` instead of `rc-service`), `hanoip-bot`, and
  `RuntimeWatchdogSec=` in `systemd.conf.d` in place of the watchdog daemon. NetworkManager runs
  `dispatcher.d/50-hanoip-ml-chrony` under systemd too.
- Build the mainline kernel package (step 1) so the modules the units load exist.
- Test on the phone once the Duranium image boots.

### 4. A Duranium image for the hanoip

- `duranium-build` with the mkosi container (`scripts/mkosi-container`, podman) or Arch's mkosi 27.1; our packages
  through a local apk repository the build can read; `ui-console`.
- Headless first boot: the user with the SSH key from the server, no f0rmz on the screen, the blank LUKS2
  passphrase kept (it unlocks by itself).
- In the image: openssh, NetworkManager, Tailscale, nftables, AdGuard Home, Samba, Syncthing, the Telegram bot.
  Secrets (`/etc/hanoip/telegram.env`, Tailscale's state, Wi-Fi profiles) come from the phone at install, never from
  git.
- Test: build it, list its partitions and check the UKI holds the hanoip DTB and our kernel.

### 5. U-Boot boots Duranium

- An environment like `qcom-phone.env`: `scsi scan`, `part start`/`part size` of `userdata`, `blkmap create root`,
  `blkmap map root 0 <size> linear scsi <dev> <start>`, then the EFI boot manager or `bootflow scan`. Keep
  `bootdelay=-2`, `scsi dev` before `scsi read`, the unfed watchdog and the Volume Down menu that stops it.
- Before `userdata` holds Duranium, test what can be tested: a RAM boot of this U-Boot finds `userdata` through
  `blkmap`, finds no ESP and resets to slot b.

### 6. Slot b becomes the rescue system

Until the switch, slot b stays the 4.14. At the switch, `boot_b` gets our persistent U-Boot and the Alpine root's
kernel image, so a Duranium that fails falls back to the Alpine root in super (Wi-Fi, Tailscale, SSH). The U-Boot
environment reads `boot_a` by name now; a slot-b copy must read `boot_b`. Back up `boot_b`, `vendor_boot_b` and
`dtbo_b` first.

### 7. Back up the 4.14 server

**State (2026-11-08, `beta`):** done once with `scripts/backup-server`. In `$HANOIP_BACKUP_DIR/server-4.14-20261108`
(folder 700, files 600): `root.tar.gz` (1.7 GB, 24,228 entries, no swap file), `boot.tar.gz` (57 MB), `packages.txt`
(534 packages) and `runlevels.txt`. The hashes matched on both sides, `/etc/apk/world` restored from the archive equals
the phone's, and the phone's copy was removed. The services kept running, so a database written during the tar
(AdGuard's statistics, Syncthing's index) may be caught mid-write; tar skipped 3 Samba sockets. Run it again right
before step 8, so the backup holds the server's last state.

- What: the root inside `userdata` (4.2 GB used): service configs and data (AdGuard Home, Samba, Syncthing, the bot,
  `hanoip-status`), `/etc`, `/home`, `/var/lib`, plus the list of installed packages.
- How: a file-level archive, transferred throttled (`pv -L 1m` over USB, or over the tailnet), stored in
  `$HANOIP_BACKUP_DIR`, its hash checked on both sides, one file restored as a test.

### 8. Write Duranium into `userdata` (only after asking the user again)

- Motorola's fastboot refuses `userdata`: write the raw image through the Lineage recovery with `dd` and check the
  hash, as `scripts/flash-rootfs` does for the 4.14's GPT image (extend it rather than write a new one), or from the
  Alpine rescue root once the user approves that.
- Then: first boot, SSH, the services restored from the backup, a reboot, an A/B update and its rollback tried once.
  Write the evidence into `docs/MAINLINE.md` and `docs/STATUS.md`, and retire the 4.14 in the README.

## State of the overnight run (2026-11-09, `beta`)

Everything up to the write of `userdata` is done and tested without it; the write itself was refused by the tool's safety
check (irreversible), so it waits for the user. What exists:
- Step 1 built: `linux-motorola-hanoip-mainline-7.3-r0.apk`, DTB `900e11eb…`, the same 863 modules as the host build. Not yet
  booted from the package's own `Image` (the boots so far used the host build, same sources).
- Steps 2 and 3: the device package (pkgrel 47) builds with `scripts/pmb build --force device-motorola-hanoip`; its mainline
  subpackage ships the USB gadget, the boot-health check, the modem and charge units, and the UKI's command line.
- Step 4: `scripts/duranium-image` builds a console image in Nura's container in about 150 s: 6.4 GB, 4096-byte GPT, ESP
  with `EFI/BOOT/BOOTAA64.EFI` and one UKI without a DTB of its own, `/usr` verity and EROFS, vendor firmware, the login user, root by
  key, wizard skipped. Inspected from the outside (UKI sections, command line, files in the EROFS); never booted.
- Steps 5 and 6: the U-Boot with the blkmap/EFI environment is in `boot_a` and `boot_b` of the phone (same image, the ABL's slot
  decides). On slot a with the 4.14's userdata it found no ESP and booted the rescue root (seen: SSH over USB, kernel
  7.3.0-sm7150, slot `_a`). The positive path (an ESP in userdata) is untested.
- Step 7: fresh backup `server-4.14-20261109-final` (hashes equal on both sides), and `boot_a`/`boot_b` as they were.
- Left as it was found: boot_b was put back from its backup, slot b is active and the 4.14 server runs again (every service
  up at 00:17). `boot_a` keeps the new U-Boot, which behaves as the old one while userdata holds no ESP.

To finish (needs the user's yes, because it replaces the 4.14 server's userdata):
1. `scripts/reboot-bootloader`; `fastboot flash boot_b $HANOIP_MAINLINE_DIR/out/boot_a-duranium.img` (the rescue slot must not
   be the 4.14 any more); `fastboot set_active a`; `fastboot reboot` (slot a boots the rescue root while userdata has no ESP).
2. `scripts/duranium-write-userdata` (about 15 minutes), then `qbootctl -a a` from the rescue root or `fastboot set_active a`
   so slot a is not marked successful, then reboot: U-Boot finds the ESP, systemd-boot starts the UKI, `hanoip-health` marks the
   slot good once sshd and an address are up, and reboots after ten minutes when they are not.
3. If the DTB choice is wrong (no boot, no USB), `HANOIP_UKI_DTB=1 scripts/duranium-image` keeps the package's DTB in the UKI.

## State after the first write of `userdata` (2026-11-09, paused by the user)

**Done**
- `userdata` now holds the Duranium image (6.4 GB, written with `scripts/duranium-write-userdata` from the rescue root, 718 s,
  SHA-256 read back equal). The 4.14 server is no longer on the phone: its backup is `server-4.14-20261109-final` in
  `$HANOIP_BACKUP_DIR` (hashes checked again before the write); `boot_b` was backed up before it got the rescue U-Boot.
- Slot b is the rescue (`boot_b` = `boot_a-duranium.img`, the Alpine root in `super`, SSH over USB and Tailscale only); slot a
  is the active slot and tries Duranium first. The services of the 4.14 (AdGuard DNS, Samba, Syncthing, the bot) are down until
  Duranium boots.
- First boot of slot a: U-Boot found the ESP through `blkmap`, systemd-boot started the UKI and the package's own kernel ran
  (the EFI path is proven up to the kernel), then it oopsed at 0.46 s in `fbcon` -> `simplefb_hanoip_imageblit` writing to
  `ffff00001c000000` (level 2 translation fault); the ABL fell back to slot b. Read from `/sys/fs/pstore/console-ramoops-0`
  in the rescue root.
- Cause: patch `0121-hanoip-simplefb` writes the framebuffer at `0x9c000000` through the linear mapping. The legacy boot leaves
  that RAM mapped (`/proc/iomem`: inside `System RAM`); the EFI memory map from U-Boot reserves it, so it is not mapped.
  The DTB in the UKI is not the cause (`HANOIP_UKI_DTB=1` would not help).
- Fixed and committed on `beta`: the patch (both copies) takes the linear mapping only when
  `pfn_is_map_memory()` is true, otherwise `ioremap_wc`; the patched tree is now `0cf5808c` (was `276fbce0`, only that line
  differs), `pkgrel=1`, `scripts/test-mainline-kernel-package` fails on the old patch and passes on the new one. A write script
  bug found on the way is already committed (the rescue root's own loop over `super` stopped it).

**Left, in order**
1. Done: the sums, the fix and its test, the PITFALLS row and the CHANGELOG `Fixed` line.
2. Build (the user runs it): `scripts/pmb build --force linux-motorola-hanoip-mainline`, then `scripts/duranium-image`.
3. `grep` the patches for the same class of bug: `phys_to_virt`, `__va`, hard-coded `0x9...` addresses, anything that assumes
   a reserved region is mapped. The next EFI-only crash costs a 15-minute write.
4. `scripts/duranium-write-userdata` again (safe while no boot reached userspace: systemd-repart has not touched the GPT yet),
   `fastboot set_active a`, reboot, watch for SSH. Once a boot reaches userspace, a full rewrite wipes what repart created.
5. Then: first boot checks, services restored from the backup, a reboot, the A/B rollback once, `docs/STATUS.md` and the README.

## State after the second write of `userdata` (2026-11-09, evening, `beta`)

**Done and seen on the phone**
- The kernel package with the framebuffer fix (`linux-motorola-hanoip-mainline-7.3-r1`) and a new image were built; the UKI's
  kernel is byte-identical to the package's `vmlinuz`, the DTB `900e11eb…`. The 4.14 backup's hashes were checked, then
  `userdata` was written again from the rescue root (718 s, SHA-256 read back equal). Slot a active.
- Slot a boots Duranium through EFI to `Duranium login:` on the screen: the framebuffer oops is gone. Nothing could reach the phone
  at first, because the units were not enabled (`docs/PITFALLS.md`: mkosi's `systemctl preset-all` removes enable links in
  `/etc`). The systemd-boot menu has a `debug-shell` entry: a telnet shell in the initramfs on `172.16.42.1:23`; from there the
  root was mounted read-only (`/dev/mapper/root`, blank-passphrase LUKS) and its journal read.
- Fixed live (the image on the phone is r47-based and still has no preset): links for `hanoip-usb-gadget` (in the mutable `/etc`
  under `/var/lib/extensions.mutable`), then `systemctl enable` for `hanoip-health`, `hanoip-charge`, `hanoip-modem` and `tailscaled`.
  Fixed in the package for the next image: preset `10-hanoip.preset`, `device-motorola-hanoip` `pkgrel=49` (**not built**: `pmbootstrap`
  needed sudo and the password in `.env` was refused that evening).
- Checked after a reboot of Duranium: it comes back by itself, SSH over USB (`root`, key), `hanoip-health` marks slot `_a` good,
  Wi-Fi (the two profiles from the 4.14 backup, copied with mode 600), Tailscale with the node state from the backup (same name,
  SSH over the tailnet), port 22 closed from the LAN, nftables with the USB/tailnet SSH rules. The tailnet host key changed (new
  system): its fingerprint was compared with the one seen over USB, then the old `known_hosts` entry was replaced.

**Still open**
1. AdGuard Home (the household's DNS), Samba, Syncthing and the Telegram bot are **not in the image** and are down. They have to be added to the
   image's package list and the data restored from `server-4.14-20261109-final` (`/var/lib` of each, `/etc/AdGuardHome.yaml`, Samba's config).
2. `hanoip-pd-mapper` fails with "no pd maps available": the firmware's protection-domain maps are not in the image. Wi-Fi works without it.
3. Rebuild the package (`scripts/pmb build --force device-motorola-hanoip`, needs sudo) and the image, and check offline that the
   confext has the three enable links, before the next write. A write is now only needed for a changed image: A/B update and its rollback are untested.
4. The login user has no sudo (by design); administration is `root` over USB.

## Rules that stay

- `scripts/check-repo` before every commit and push; one commit per change; English `type(scope): summary`; no AI
  mentions, no `Co-Authored-By`; push only to `beta`, contributors only `klockmt`.
- Personal data only in `.env` (`$HANOIP_*`); never print or commit keys, passwords, tokens, serials or MACs. The
  ABL's command line in the DT (`u-boot,abl-bootargs`) holds the serial and the MACs: read only
  `androidboot.slot_suffix` from it.
- Writes from mainline: super's region (bounded loop, hashes outside before and after) and the `qbootctl` exception
  only. `qbootctl` with no arguments is not read-only.
- Never wipe or factory-reset; inspect and hash every partition before writing it; large USB transfers throttled.

## Lessons from 2026-11-08 that matter here

- A key is pending at U-Boot's autoboot: `bootdelay=-2`. Read U-Boot's own console with `fastboot oem console`.
- `scsi read` reads the current device: `scsi dev` first. UFS LUN 0 (16 MB) answers slowly under mainline.
- The unfed watchdog bites about 50 s after U-Boot starts (it programs 30 s).
- `fastboot getvar` without a device waits forever: wrap it in `timeout`, watch `lsusb`. `fastboot getvar all` prints
  the serial number: filter it.
- `tailscale ping` gets no reply from this node; SSH over the tailnet works.
- A GPT can be checked read-only by dumping its first 6 and last 5 sectors (4096 bytes) and checking both header
  CRCs, both entry-array CRCs and that the two arrays match: that showed `qbootctl -m a` changed only A/B bits.

## State of the preparation for the single-system server (2026-11-09, night, `beta`)

The goal the user set: Duranium is the only system on the phone (both Android slots), with everything the 4.14 server did, SSH
guaranteed, tested, and then a v1.0.0 only with the user's yes. **Written and tested without the phone** (each with a
`scripts/test-*` that fails on the old state): the heat protection, LED and `/status` on the 7.3 sysfs; units for AdGuard Home,
Samba, Syncthing, the heat protection, the LED, the bot (each off until its restored data or its disk is there) and the NAS mounts;
the 4.14 package list plus `fastfetch` and `lstopo` (`hwloc-tools`) in `device-motorola-hanoip-mainline`; what a headless server does
not need switched off (avahi, rtkit, oomd, getty, LLMNR/mDNS, core files, backlight); a PAM service for sudo-rs; the login user as
1001 (hdshare owns 1000); the pd-mapper maps in the image script; `scripts/restore-server` (data, `password`, `harden`); and the U-Boot
environment that tries Duranium first from both slots with the Alpine root in `super` as the last resort.

**Not done, and why** (nothing below was run on the phone): the build needs `pmbootstrap` with the PC's sudo (the agent's attempt to
read that password was refused by the harness, so the user runs it); the image, the rewrite of `userdata`, `restore-server`, the
flash of `boot_a`/`boot_b` and every stability test come after it. The password step and `harden` were refused as well, and are
left to the user. In order:

1. `scripts/setup-pmaports`, then `scripts/pmb build --force device-motorola-hanoip` (the kernel package is unchanged: r1), then
   check `ls $PMB_WORK/packages/edge/aarch64 | grep 'device-motorola-hanoip.*-r49'`.
2. `scripts/duranium-image`; look inside before writing: the confext's `/etc/passwd` must give `syncthing` and `adguardhome` any ID
   (not 105/106 if chrony or dnsmasq hold them), `etc/pam.d/sudo`, the preset, the units, the `*.jsn` maps in the firmware directory.
3. `scripts/duranium-write-userdata` from the rescue root (the same flow as the first two writes), `fastboot set_active a`.
4. `scripts/restore-server`, then `scripts/restore-server password`, then `scripts/hanoip-sudo 'id -u'` over the USB **and** over the
   tailnet; only then `scripts/restore-server harden`. Reboot once and check `systemctl is-system-running` = `running`.
5. Back up `boot_a`, `boot_b`, `vendor_boot_*`, `dtbo_*` to `$HANOIP_BACKUP_DIR` with `SHA256SUMS`, then
   `scripts/build-mainline-uboot`, `scripts/mainline-boot-a-image` (`DRY_RUN=1` first) and flash `boot_a` and `boot_b` only; prove
   each slot (`fastboot set_active b`, reboot, `fastboot getvar slot-successful:b`) and then the Alpine last resort.
6. The stability tests of the plan (10 reboots, panic, watchdog, A/B update and rollback, network, services, soak), each finding with a
   test and a row in `PITFALLS.md`; then the v1.0.0 question to the user.

## State on 2026-11-11 (`beta`): Duranium is the running system

The image built on 2026-11-09 (`IMAGE_VERSION=26110901`, device package `pkgrel 49`) was written to `userdata` and restored
(`scripts/restore-server`, `password`, `harden`) on 2026-11-10; steps 1 to 4 of the list above are done. Checked on the phone on
2026-11-11 over the USB cable, without the NAS disk:

- Slot `_a`, kernel `7.3.0-sm7150`, `systemctl is-system-running` = `running`, no failed unit, on three boots in a row;
  `hanoip-health` marked the slot good each time. `watchdog.open_timeout=300`, `panic=5`, `RuntimeWatchdogSec=20`.
- Access: `scripts/hanoip-sudo 'id -u'` prints 0; `sshd -T` gives `permitrootlogin no`, `passwordauthentication no`; a login as
  root with the key is refused. Root's `authorized_keys` comes back at every boot (the image's tmpfiles line copies it), so
  `harden` does not remove it for good; it is unused while `PermitRootLogin no` stands.
- Network: Wi-Fi and Tailscale up by themselves; from the PC on the same LAN no TCP port of the phone answers, while it answers
  ping. SSH over the tailnet was not tried: the PC was logged out of the tailnet.
- Services active: AdGuard Home (`127.0.0.1:53` and `:3000`, resolves a name), `hanoip-charge` (78 %, not charging, on the
  cable), `hanoip-thermal`, `hanoip-led`, `hanoip-bot` (its unit; no command was sent from Telegram), zram (zstd, 5.2 GB),
  `systemd-timesyncd` for the clock (chrony is not used on Duranium). `smb` and `syncthing` are enabled and wait for the disk.
- `hanoip-status` prints every line; the cycle count reads `n/a` on the mainline gauge.

Three bugs found and fixed that day (rows in `PITFALLS.md`, checks in `scripts/test-duranium-boot`), applied to the phone's
`/etc` by hand and measured over two reboots; the package that carries them is `pkgrel 50`, **not built yet**:
the NAS mounts without `nofail` held sshd for 90 s (sshd at 101 s, now 12.8 s); the journal's limit lost to pmOS's 1 GB (501 MB,
now about 200 MB); and the kernel flooded the log trying to set the PMIC RTC (about 40 lines a second, now none).

Still open before a v1.0.0, in order: the NAS disk plugged in (the mounts, Samba from a client over the tailnet, Syncthing,
a reboot with and without it; #27, #28); the bot's commands from Telegram; step 5 above (back up and flash `boot_a` and
`boot_b`, prove each slot and the Alpine last resort); a build of `pkgrel 50` and its delivery as an A/B update, with one
rollback of a bad image; then the stability list of step 6 (ten reboots, panic, watchdog, Wi-Fi loss, a 24 to 72 h run).

### Later on 2026-11-11: the new U-Boot in `boot_b`, and what stopped the slot test

- Backup first: `boot_a`, `boot_b`, `vendor_boot_a/b` and `dtbo_a/b` read from the running Duranium into
  `$HANOIP_BACKUP_DIR/2026-11-11-before-uboot/` with `SHA256SUMS`, checked on both sides. Both boot partitions held the
  same image, `boot_a-duranium.img` of 2026-11-08 (`ea819e45…` over the whole partition), the U-Boot that tries Duranium
  on slot a only.
- `scripts/build-mainline-uboot` and `scripts/mainline-boot-a-image` built the image with the U-Boot that tries Duranium
  first from either slot. It was written to **`boot_b` only**, with `dd` from Duranium (the phone was on the hub, so no
  fastboot), and read back: the whole partition is `bc2c6276…`, the image padded with zeros. `boot_a` is unchanged.
- The new U-Boot **has not run yet**: slot b could not be made active from the phone. `qbootctl -s b` fails ("Failed to
  set active": the kernel has no `SCSI_UFS_BSG`, which changes the UFS boot LUN), and this build of qbootctl lists `-i`
  (write the GPT flags anyway) in its help but does not accept it. `xbl`, `xbl_config`, `abl`, `tz` and `hyp` are
  byte-identical in both slots, so the GPT flags alone would do. `boot_a` stays as it is until the new U-Boot has
  booted once: two slots with a U-Boot nobody has seen boot would leave no way back but the cable.
- To finish step 5 with the cable on the PC: `fastboot set_active b`, reboot, check Duranium comes up and
  `fastboot getvar slot-successful:b` says yes; then write the same image to `boot_a`, `fastboot set_active a`, check
  again; then the Alpine last resort from a RAM boot.
- `hanoip-health` on the phone runs the fixed script from `/etc/hanoip` through a drop-in
  (`hanoip-health.service.d/r50.conf`) until the image carries `pkgrel 50`; remove both then.
- The owner sent `/status` to the bot on Telegram and got the full report (Ethernet through the hub, 77 %, bypass).
- The hub's Ethernet works (`eth0`, `cdc_ether`); the NAS disk did **not** appear on the bus. The only storage device is
  the hub's own card reader (`05e3:0751`, no medium), and nothing sits on the SuperSpeed bus. Samba and Syncthing are
  still unchecked.
- Ten reboots in a row over the tailnet, with the three fixes in `/etc` and without the disk: SSH over the tailnet
  answered 58 to 60 s after `reboot` every time, sshd started 12.6 to 12.9 s into the boot, the slot was marked good
  ten times, no `disallowed SPMI` line. Nine boots ended `running`; one ended `degraded` because
  `bootmac@bluetooth.service` timed out, which is now masked in the package (`PITFALLS.md`).
- `scripts/pmb build --force device-motorola-hanoip` ran with the PC's sudo (the user approved the helper once), and
  `scripts/duranium-image` built an image from it.

### The A/B update to `pkgrel 52`, its rollback and a panic (2026-11-11, night)

- **Delivery.** `scripts/pmb build --force device-motorola-hanoip` (`pkgrel 52`), then
  `scripts/duranium-image --image-version=26111101` (without the option the image keeps the version in duranium-build's
  `mkosi.version`, and sysupdate offers nothing). Checked inside the image with `dump.erofs` before sending it: the
  tmpfiles line, the `nofail` mounts, `90-hanoip.conf`, the health script and the empty udev rule. The three files
  sysupdate wants (`…usr-arm64-verity.<uuid>.raw.zst`, `…usr-arm64.<uuid>.raw.zst`, both compressed with `zstd`, and
  `….efi`) went to `/var/tmp/hanoip-update/` on the phone over the tailnet (777 MB in 107 s), hashes equal. The drop-ins in
  `/etc/sysupdate.d/*.transfer.d/10-hanoip-test.conf` point sysupdate at that directory (`Type=regular-file`); they stay,
  since no server publishes an image for this phone.
- **Update.** `/usr/lib/systemd/systemd-sysupdate update` wrote the free partition pair and
  `…_26111101+2-0.efi`. The hand-made fixes were deleted from the writable `/etc` layer
  (`/var/lib/extensions.mutable/etc`) first, so that the package's files are the ones in use. After `reboot`: SSH over
  the tailnet in 59 s, `IMAGE_VERSION=26111101`, `running`, nothing failed, sshd at 12.75 s, no SPMI line, the journal
  limit at 200 MB, the slot marked by the packaged `hanoip-health`, no `bootmac@bluetooth` unit, the boot entry renamed
  to `…_26111101.efi` (blessed). From the LAN no TCP port answers and root is refused.
- **Rollback.** A UKI broken on purpose (1 MiB of zeros in the middle of its `.linux` section) was placed as
  `…_26111102+2-0.efi`, the default entry. After `reboot` the phone came back by itself on 26111101, 117 s later, with
  the bad entry at `+0-2`; slot a stayed the ABL's slot. The bad file was removed. Which reset ended each failed try
  (the EFI stub returning, or the watchdog U-Boot leaves running) was not looked at.
- **Panic.** `echo c > /proc/sysrq-trigger` (through `systemd-run`: a background shell of the ssh session dies with it):
  "Kernel panic - not syncing: sysrq triggered crash", and the phone answered SSH on a new boot 197 s after the panic
  was armed, `running`, slot marked.
- **USB role after a reboot.** With the hub left plugged in, every boot leaves the port in the device role
  (`/sys/class/typec/port0/data_role` and `/sys/class/usb_role/a600000.usb-role-switch/role`), so the hub, its Ethernet
  and any disk are not there; the tailnet ran over Wi-Fi. Plugging the hub into the running system gives the host role.
  Writing `host` to the Type-C port changes nothing; writing `host` to the role switch brings the hub and `eth0` back.
  Nothing does that at boot yet: it must not take the USB network away from a PC on the cable, which is the way in
  when everything else is down, and that half cannot be tried without the cable. Open.
- **Not done:** the NAS disk (it never appeared on the bus, in either role); slot b and `boot_a` (the cable); a boot
  with sshd down (the health script's reboot after ten minutes is only covered by `scripts/test-duranium-boot`); a
  watchdog test on Duranium; Wi-Fi loss; a 24 to 72 h run.

### The NAS disk, once it was switched on (2026-11-12, 00:00 UTC)

- The disk (`152d:0583`, 465.8 GB, btrfs `hd-a` and `hd-b`) appears on the hub and both units mount it by hand
  (`systemctl start`), read-write with `compress=zstd`; `smb` and `syncthing` start. A disk that arrives after the boot is
  **not** mounted by itself: the mounts are only tried at boot.
- `/mnt/hd-a/obsidian` belonged to the 4.14's number for syncthing, which is `adguardhome` here: the owner step of
  `scripts/restore-server` had run while the disk was away. Done by hand with the same command; Syncthing then scanned
  the folder ("Ready to synchronize"). No file on the disk is left without a user or group.
- Samba: port 445 answers over the tailnet and not on the LAN (Wi-Fi and Ethernet addresses), and an anonymous
  `smbclient -L` is refused. A login with the share's password from a client was not tried.
- After the forced host role (the entry above) the whole hub is at 12 Mbit/s and a raw read of the disk did not finish
  256 MB in 90 s. The services were stopped and the disk unmounted again, so the hub can be unplugged and plugged back
  in, which is what gave 480 Mbit/s earlier that evening.

## Where it stands (2026-11-12): what is done and what a v1.0.0 still needs

The dated sections above are the log; where one of them says "not built" or "still open", this section is what holds.

**The phone now.** Duranium image `26111101`, device package `pkgrel 52`, kernel 7.3.0, slot a, reached over the tailnet
(Wi-Fi, and Ethernet while the hub is in the host role). `boot_a` holds the U-Boot of 2026-11-08, `boot_b` the new one
(never booted). The NAS disk is plugged in and **not mounted**. Everything is on `beta`; `main` is at v0.2.2.

**Done and seen on the phone:** the server's services except the two that need the disk; sudo for the login user, no
root login, no port open to the LAN; ten reboots in a row; an A/B update; the automatic rollback from a broken boot
entry; a kernel panic; `/status` from Telegram; the disk mounted by hand with Samba and Syncthing started.

**Open, in the order to do it** (each needs what is in brackets):
1. [the hub unplugged and plugged back in] Mount the disk at 480 Mbit/s, start `smb` and `syncthing`, measure the read
   rate, `btrfs scrub`, a Samba login and a Syncthing sync from a client (#27).
2. [the cable to the PC] `fastboot set_active b`, reboot: the new U-Boot boots Duranium and `hanoip-health` marks slot b
   (`fastboot getvar slot-successful:b`). Then the same image into `boot_a`, back to slot a, and the Alpine last resort
   from a RAM boot. The backup is `$HANOIP_BACKUP_DIR/2026-11-11-before-uboot/`.
3. [the cable, and a decision] The USB role at boot: with the hub plugged in, a boot leaves the port in the device role,
   so there is no Ethernet and no disk until someone plugs the hub in again. Only reboots were tried, not a cold boot.
   Whatever sets the host role must not take `usb0` away from a PC on the cable. With it, the disk mounted by itself
   when it appears, and a reboot with the disk plugged in (#28).
4. `hanoip-health` counts the phone as reachable only with an address on `usb0` or `wlan0`: on Ethernet alone, with the
   Wi-Fi down, it would not mark the slot and would reboot the phone ten minutes into every boot. Found by reading on
   2026-11-12, not fixed, no test yet.
5. Root's `authorized_keys` comes back at every boot (the image's tmpfiles line), unused while root login is off.
6. The tests not run: a boot with sshd down, the watchdog on Duranium, Wi-Fi loss, 24 to 72 hours without a restart.
7. `docs/HOMELAB.md` rewritten for Duranium; then `git log main..beta` to the user and the v1.0.0 question.

**How to make and deliver an update** (done once, 2026-11-11): `scripts/setup-pmaports`,
`scripts/pmb build --force device-motorola-hanoip` (the PC's sudo: the user approves it), `scripts/duranium-image
--image-version=YYMMDDNN`, look inside with `dump.erofs`, compress the `usr` and `usr-verity` files with `zstd`, copy
them and the `.efi` to `/var/tmp/hanoip-update/` on the phone, `/usr/lib/systemd/systemd-sysupdate update`, reboot.
