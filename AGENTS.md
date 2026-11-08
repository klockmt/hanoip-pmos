# hanoip-pmos

postmarketOS on the Moto G60 (hanoip, SM7150), run as a **headless server reached over SSH**. The repo is public.

The rules are here. Before touching anything read `docs/STATUS.md` (what works), then `NOTES.md` (the log of the investigations); `docs/PITFALLS.md` and `docs/HOMELAB.md` say what broke and how the phone is run. The Duranium work follows `docs/DURANIUM.md`.

## Rules that always apply
- The repo check is `scripts/check-repo`: run it before every commit and push.
- Local paths and personal data live in `.env` (copy of `.env.example`). Scripts source `scripts/env.sh`; docs use `$HANOIP_*` variables. To resolve one, read `.env`.
- Never write an IMEI, serial, MAC, battery id, home path or the user name in any tracked file, test fixture, log or commit message. Device dumps and partition backups stay in `$HANOIP_BACKUP_DIR` / `$HANOIP_COLLECTION_DIR`, outside the repo.
- Keep `README.md` honest about which scripts were run end to end.
- Do not flash partitions beyond what the user asked for, never wipe or factory reset. Inspect the target and compare with `SHA256SUMS` first. `fastboot set_active b` + `reboot` is pre-authorized only to recover the A/B slot (see below).
- `pmbootstrap` asks for the `sudo` password and needs interactive input, so **the user runs it**: hand over ready-made commands and wait. It is not on PATH; use `$PMBOOTSTRAP`.
- When zeroing the in-use slot's `dtbo` for a test, restore the backup right afterwards, before `fastboot continue` or any reboot.
- The mainline development root lives only in the unallocated region of `super` (`sdb23`): offset 1120 MiB, 2400 MiB (approved by the user on 2026-11-07). Write it only through a loop device bounded by that offset and size, hash `super` outside the region before and after every write, and never write the LP metadata, the Android volumes or any other partition from a mainline boot. One exception (approved by the user on 2026-11-08, so that slot a is a second working slot): after a complete boot from slot a, `qbootctl -m a` (`98-hanoip-ml-slot.start`) marks every partition of slot a and writes back the GPT of each disk that holds one, LUN 4 and LUN 2 (`xbl_a`), as the 4.14 does for slot b at every boot; the guard lends it those whole disks only. `qbootctl` with no arguments is not read-only either.

## Working over SSH, securely
- Use `scripts/hanoip-ssh '<cmd>'`. It uses the dedicated key from `.env` with `IdentitiesOnly` and `BatchMode`; `HANOIP_SSH_TTY=1` only when a password prompt is needed.
- `sudo` asks for a password, which the user allowed the agent to keep in `.env` (`HANOIP_SUDO_PASSWORD`, `HANOIP_ROOT_PASSWORD`). Run root commands with `scripts/hanoip-sudo '<cmd>'` (password over the ssh stdin, command as base64); never print or log the value. Do not make `sudo` passwordless again, and keep ssh on keys only (`hanoip-sshd.conf`).
- Never print, log or commit a key, the Telegram token or owner id. The bot secrets live only on the phone, in `/etc/hanoip/telegram.env`; `telegram.env` and `*.token` are gitignored.
- SSH is accepted only over USB (`usb*`) and Tailscale (`02_hanoip-ssh.nft`); do not undo that. Do not weaken the nftables firewall, open a port, enable password login or add a public-facing service without asking. Remember `51_usb_inet.nft` already allows everything coming in over USB.
- The Telegram bot runs fixed commands only. Text from a message must never be used to build a command.
- Before a risky change over SSH (network, firewall, boot), check you still have a way back: `flash-boot-ssh` saves the old `boot_b` first; otherwise the recovery is fastboot.
- Large transfers over the USB (NCM) network take it down until a reboot. Throttle them (`pv -L 1m`, as `scripts/push-firmware` does).

## Server setup
- The screen is off by design and only comes back with a reboot on an image built without `hanoip_splashfb.screen_off=1`. Do not use `fb0/blank` (`docs/HOMELAB.md`).
- Services (OpenRC): `hanoip-qgd` (battery percentage, see below), `hanoip-charge-limit` (needs `hanoip-qgd`), `hanoip-bot`, `hanoip-thermal`, `hanoip-modem` (Wi-Fi), Tailscale. `qbootctl -m` runs from `/etc/local.d` (keep `local` in `default`).
- Pending: `systemd-tmpfiles`/`sysusers` cannot read their `.d` directories on 4.14 (`openat2`), which is harmless here (`NOTES.md`). Open work lives in the GitHub issues (roadmap #20).
- Kernel direction (2026-11-07): mainline is the goal again: make it boot, then make it the server (`docs/STATUS.md`, section 5). Meanwhile the downstream 4.14 server stays on slot `b`, frozen as v0.2.2; enforce its hardening options in `scripts/test-kernel-config`. The base is the mainline base (U-Boot plus Linux 7.3, RAM-only `fastboot boot`). Mainline tests are RAM-only and never mount `userdata` until the user approves a flash. systemd packages are fine; Nura's Duranium is the long-term target.
- A package update flashes `boot` and `vendor_boot` (`deviceinfo_flash_kernel_on_update="true"`): the 2026-10-30 hang was `console=tty0` in the command line, and `scripts/test-deviceinfo` keeps `console=ttyMSM0,115200n8` first (`NOTES.md`). Never flash by hand `boot`/`vendor_boot` without a backup on the PC (`$HANOIP_BACKUP_DIR`).

## Bugs found, and what to remember
What broke on this phone, why, and the test or file that keeps it from breaking again is in [`docs/PITFALLS.md`](docs/PITFALLS.md). Read it before changing the kernel config, the device package, the charge limit, the firewall or a flashing script.

## Tests for every bug
Prefer extending the matching `scripts/test-*` (they run without the phone); create a new `scripts/test-<thing>` in the same style when none fits.

## Commits and releases
Before pushing, check the contributors with `gh api repos/klockmt/hanoip-pmos/contributors`: only `klockmt` may show. The beta branch is `beta`.
