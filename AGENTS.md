# hanoip-mainline

postmarketOS on the Moto G60 (hanoip, SM7150), run as a **headless server reached over SSH**. The repo is public. Reply to the user in Portuguese; commits, code and docs are in English.

Read `NOTES.md` (state, pitfalls, working rules) and `docs/STATUS.md` before touching anything. This file is the short version.

## Rules that always apply
- Run `scripts/check-repo` before every commit and push.
- Local paths and personal data live in `.env` (copy of `.env.example`). Scripts source `scripts/env.sh`; docs use `$HANOIP_*` variables. To resolve one, read `.env`.
- Never write an IMEI, serial, MAC, battery id, home path or the user name in any tracked file, test fixture, log or commit message. Device dumps and partition backups stay in `$HANOIP_BACKUP_DIR` / `$HANOIP_COLLECTION_DIR`, outside the repo.
- Scripts that change something accept `DRY_RUN=1`. Keep `README.md` honest about which were run end to end.
- Do not flash partitions beyond what the user asked for, never wipe or factory reset. Inspect the target and compare with `SHA256SUMS` first. `fastboot set_active b` + `reboot` is pre-authorized only to recover the A/B slot (see below).
- `pmbootstrap` asks for the `sudo` password and needs interactive input, so **the user runs it**: hand over ready-made commands and wait. It is not on PATH; use `$PMBOOTSTRAP`.
- When zeroing the in-use slot's `dtbo` for a test, restore the backup right afterwards, before `fastboot continue` or any reboot.
- Report test results as they are, failures included.

## Working over SSH, securely
- Use `scripts/hanoip-ssh '<cmd>'`. It uses the dedicated key from `.env` with `IdentitiesOnly` and `BatchMode`; `HANOIP_SSH_TTY=1` only when a password prompt is needed.
- `sudo` asks for a password, which the user allowed the agent to keep in `.env` (`HANOIP_SUDO_PASSWORD`, `HANOIP_ROOT_PASSWORD`). Run root commands with `scripts/hanoip-sudo '<cmd>'` (password over the ssh stdin, command as base64); never print or log the value. Do not make `sudo` passwordless again, and keep ssh on keys only (`hanoip-sshd.conf`).
- Never print, log or commit a key, the Telegram token or owner id. The bot secrets live only on the phone, in `/etc/hanoip/telegram.env`; `telegram.env` and `*.token` are gitignored.
- SSH is accepted only over USB (`usb*`) and Tailscale (`02_hanoip-ssh.nft`); do not undo that. Do not weaken the nftables firewall, open a port, enable password login or add a public-facing service without asking. Remember `51_usb_inet.nft` already allows everything coming in over USB.
- The Telegram bot runs fixed commands only. Text from a message must never be used to build a command.
- Before a risky change over SSH (network, firewall, boot), check you still have a way back: `flash-boot-ssh` saves the old `boot_b` first; otherwise the recovery is fastboot.
- Large transfers over the USB (NCM) network take it down until a reboot. Throttle them (`pv -L 1m`, as `scripts/push-firmware` does).

## Server setup
- The screen is off by design (`hanoip_splashfb.screen_off=1` from `scripts/mkbootimg-kernel`). It only comes back with a reboot on an image built without the parameter. Do not use `fb0/blank`.
- Services (OpenRC): `hanoip-qgd` (battery percentage, see below), `hanoip-charge-limit` (needs `hanoip-qgd`), `hanoip-bot`, `hanoip-thermal`, `hanoip-modem` (Wi-Fi), Tailscale. `qbootctl -m` runs from `/etc/local.d` (keep `local` in `default`).
- Pending: `systemd-tmpfiles`/`sysusers` cannot read their `.d` directories on 4.14 (`openat2`), which is harmless here (`NOTES.md`). Open work lives in the GitHub issues (roadmap #20).
- On the phone, `/etc/deviceinfo` sets `deviceinfo_flash_kernel_on_update="false"` until the vendor_boot boot failure of 2026-10-30 is understood (`NOTES.md`). Never flash `boot`/`vendor_boot` without a backup on the PC (`$HANOIP_BACKUP_DIR`).

## Bugs found, and what to remember
Every bug gets an automated test that fails before the fix and passes after (see below). They live in `scripts/test-*` and `scripts/check-repo` runs them all.

| Bug | Cause | Rule |
|---|---|---|
| Battery stuck at 100 % for days, charge limit never cut, battery died, PMIC lost SDAM | `qpnp-qg` does not compute SoC after boot; a userspace daemon must write it back | `hanoip-qgd` must keep running; the charge limit must not trust the percentage alone (stale file > 15 min or < 3.75 V resumes charging) |
| Percentage frozen at 100 % with the cable suspended | Charger keeps reporting `Full` while discharging | `Full` sets 100 % only when the batch shows no discharge (< 100 mA) or no samples |
| Gauge scale off | Profile table is the cell's scale, kernel maps 3.4 V to 0 % and 4.45 V to 100 % | Pull toward the OCV on the kernel's scale |
| SSH dropped, load 3-4, conntrack full (65,536) | Tailscale rewrote `resolv.conf` to its own MagicDNS and looped | `tailscale set --accept-dns=false`, then `resolvconf -u` |
| Phone gone after ~7 reboots | pmOS never marks the A/B slot successful; ABL makes slot b unbootable and flips to a | `qbootctl -m` at every boot (from `/etc/local.d`); recovery `fastboot set_active b` + `fastboot reboot` (check the slot flags first and say what was done) |
| nftables rules with `{ ... }` fail with "Not supported" | Set backends missing | Keep `NFT_SET_HASH/RBTREE/BITMAP` as `=y` (the kernel package installs no modules) |
| `losetup -P --sector-size 4096` makes no `loopNpX` | Loop does not rescan after `LOOP_SET_BLOCK_SIZE` | Keep kernel patch 0001 |
| ABL never ran the mainline kernel | ABL only runs header v3 and only `Image`/`Image.gz`, not EFI zboot | `EFI_ZBOOT` off; v3 `vendor_boot`; earlier v0 tests are invalid |
| `fastboot flash userdata` refused | Motorola fastboot | Write the rootfs through the Lineage recovery with `dd` and verify the hash |
| Every boot took ~3 min | Each missing firmware file waited 60 s | Keep `FW_LOADER_USER_HELPER_FALLBACK` off |
| `dtc` segfaults | The tree's own `dtc` built with clang 23 | Build with `DTC=/usr/bin/dtc` |
| Cycle counter lost (8 buckets) | A fully drained battery clears PMIC SDAM | Never let the battery die; if it happens, `scripts/restore-cycle-counts` (needs kernel patch 0007, which makes `bms/cycle_counts` writable; keep it) |
| Wi-Fi firmware crashed ~0.7 s after WLFW connected (`EX:kernel:0x0:tmr_slave1`) | Modem reads SMEM item 134, created by `mmi_info`, which was off | Keep `CONFIG_MMI_INFO=y` and `CONFIG_MMI_ANNOTATE=y` (`scripts/test-kernel-config`) |
| Kernel upgrade would flash an old cmdline | Device subpackage `-kernel-downstream` (owner of `deviceinfo`) left at r0 | Install the main device apk and its subpackage together |
| Phone hung at boot after `boot-deploy` regenerated `vendor_boot` | Unconfirmed; only the cmdline changed (`console=tty0` instead of `console=ttyMSM0,...`) | Back up to the PC first; test a new `vendor_boot` before trusting package updates |
| `hanoip-qgd` and `hanoip-bot` missing on a fresh image | `python3` was not in the device package's `depends`; it ran only because the phone already had it | Declare the interpreter of every shipped script (`scripts/test-apkbuild-depends`) |
| `abuild` called `deviceinfo` corrupted | `sha512sums` was not in the order of `source=`; abuild pairs them by position | Keep the same order (`scripts/test-apkbuild-sums`; `update-apkbuild-sums` keeps the lines in place) |
| Package build died before abuild, installing `firmware-qcom-sm7150` in the x86_64 chroot | `rust` in `makedepends` makes pmbootstrap use crossdirect, which installs every dependency natively | Keep `options=!pmb:crossdirect` on a package that builds Rust (checked by `scripts/test-apkbuild-depends`) |
| `flash-boot-ssh` stopped at its first root command, and pushed 64 MB with no rate limit | Plain `sudo` over the BatchMode login cannot ask for the password; no `pv` | Root commands go through `hanoip-sudo`, transfers through `pv -L 1m` (`scripts/test-flash-boot-ssh`) |
| `postmarketos-zram-swap` never started | No `CRYPTO_ZSTD`, no `/lib/modules` for `modprobe zram`, and `vm.swappiness=180` rejected over 100 | Keep `CRYPTO_ZSTD=y`, the module tree in the kernel package and patch 0008 (`scripts/test-kernel-config`); `scripts/push-modules` for a phone that already runs the kernel |
| A boot's log missing from `logbookd.db` | logbookd writes only on a clean shutdown; the raw `reboot()` syscall skips it | Use `reboot` when the log matters |
| "Not charging" with `usb/online=0` read as no power | In the limiter's hold the input still feeds the phone | Check `usb/input_current_now`, not `usb/online` |

## Tests for every bug
- Add the regression test before or with the fix. Prefer extending the matching `scripts/test-*` (they run without the phone); create a new `scripts/test-<thing>` in the same style when none fits.
- Run the test and show it failing on the old behavior, then passing.
- Commit the test with the fix, or right before it.

## Commits
Small, one per specific change, `type(scope): summary` with a body saying why. Never mention AI, no trailers. Author is always the git config identity; contributors on GitHub must show only `klockmt` (`gh api repos/klockmt/hanoip-pmos/contributors` before pushing).

Push to the `beta` branch first. Move commits to `main` only after they were tested and the user approved; never push straight to `main`.
