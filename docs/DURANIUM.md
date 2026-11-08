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
| Kernel | `scripts/build-mainline-kernel`: kernel.org 7.3 + the 195 patches in `kernel/mainline/patches`, trees `30fd2ef8` (v7.3) and `276fbce0` (patched), `kernel/mainline/config-7.3-hanoip` plus `kernel/mainline/server.config`; it already has `EFI_STUB`, `EFI_ZBOOT`, dm-verity, EROFS, dm-crypt, overlayfs and efivarfs |

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
