# Changelog

All notable changes to this project are written here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions follow
[Semantic Versioning](https://semver.org/). While the version is below 1.0 the project is not stable yet:
the interface (install and flash scripts, `.env` variables, device package) may change between releases.

Up to v0.1.14 the notes lived only in the message of each annotated tag; this file condenses them.
`git tag -n99 <tag>` shows the original text, including what was and was not tested.

## [Unreleased]

### Added
- `scripts/restore-server keys` adds the login user's other authorized public keys from the 4.14 backup (the Termux phone, another PC) to the ones the image has, through sudo, without replacing the file and without ever sending a private key. The new image only had the key of the PC that built it. `scripts/test-restore-server` checks it.
- `scripts/restore-server` (with `DRY_RUN=1`): after a write of `userdata`, puts the 4.14 server's data back from its backup over the USB as root: the Wi-Fi profiles, Tailscale's node state, AdGuard Home, Samba (config and password database), Syncthing (identity, folders, and the owner of its NAS folder), the bot's secrets and the cycle-count marker, with their owners and modes, throttled to 1 MB/s, then restarts the services. `restore-server password` sets the login user's sudo password from `HANOIP_SUDO_PASSWORD` (a hash over the ssh stdin, never printed) and `restore-server harden` turns root login off and removes root's key, only after `scripts/hanoip-sudo` works. `scripts/test-restore-server` runs it against a simulated phone and fails when an unrelated file is sent, a secret is printed, or harden runs without sudo.
- The Duranium preset switches off what a headless server does not need, as the 4.14 server had it: `avahi-daemon` (mDNS on 5353), `rtkit-daemon`, `systemd-oomd` and `getty@` (the screen is off). `systemd-resolved` stops answering LLMNR (5355) and mDNS but keeps its stub for the phone's own lookups, which leaves port 53 to AdGuard Home on loopback. Core files are not stored, the journal is limited to 200 MB, and `hanoip-screen-off.service` switches the backlight off at boot. `scripts/test-duranium-boot` checks each one.
- The mainline subpackage depends on what the 4.14 server's package list had that the Duranium base does not: `btop`, `btrfs-progs` (+ `-extra`), `debootstrap`, `doas`, `fastfetch`, `hwloc-tools` (`lstopo`), `iperf3`, `libdrm-tests`, `memtester`, `smartmontools`, `stress-ng`, `sudo-rs`, `tar`, `wget`, `tzdata`, `openssl`, `gcompat`, `libstdc++`, `musl-utils`, plus the three services' packages. The OpenRC and postmarketOS 4.14 pieces stay out. `scripts/test-duranium-boot` checks the list.
- `hanoip-thermal.service` and `hanoip-led.service` in the mainline subpackage, enabled by the preset together with `hanoip-bot.service`: the heat protection (pause charging at 40 °C, cap the CPU at 45 °C, power off at 55 °C) and the power LED that the 4.14 server ran under OpenRC. Stopping the heat protection lifts the paused charge and the CPU cap. `scripts/test-duranium-boot` checks them.
- The 4.14 server's services as systemd units in `device-motorola-hanoip-mainline`: `adguardhome.service`, `smb.service` and `syncthing.service` (Alpine ships only OpenRC scripts for them), each off until its restored data is there (AdGuard's config, Syncthing's identity) and, for Samba and Syncthing, off while the NAS disk is away; the NAS partitions as `mnt-hd\x2da.mount` and `mnt-hd\x2db.mount` (`WantedBy`, so a missing disk never stops the boot); and sysusers entries: `hdshare` keeps the 1000 it had, because the Samba share's files on the NAS disk belong to that number, and `syncthing` and `adguardhome` take the IDs the image gives them (the 4.14's 105 and 106 are chrony and dnsmasq in the image). The preset enables all of them. `scripts/test-duranium-boot` checks the wiring and fails when a condition or a mount's `WantedBy` is removed.
- `scripts/backup-server`: step 7 of `docs/DURANIUM.md`, a file-level backup of the 4.14 server before `userdata` is written. On the phone GNU tar archives `/` (one file system, without the swap file) and `/boot`, with the package list and the OpenRC runlevels, into a root-only folder; each file comes to `$HANOIP_BACKUP_DIR` at 1 MB/s, resuming a copy that dropped, is checked against the phone's hash, and one file is restored and compared before the phone's copy is removed. The archives hold the server's secrets and stay mode 600. `scripts/test-backup-server` runs it on a fake root with a dropped and a damaged copy; `DRY_RUN=1` prints the plan. Run on the phone on 2026-11-08: 1.7 GB, hashes equal on both sides, `/etc/apk/world` restored and equal.
- `linux-motorola-hanoip-mainline`: a pmaports package of our Linux 7.3, the first step of `docs/DURANIUM.md`. It builds kernel.org's `linux-7.3.tar.xz` plus the 195 patches with GCC, checks both source trees by hash as `scripts/build-mainline-kernel` does (`30fd2ef8`, `276fbce0`), and installs what Duranium's image build takes: the EFI kernel (`EFI_ZBOOT`) as `/boot/vmlinuz`, the hanoip DTB under `/boot/dtbs/qcom/` and the modules, `hanoip_mmi_unit_info` included, in `/usr/lib/modules`. Its patches and configs are copies of `kernel/mainline`, kept identical by `scripts/test-mainline-kernel-package`, which also checks the pinned trees and the kernel release. Not built yet.
- `device-motorola-hanoip-mainline`: the mainline root's own pieces as a pmaports subpackage (the modem and Wi-Fi service, the charge limit, the guard, slot marking, the boot marker, the dead man, the battery reboot, the USB network, the module blacklist, NetworkManager's configuration and chrony's dispatcher), so Duranium's image build takes them from a package instead of the overlay. They are the OpenRC scripts of `kernel/mainline/root-overlay` as copies, kept identical by `scripts/test-device-mainline-subpackage`, which also checks that git holds every source (they had been left out of the commit that added the package, so only this PC could build it) and that `amove` keeps them out of the 4.14 server's package. systemd units for Duranium are still open.
- `scripts/check-repo` refuses the names listed in `HANOIP_PRIVATE_NAMES` (`.env`, any case, whole words), people who asked not to be named, and reports only file and line; `scripts/test-check-repo` covers it.
- `scripts/check-repo` refuses an `APKBUILD` source that git does not track: a copy that sits only in the working tree builds on this PC and fails on a clean clone, which is how the mainline subpackage's thirteen files were lost; `scripts/test-check-repo` covers a tracked and an untracked source.
- `docs/DURANIUM.md`: the plan for Duranium as the server, step by step with what each needs and how it is checked, the user's decisions, the state it starts from, and the rules and lessons that apply; `AGENTS.md` points to it.
- `docs/MAINLINE.md`, section 7e: what Nura's Duranium needs on this phone (research): a pmaports device package with the mainline kernel, systemd, EFI (our U-Boot already has it, and `blkmap`), and 16 GB, flashed into `userdata` by design; its LUKS2 unlocks by itself with a blank passphrase; no image exists for SM7150.
- Slot a marks itself successful after a complete boot from it, so the ABL keeps it as a second working slot instead of using up its retries (approved by the user): `98-hanoip-ml-slot.start` reads the ABL's slot from `u-boot,abl-bootargs` in the DT (`hanoip-ml-abl-slot`, only `androidboot.slot_suffix`), and with sshd up runs `qbootctl -m a` while the guard lends it only the whole disks that hold slot a's partitions (LUN 4 and LUN 2). A boot from the PC marks nothing, and the 15-minute dead man no longer reboots a boot from slot a. On the phone the ABL then said `slot-successful:_a: yes`, and only A/B bits changed in those two GPTs.
- `docs/MAINLINE.md`, section 7d: U-Boot in `boot_a` boots the mainline root with no PC (started from RAM by the ABL), how the hangs on the way were found, and what is still open: the server's services on mainline, the cause of one root stall, a slow LUN 0.
- `scripts/mainline-make-root` installs `hanoip-reboot-bootloader` from the server into the mainline root, so `scripts/reboot-bootloader` also works from mainline (it had been copied there by hand). Used on the phone on 2026-11-08 to go from mainline to the ABL's fastboot.
- Stage 1 of the mainline root reboots a root that has not finished its boot (local.d's last step writes `/run/hanoip-ml-up`) 5 minutes after `switch_root`, and leaves the OpenRC services and the kernel log in `/var/log/hanoip-ml-deadline.log` for the server to read: a root stalled before `local.d` had sat 10 minutes with no network while the watchdog was still fed. A good boot is left alone (checked past 5 minutes on the phone); the reboot path ran in a namespace, not on the phone.
- The hardware watchdog of the persistent mainline boot runs from U-Boot into the kernel: nothing in U-Boot feeds it and the handoff no longer stops it (`kernel/mainline/uboot/0001-leave-the-watchdog-running-for-the-kernel.patch`), so a hang in U-Boot, or in the kernel before stage 1, resets the phone. U-Boot programs 30 s and it bit about 50 s after U-Boot started; the kernel took it over in every boot of 2026-11-08. The Volume Down menu stops it first (not tried on the phone).
- `scripts/build-mainline-uboot` and `scripts/mainline-boot-a-image`: the persistent mainline boot. Our U-Boot (sm7150-mainline/u-boot pinned, plus the board diffs in `kernel/mainline/uboot/base`, checked against the tree of the first validated build) with UFS and an environment that finds `boot_a` by name, reads the kernel image 16 MiB into it and boots it with U-Boot's DTB, and resets when that fails, so the ABL counts a failed boot of slot a; it still cannot flash. `bootdelay=-2`, since a key already pending at autoboot left `bootdelay=0` at the prompt for good, and `scsi dev` before `scsi read`, which reads the current device (LUN 0) and not boot_a's. Booted from RAM by the ABL on 2026-11-08, it read `boot_a` (CRC equal to the image) and booted the mainline root to SSH over USB in 84 s; booted as slot a itself, with no PC, SSH came at 113 s (slot b, the server, active again since).
- `docs/MAINLINE.md`, section 7c: the mainline root boots by itself to the modem, Wi-Fi, Tailscale (SSH over the tailnet in 133 s), nftables, chrony and the charge limit, with only super's region writable. Still missing: the server's own services and a boot without the PC (#6).
- `hanoip-ml-charge` (mainline root): holds the battery at 75-80 % through the kernel's opt-in charging (`/sys/kernel/debug/hanoip-charge`), never charges over 4.30 V or 40 °C and switches charging off when it stops; `scripts/test-mainline-charge` runs it against a fake gauge (#6).
- The mainline root brings up the modem and Wi-Fi and joins the tailnet: vendor firmware where the mainline DT asks for it, `hanoip-ml-modem` (EFS copied to RAM and served read-only by `rmtfs -r`, `tqftpserv`, SMEM item 134 with the generic values of the 4.14 command line, then the remoteprocs, IPA and ath10k), and from the server the Wi-Fi profiles and MAC, the Tailscale node and the nftables rules. udev does not load the modem, Wi-Fi, display, touch, video, camera or Bluetooth drivers on its own. The guards moved to `kernel/mainline/root-overlay`; `scripts/test-mainline-root` covers the root's rules (#6).
- `docs/MAINLINE.md`, section 7b: the mainline root in super's free region boots to OpenRC and SSH over USB (73 s), with only `super`, its disk and the root loop writable. No firmware in it yet, so no Wi-Fi or modem (#6).
- `scripts/mainline-make-root`: writes the mainline development root into super's approved region from the running 4.14 server (Alpine edge plus the pmOS repository, OpenRC, SSH with the server's user and host keys, our 7.3 modules, and guards that lock the other disks, reboot to the server after 15 minutes unless someone logs in and when the battery falls under 50 %). It hashes super outside the region before and after and fails if anything there changed; `DRY_RUN=1` prints the plan (#6).
- `kernel/mainline/super-root-init` (`INIT=super-root scripts/mainline-ram-image`) and `kernel/mainline/bounded-loop.c`: stage 1 of the mainline root in super's approved region. It locks every other disk read-only, feeds the hardware watchdog, opens the region only through a loop bounded in offset and size, checks the label and switches to OpenRC; on failure it falls back to a USB shell with a dead man (#6).
- `docs/MAINLINE.md`, section 7: our own 7.3 build boots the same way, its modules load (UFS read-only, battery), and an unfed hardware watchdog (`qcom_wdt`, 30 s) resets the phone back to the server by itself, which answered SSH 42 s after the reset. U-Boot takes images up to 112 MiB (#6).
- `scripts/mainline-ram-image` and `kernel/mainline/ram-test-init`: a RAM-only test image for the mainline kernel, booted from our U-Boot with `fastboot boot`. Its initramfs locks every disk read-only, mounts nothing from them, offers a shell on `172.16.42.1:2323` over USB and reboots back to the server after 10 minutes unless someone logs in. `scripts/test-mainline-ram-image` checks those rules and packs fake inputs (#6).
- `scripts/build-mainline-kernel`: Linux 7.3 for this phone from kernel.org plus the 195 patches in `kernel/mainline/patches`, with both source trees checked by hash, `kernel/mainline/config-7.3-hanoip` plus `kernel/mainline/server.config` (the hardware watchdog `QCOM_WDT`) and the `hanoip_mmi_unit_info` module (`kernel/mainline/extra`). Run end to end on 2026-11-07; the DTB comes out byte-identical to the first validated build's (#6).
- `docs/MAINLINE.md`, section 7: Linux 7.3 boots on this phone RAM-only through U-Boot, with our own test initramfs that locks every disk read-only: USB network and serial, cpufreq, battery read in hold on the cable, UFS reads matching the backup, and back to the server with a reboot (#6).
- `docs/PITFALLS.md`: NetworkManager flushes a hand-set address on the USB interface during a RAM test, and U-Boot passes only the boot image's command line to the kernel.

### Changed
- The persistent mainline U-Boot (`hanoip-server.env`) tries Duranium in `userdata` first from either slot and goes on to the Alpine root in `super` only when that fails, so both Android slots can be the server and the Alpine root is the last resort. The Volume Down menu gains "Boot the Alpine rescue root (super)" so it can still be started by hand. Not flashed yet: `boot_a` and `boot_b` still hold the previous U-Boot. `scripts/test-mainline-uboot` checks the environment.
- The mainline kernel's 195 patches, its config (`config-7.3-hanoip`) and the out-of-tree `hanoip_mmi_unit_info` module, and U-Boot's board diffs and base config, now live in `kernel/mainline` (`patches/`, `extra/`, `uboot/base/`). `scripts/build-mainline-kernel` and `scripts/build-mainline-uboot` fetch only kernel.org's tarball and the upstream U-Boot commit, and check the same source trees as before (`276fbce0`, `a38f1757`); rebuilt from them the DTB is byte-identical. `scripts/check-repo` accepts the upstream authors' e-mails in those patches.
- Plan (`docs/STATUS.md`, section 5): Nura's Duranium becomes the server, in all of `userdata` in place of the 4.14, decided by the user on 2026-11-08; the steps before `userdata` is written, and slot b as a rescue system with the Alpine root in super.
- The phone runs device package `0-r41`, built with `pmbootstrap` and installed with `apk add` together with its `-kernel-downstream` and `-modem` subpackages. That also installs the 0.2.1 LED fix, which had only been checked with the script in place. The update re-flashed `boot_b` and `vendor_boot_b` with images byte-identical to the previous ones (partition hashes equal to the 2026-11-07 backup), and the phone booted with no PSI hang (#31).
- Kernel direction: mainline is the goal again, starting from a U-Boot and a Linux 7.3 tree for this phone (RAM-only at first); the downstream 4.14 server stays on slot `b` meanwhile (`docs/STATUS.md`, section 5). The 2026-11-02 plan of a hardened 4.14 fork is on hold (#6).

### Fixed
- `scripts/restore-server` gives the restored files their owners by name: the 4.14 backup records owners as numbers only and the new system numbers its users differently (the 4.14's syncthing 105 is `adguardhome` in the image, 109 is `chrony`), so the first restore on the phone left Syncthing's, AdGuard's and Tailscale's files owned by the wrong users. It reads the names from the backup's own `passwd` and `group`. `scripts/test-restore-server` now uses an archive with numbers only and fails on the old script.
- `scripts/duranium-image` copies the vendor firmware's `*.jsn` protection-domain maps next to the remoteprocs' firmware. `hanoip-pd-mapper` read none of them (`no pd maps available`), failed on every boot and left the system `degraded`. `scripts/test-duranium-boot` checks the copy.
- `scripts/duranium-image` creates the login user as 1001, not 1000: `hdshare`, the user of the Samba share, is 1000 on the 4.14 server and owns the files on the NAS disk, so the two would have collided once the share's user was created. `scripts/test-duranium-boot` checks it.
- The Duranium image gets a PAM service for `sudo` (`/etc/pam.d/sudo`, from the mainline subpackage). sudo-rs asks PAM for that service; the running image had `base-auth` and friends in `/usr/lib/pam.d` and no `sudo` file, so sudo would fall back to the generic `other` service (not tried on the phone: the password step was refused). `scripts/hanoip-sudo` and the end of root login depend on `sudo` working. `scripts/test-duranium-boot` checks the file and its install.
- `hanoip-bot.service` stays off until `/etc/hanoip/telegram.env` exists (`ConditionPathExists`): the OpenRC script had that guard, the unit did not, and with `Restart=always` a fresh Duranium would have panicked every 5 s. `scripts/test-bot` checks the unit and its copy in the root overlay.
- `hanoip-status` (and the bot's `/status`) works on the mainline kernel: `qcom_qg` and the PM6150 charger replace `battery` and `usb`, the GPU is read from devfreq, the sensors are the `*-thermal` zones, and a missing figure prints `n/a` instead of breaking the line. `scripts/test-status` covers it.
- `hanoip-led` works on the mainline kernel: the LED is `white:status`, the cable shows on the PM6150 charger supply, and the kernel has no `timer` trigger, so the battery flash uses `oneshot`. `scripts/test-led` covers both kernels.
- `hanoip-thermal` reads the mainline gauge (`qcom_qg`) when the 4.14's `battery` supply is not there. Without it the heat protection of a Duranium boot (pause at 40 °C, CPU cap at 45 °C, power off at 55 °C) read nothing and never acted. `scripts/test-thermal` fails on the old script.
- The Duranium image enables `hanoip-usb-gadget`, `hanoip-health`, `hanoip-charge` and `hanoip-modem` with a systemd preset (`device-motorola-hanoip` `pkgrel=49`). Links in `/etc` were removed by mkosi's `systemctl preset-all`, so the first Duranium boot that reached userspace had no `usb0` and no SSH. `scripts/test-duranium-boot` checks the preset.
- The mainline kernel's framebuffer (patch 0121, simplefb) takes the linear mapping only where the kernel maps that RAM. The first Duranium boot through U-Boot's EFI oopsed in `fbcon` at 0.46 s and the ABL fell back to slot b. `linux-motorola-hanoip-mainline` is `pkgrel=1`; `scripts/test-mainline-kernel-package` fails on the old patch and passes on the new one.
- `scripts/duranium-write-userdata` no longer refuses to write because of the loop device the mainline rescue root runs from (a loop over `super`): only a loop over `userdata` stops it. `scripts/test-duranium-write` covers both cases.
- The Alpine mainline root's NetworkManager dispatcher restarts chronyd with `rc-service` again: a change meant for Duranium had made it call `systemctl`, which that OpenRC root does not have, so after the next `scripts/mainline-make-root` the clock would have stayed in 1974 and Tailscale logged out (it never reached the phone). `scripts/test-mainline-root` now runs the dispatcher with stand-ins and no other command on `PATH`, instead of accepting either command in the file.
- `device-motorola-hanoip-mainline` installs chrony's NetworkManager dispatcher executable (0644 became 0755): NetworkManager only runs a dispatcher script it may execute, so on the mainline root the clock was never set once the Wi-Fi came up. `scripts/test-device-mainline-subpackage` now checks the mode each destination needs, not only its path.
- `scripts/check-repo` skips a downloaded `.tar.xz` source as it does a `.tar.gz`, instead of reporting it missing (kernel.org ships `linux-7.3.tar.xz`); `scripts/test-check-repo` covers both and that a missing local file is still reported.
- The mainline root no longer floods the kernel log with "disallowed SPMI write" (about 30 a second, which pushed the whole boot out of `dmesg`): chrony's `rtcsync` made the kernel retry writing the PMIC RTC, which Linux may not write. `scripts/mainline-make-root` drops it; the next boot had none and kept its whole boot log.
- `scripts/mainline-make-root` runs `e2fsck -p` on the region before mounting it: after an unclean mainline shutdown the orphan file holds inodes, and the 4.14 kernel refused to mount the root read-write ("unsupported optional features (10000)"). Run on the phone: it replayed the journal and the root was written, super unchanged outside the region.
- The mainline root sets its clock: the PMIC RTC reads 1974, and chronyd, started before the Wi-Fi, had no source, which also kept Tailscale logged out (TLS). `swclock` replaces `hwclock` and a NetworkManager dispatcher script restarts chronyd when a connection comes up; `scripts/test-mainline-root` covers both (#6).
- `scripts/env.sh` no longer prints the values of `.env` when a script runs under `sh -x`: the trace of its `eval` showed every value, passwords included. `scripts/test-env-xtrace` covers it.
- elogind no longer hangs a CPU on the 4.14 kernel: its memory-pressure trigger corrupted a list in `psi_trigger_create` on three boots in a row, leaving the server with slow SSH logins and a `reboot` that hangs. The device package (`pkgrel` 41) overrides elogind's D-Bus service file to start it with `MEMORY_PRESSURE_WATCH=/dev/null`; `scripts/test-elogind` covers it (#31).

## [0.2.2] - 2026-11-07

The downstream 4.14 server as it runs on the phone, tagged before the mainline work starts on `beta`.

### Changed
- `docs/HOMELAB.md` documents AdGuard Home, installed by hand as a secondary DNS: the config written by hand, the `bootstrap_dns` list that makes it refuse to start, and what is still open (#25). It listens on 127.0.0.1 only, so nothing new is reachable from the LAN or the tailnet.

### Tested
- `scripts/check-repo` passes. On the phone (kernel 4.14.357, device package `0-r39`): `nslookup example.com 127.0.0.1` resolves through AdGuard Home, and `netstat -ltnu` shows it on `127.0.0.1:53` (TCP and UDP) and `127.0.0.1:3000` only.

### Known issues
- AdGuard Home is not in the device package and does not serve the LAN yet (#25).
- `/proc/config.gz` on the phone does not describe the running kernel: it shows `BTRFS_FS` and `WATCHDOG` off and has no `MMI_INFO` line, while btrfs is in `/proc/filesystems`, `/dev/watchdog` exists and the Wi-Fi works, which needs `MMI_INFO`. Do not check kernel options with it.
- The docs describe only `/dev/watchdog` (a device with no parent in sysfs). The kernel also runs the downstream hardware watchdog driver `msm_watchdog` (`QCOM_WATCHDOG_V2`), bound to the APSS block `17c10000.qcom,wdt`, which the docs do not mention; what it does on a kernel hang has not been tested.
- The phone keeps sourcing VBUS to a USB hub and drains the battery (#30).
- Open from 0.2.1: the package was not rebuilt or installed from `pkgrel` 40 on the phone (it runs `0-r39`); the USB hub re-enumerates now and then and the mounted disk goes read-only until remounted (#28).

## [0.2.1] - 2026-11-04

### Changed
- The Samba share is confirmed working from Nautilus on the PC over the tailnet (`docs/STATUS.md`); large copies and a reboot with the disk are still open (#27).
- `docs/STATUS.md` and `NOTES.md` record that SSH access (USB cable and tailnet only) was checked again after a clean reboot (#23).

### Fixed
- `hanoip-led`: the front LED now stays lit while a cable is in. Solid was a 600000 ms timer, which left the LED dark with USB power present; it is now a neutral trigger plus a direct brightness write (`docs/PITFALLS.md`). `scripts/test-led` covers it and passes.
- `device-motorola-hanoip` `pkgrel` 40: the LED change shipped without a bump, so a package update would have kept the old script.

### Tested
- `scripts/check-repo` passes. On the phone: the LED is solid with the cable in, flashes when it is pulled and is solid again when it is put back (checked by eye); after a clean `reboot` the three `02_hanoip-ssh.nft` rules load ahead of pmOS's `accept SSH`, SSH answers over USB and the tailnet and port 22 is closed from the LAN.

### Known issues
- The package was not rebuilt or installed from `pkgrel` 40 on the phone; the LED fix was checked with the script in place.
- Before a saved Wi-Fi profile existed the phone stayed offline after a boot; with the profile it reconnects by itself.
- Open from 0.2.0: the USB hub re-enumerates now and then and the mounted disk goes read-only until remounted (`docs/PITFALLS.md`).

## [0.2.0] - 2026-11-04

### Added
- `03_hanoip-samba.nft`: Samba (139/445) is accepted on the tailnet only and dropped on every other interface, USB included.
- `docs/HOMELAB.md`: how the NAS disk is shared (Syncthing on `hd-a`, a Samba share on `hd-b`).
- `CHANGELOG.md`, condensed from the v0.1.x tag messages.

### Changed
- The NAS disk's two partitions were converted away from `block-group-tree` and mount read-write on the phone.
- `AGENTS.md` keeps only the phone-specific rules; the generic working rules moved to the global instructions.

### Known issues
- The USB hub re-enumerates now and then and the mounted disk goes read-only until remounted (`docs/PITFALLS.md`).
- Not tested yet: the share and the Syncthing folder from a real client, a reboot with the disk attached.

## [0.1.14] - 2026-11-02

### Added
- `hanoip-led`: the front LED is solid while the USB supply is present and flashes (0.5 s on, 3 s off) on the battery.

### Fixed
- The LED driver rejects `none` written to `trigger`, so only the timer trigger is used (`docs/PITFALLS.md`).

### Known issues
- Not tested: a real cable pull, a reboot with the service enabled, the package build that ships it.

## [0.1.13] - 2026-11-02

### Added
- `hanoip-watchdog` (#17): feeds `/dev/watchdog` and stops on purpose when the gateway is silent for 5 minutes with no USB link, so the phone resets.
- `scripts/test-throttle` fails when a script feeds data into `hanoip-ssh` without `pv -L` (#2).

### Changed
- The README works on a clean machine (#7): `pmbootstrap` comes from its 3.12.1 tag and needs Python 3.11, `check-host` pins that version.
- `hanoip-bot` stays off when `telegram.env` is missing.

### Fixed
- The `vendor_boot` hang (#21) was `console=tty0` first in the kernel command line. `deviceinfo` now starts with `console=ttyMSM0,115200n8`, and package updates flash `boot` and `vendor_boot` again.

### Known issues
- Not tested: a real router outage, off-mode charge, a power cut; flashing the clean-VM image on a phone.

## [0.1.12] - 2026-11-02

### Added
- `hanoip-status` shows a Warning line when `/etc/resolv.conf` uses Tailscale's DNS.
- A boot script turns the Tailscale DNS off when the saved preference has it on.

### Known issues
- The correcting case of the boot script was not run on the phone.

## [0.1.11] - 2026-11-02

### Added
- Telegram `/notes` lists every bot command.
- The device package ships Tailscale's firewall file (#11): UDP 41641 is accepted only on `eth*` and `wlan*`.

## [0.1.10] - 2026-11-01

### Added
- The modem and Wi-Fi userspace is a package, `device-motorola-hanoip-modem` (#10).
- `docs/PITFALLS.md`, `docs/HOMELAB.md`, `docs/MAINLINE.md`, `CONTRIBUTING.md` and issue forms (#15).

### Changed
- `hanoip-bot` is written in Rust (#22); the device package depends on `curl` instead of `python3`.

### Fixed
- Kernel modules load again (patch 0010, #24): the vendor `load_module()` forced flags that made every module fail with `-ENOKEY`, then `-ENOEXEC`.

### Known issues
- #1 was closed without the 24 h Wi-Fi hold, and #22 without the bot commands checked on Telegram.

## [0.1.9] - 2026-11-01

### Added
- btrfs is built into the kernel (patch 0009 fixes the build).
- Container support: user, PID and network namespaces, `pids` and `devices` cgroups, macvlan, `br_netfilter` (#4). Use `crun`; the `runc` of Alpine edge panics on 4.14.
- `/dev/watchdog` (`WATCHDOG` and `SOFT_WATCHDOG`).

### Known issues
- Not tested: btrfs on the NAS disk itself (#19).
- Not supported: WireGuard (not in 4.14), a hardware watchdog, `CFS_BANDWIDTH`, swarm networking.

## [0.1.8] - 2026-11-01

### Added
- `scripts/push-modules` copies the module tree to a phone that already runs the kernel.

### Fixed
- zram swap works (#8): `CRYPTO_ZSTD` on, the module tree installed in the kernel package, and patch 0008 raises the `vm.swappiness` limit to 200.

### Known issues
- Not checked: the swap under real memory pressure.

## [0.1.7] - 2026-11-01

### Changed
- One license, GPL-2.0-only, everywhere (#14). `LICENSE` is the standard text, the scope statement moved to `NOTICE`.

### Fixed
- `systemd-tmpfiles` and `systemd-sysusers` failures on 4.14 (`openat2`) checked on the phone and documented as harmless (#16).

## [0.1.6] - 2026-11-01

### Added
- `check-repo` flags a 15-digit number and the phone login user from `.env` (#13).
- `DRY_RUN=1` for `push-firmware`, `reboot-bootloader`, `mount-vendor` and `update-apkbuild-sums` (#12).

### Fixed
- `flash-boot-ssh` runs its root commands through `hanoip-sudo` and limits transfers to 1 MB/s with `pv` (#9).

## [0.1.5] - 2026-11-01

### Added
- Kernel patch 0007 makes `bms/cycle_counts` writable; `scripts/restore-cycle-counts` writes the saved buckets once (#5).

### Changed
- `/status` shows the kernel's cycle count as it is; the temporary offset is gone.

### Known issues
- Not checked: a charge session ending after the restore.

## [0.1.4] - 2026-11-01

### Added
- `02_hanoip-ssh.nft`: SSH is accepted only over USB and Tailscale.
- `/status` shows the gateway ping and the 1.1.1.1 ping.

### Known issues
- Not checked: the USB path and a reboot with the file in place (#23).

## [0.1.3] - 2026-11-01

### Added
- `/status -h` and `hanoip-status -h` explain each line. The GPU line is new.

### Changed
- `hanoip-qgd` (the battery gauge daemon) is written in Rust, using 1.0 MB of RAM against 7.7 MB.
- `/status` and `hanoip-status` print one labelled line per item.

### Fixed
- `python3` was missing from the device package's dependencies, `sha512sums` was out of `source=` order, and the package builds under qemu, not crossdirect.

## [0.1.2] - 2026-10-31

### Added
- Wi-Fi (WCN3990): 2.4 and 5 GHz, WPA3, back after a reboot. The modem needed `CONFIG_MMI_INFO`.
- Server use over Ethernet (USB adapter) or Wi-Fi.

### Changed
- The charge limit holds 81-82 % in bypass after charging between 60 and 80 %.
- CPU governor `schedutil` instead of `performance`.

## [0.1.1] - 2026-10-26

### Added
- Charge limit: the battery stays between 60 and 80 %; every ten cycles the next charge goes to 100 % once.
- `hanoip-status`: latency, network rate, battery, RAM, disk, CPU and temperatures in one command.

## [0.1.0] - 2026-10-26

### Added
- First release: a working phone with boot, screen, kernel and device package only.

[Unreleased]: https://github.com/klockmt/hanoip-pmos/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/klockmt/hanoip-pmos/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/klockmt/hanoip-pmos/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.14...v0.2.0
[0.1.14]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/klockmt/hanoip-pmos/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/klockmt/hanoip-pmos/releases/tag/v0.1.0
