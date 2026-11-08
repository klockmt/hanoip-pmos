# Duranium on the hanoip: the plan

Hand-off for the work that follows the persistent mainline boot (2026-11-08). Read `AGENTS.md`, `docs/STATUS.md` and
`docs/MAINLINE.md` (sections 7 to 7e) first; `docs/PITFALLS.md` before touching U-Boot, the mainline root, the slots
or a flashing script. Steps are in order. Each one ends with a test (`scripts/test-*`), a commit on `beta` and, when
it touches the phone, evidence written into `docs/MAINLINE.md`.

## What the user decided

- **2026-11-07:** mainline is the goal; the mainline base may be used (his permission). Its repository
  has no license file: reference his files by URL at a pinned commit, never copy them into this public repository.
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
| U-Boot | `scripts/build-mainline-uboot` (sm7150-mainline/u-boot `tauchgang` at `f8c04469`, the base's diffs, tree `a38f1757`), already with the EFI loader, `bootefi`, the EFI boot manager, `blkmap` and FAT |
| Kernel | `scripts/build-mainline-kernel`: kernel.org 7.3 + the base's 195 patches at `ea0186ba`, trees `30fd2ef8` (v7.3) and `276fbce0` (patched), its config plus `kernel/mainline/server.config`; it already has `EFI_STUB`, `EFI_ZBOOT`, dm-verity, EROFS, dm-crypt, overlayfs and efivarfs |

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

`linux-motorola-hanoip-mainline`, next to the 4.14's `pmaports/device/downstream/linux-motorola-hanoip`, modelled on
Nura's `device/community/linux-postmarketos-qcom-sm7150/APKBUILD`:
- sources: `linux-7.3.tar.xz` from kernel.org, the base's 195 patches and `config-7.3-hanoip` by URL at commit
  `ea0186ba` (`raw.githubusercontent.com/the mainline base/<commit>/work/kernel-7.3/v7/...`), and
  `kernel/mainline/server.config` from this repository. Generate the source list from `patches.list` with a script;
  the checksums with `pmbootstrap checksum`.
- `prepare()`: `git am` the patches (one has a rename, none is binary) and check both trees, as
  `scripts/build-mainline-kernel` does; merge the config with `merge_config.sh` and `olddefconfig`.
- `build()`/`package()`: `Image`, modules and `qcom/sm7150-motorola-hanoip.dtb`; the out-of-tree
  `hanoip_mmi_unit_info` (`work/kernel-7.3/v7/extra/`, SMEM item 134, needed by the modem); install like the Nura
  package (`modules_install zinstall dtbs_install`, the hanoip DTB under `/boot/dtbs/qcom/`).
- Or package the tree that `scripts/build-mainline-kernel` already builds with `pmbootstrap build --envkernel`, as
  `scripts/build-image` does for the 4.14 (it needs the build in the tree's `.output`). Pick one and say why.
- Test: a `scripts/test-*` that the package pins the same commit and trees as `scripts/build-mainline-kernel`; build
  it and compare `Image`, the DTB and the module list with `$HANOIP_MAINLINE_DIR/out`.

### 2. The device package for mainline

- `device-motorola-hanoip-kernel-mainline` depends on the new kernel instead of `linux-postmarketos-qcom-sm7150`.
  `pmaports.py` takes the **first** kernel subpackage, so mainline goes first (the 4.14 keeps working through
  `pmbootstrap config kernel downstream`).
- Per-kernel `deviceinfo` overrides (`deviceinfo_<variable>_mainline`): `deviceinfo_dtb_mainline=
  "qcom/sm7150-motorola-hanoip"`, a mainline command line (no downstream arguments; keep `panic=5`, add the boot
  console), no Android boot image for the EFI path, and `deviceinfo_rootfs_image_sector_size="4096"` (UFS).
- Firmware: the vendor blobs (`adsp`, `modem`, `ipa_fws`, `wlanmdsp.mbn`, `bdwlan_hanoip_ipa.bin` as ath10k's
  `board.bin`, under `qcom/sm7150/motorola/hanoip/`) cannot be published. Build a firmware package only locally,
  from files taken off the phone into `$HANOIP_COLLECTION_DIR`, gitignored, never pushed.
- Test: extend `scripts/test-deviceinfo` for the mainline overrides.

### 3. systemd units for the phone's own pieces

Port what `kernel/mainline/root-overlay` does under OpenRC, each with the test it has now:
- the modem: EFS copied to RAM (`modemst1`, `modemst2`, `fsg_b`/`fsg_a`, `fsc`) and served read-only by
  `rmtfs -o /run/efs -r`, `tqftpserv`, modules in this order: `hanoip_mmi_unit_info`, `qcom_pd_mapper`, `qrtr_smd`,
  `qcom_q6v5_pas`, `ipa`, `ath10k_snoc`, recovery off, then `echo start` to the modem's remoteproc
  (`scripts/test-mainline-root` has the details);
- the charge limit (`hanoip-ml-charge`: 75-80 %, never over 4.30 V or 40 °C, through the base's
  `/sys/kernel/debug/hanoip-charge`);
- the guard: every block device read-only except what Duranium owns;
- the slot marking (`qbootctl -m <slot>` after `boot-complete.target`, the same exception, the same lent disks);
- the watchdog: systemd's `RuntimeWatchdogSec` replaces the stage-1 daemon (U-Boot leaves it running);
- Wi-Fi with the server's MAC, Tailscale's state, nftables (port 22 closed from the LAN), no `rtcsync` in chrony.

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
