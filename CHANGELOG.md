# Changelog

All notable changes to this project are written here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions follow
[Semantic Versioning](https://semver.org/). While the version is below 1.0 the project is not stable yet:
the interface (install and flash scripts, `.env` variables, device package) may change between releases.

Up to v0.1.14 the notes lived only in the message of each annotated tag; this file condenses them.
`git tag -n99 <tag>` shows the original text, including what was and was not tested.

## [Unreleased]

### Added
- `kernel/mainline/super-root-init` (`INIT=super-root scripts/mainline-ram-image`) and `kernel/mainline/bounded-loop.c`: stage 1 of the mainline root in super's approved region. It locks every other disk read-only, feeds the hardware watchdog, opens the region only through a loop bounded in offset and size, checks the label and switches to OpenRC; on failure it falls back to a USB shell with a dead man (#6).
- `docs/MAINLINE.md`, section 7: our own 7.3 build boots the same way, its modules load (UFS read-only, battery), and an unfed hardware watchdog (`qcom_wdt`, 30 s) resets the phone back to the server by itself, which answered SSH 42 s after the reset. U-Boot takes images up to 112 MiB (#6).
- `scripts/mainline-ram-image` and `kernel/mainline/ram-test-init`: a RAM-only test image for the mainline kernel, booted from the base's U-Boot with `fastboot boot`. Its initramfs locks every disk read-only, mounts nothing from them, offers a shell on `172.16.42.1:2323` over USB and reboots back to the server after 10 minutes unless someone logs in. `scripts/test-mainline-ram-image` checks those rules and packs fake inputs (#6).
- `scripts/build-mainline-kernel`: Linux 7.3 for this phone from kernel.org plus the base's 195 patches at a pinned commit of its repository, with both source trees checked by hash, its config plus `kernel/mainline/server.config` (the hardware watchdog `QCOM_WDT`) and his `hanoip_mmi_unit_info` module. Run end to end on 2026-11-07; the DTB comes out byte-identical to his (#6).
- `docs/MAINLINE.md`, section 7: the base's Linux 7.3 boots on this phone RAM-only through its U-Boot, with our own test initramfs that locks every disk read-only: USB network and serial, cpufreq, battery read in hold on the cable, UFS reads matching the backup, and back to the server with a reboot (#6).
- `docs/PITFALLS.md`: NetworkManager flushes a hand-set address on the USB interface during a RAM test, and U-Boot passes only the boot image's command line to the kernel.

### Changed
- Kernel direction: mainline is the goal again, starting from the mainline base (U-Boot plus Linux 7.3, RAM-only); the downstream 4.14 server stays on slot `b` meanwhile (`docs/STATUS.md`, section 5). The 2026-11-02 plan of a hardened 4.14 fork is on hold (#6).

### Fixed
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
