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
- Do not weaken the nftables firewall, open a port, enable password login or add a public-facing service without asking. Remember `51_usb_inet.nft` already allows everything coming in over USB.
- The Telegram bot runs fixed commands only. Text from a message must never be used to build a command.
- Before a risky change over SSH (network, firewall, boot), check you still have a way back: `flash-boot-ssh` saves the old `boot_b` first; otherwise the recovery is fastboot.
- Large transfers over the USB (NCM) network take it down until a reboot. Throttle them (`pv -L 1m`, as `scripts/push-firmware` does).

## Server setup
- The screen is off by design (`hanoip_splashfb.screen_off=1` from `scripts/mkbootimg-kernel`). It only comes back with a reboot on an image built without the parameter. Do not use `fb0/blank`.
- Services (OpenRC): `hanoip-qgd` (battery percentage, see below), `hanoip-charge-limit` (needs `hanoip-qgd`), `hanoip-bot`, `qbootctl`, Tailscale.
- Pending: `zram`; `systemd-tmpfiles`/`sysusers` fail on 4.14 (`openat2`); Wi-Fi (firmware crashes ~0.7 s after connecting).
- Pending later: patch `qpnp-qg` so `bms/cycle_counts` is writable and restore the lost buckets (`NOTES.md`, next steps 7). `/var/lib/hanoip/cycle-offset` on the phone is a temporary display offset.

## Bugs found, and what to remember
Every bug gets an automated test that fails before the fix and passes after (see below). Existing ones are `scripts/test-qgd`, `scripts/test-charge-limit`, `scripts/test-bot`.

| Bug | Cause | Rule |
|---|---|---|
| Battery stuck at 100 % for days, charge limit never cut, battery died, PMIC lost SDAM | `qpnp-qg` does not compute SoC after boot; a userspace daemon must write it back | `hanoip-qgd` must keep running; the charge limit must not trust the percentage alone (stale file > 15 min or < 3.75 V resumes charging) |
| Percentage frozen at 100 % with the cable suspended | Charger keeps reporting `Full` while discharging | `Full` sets 100 % only when the batch shows no discharge (< 100 mA) or no samples |
| Gauge scale off | Profile table is the cell's scale, kernel maps 3.4 V to 0 % and 4.45 V to 100 % | Pull toward the OCV on the kernel's scale |
| SSH dropped, load 3-4, conntrack full (65,536) | Tailscale rewrote `resolv.conf` to its own MagicDNS and looped | `tailscale set --accept-dns=false`, then `resolvconf -u` |
| Phone gone after ~7 reboots | pmOS never marks the A/B slot successful; ABL makes slot b unbootable and flips to a | `qbootctl` service; recovery `fastboot set_active b` + `fastboot reboot` (check the slot flags first and say what was done) |
| nftables rules with `{ ... }` fail with "Not supported" | Set backends missing | Keep `NFT_SET_HASH/RBTREE/BITMAP` as `=y` (the kernel package installs no modules) |
| `losetup -P --sector-size 4096` makes no `loopNpX` | Loop does not rescan after `LOOP_SET_BLOCK_SIZE` | Keep kernel patch 0001 |
| ABL never ran the mainline kernel | ABL only runs header v3 and only `Image`/`Image.gz`, not EFI zboot | `EFI_ZBOOT` off; v3 `vendor_boot`; earlier v0 tests are invalid |
| `fastboot flash userdata` refused | Motorola fastboot | Write the rootfs through the Lineage recovery with `dd` and verify the hash |
| Every boot took ~3 min | Each missing firmware file waited 60 s | Keep `FW_LOADER_USER_HELPER_FALLBACK` off |
| `dtc` segfaults | The tree's own `dtc` built with clang 23 | Build with `DTC=/usr/bin/dtc` |
| Cycle counter lost (8 buckets) | A fully drained battery clears PMIC SDAM | Never let the battery die; see the pending patch above |

## Tests for every bug
- Add the regression test before or with the fix. Prefer extending the matching `scripts/test-*` (they run without the phone); create a new `scripts/test-<thing>` in the same style when none fits.
- Run the test and show it failing on the old behavior, then passing.
- Commit the test with the fix, or right before it.

## Commits
Small, one per specific change, `type(scope): summary` with a body saying why. Never mention AI, no trailers. Author is always the git config identity; contributors on GitHub must show only `klockmt` (`gh api repos/klockmt/hanoip-pmos/contributors` before pushing).

Push to the `beta` branch first. Move commits to `main` only after they were tested and the user approved; never push straight to `main`.
