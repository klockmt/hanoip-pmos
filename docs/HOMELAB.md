# Running the phone as a server

How the Moto G60 is operated now that Duranium (Nura's systemd image) is the system on it: how to reach it, what runs,
how it is updated and how to get it back. What works is in [`STATUS.md`](STATUS.md); what broke and why is in
[`PITFALLS.md`](PITFALLS.md); the log of the work is [`DURANIUM.md`](DURANIUM.md) and the investigations are in
[`../NOTES.md`](../NOTES.md). Paths and addresses are variables from `.env` (`.env.example` lists them): nothing about
your phone goes in the repository.

The phone runs the 4.14 server's job on a mainline 7.3 kernel with systemd, from the image `26111201` (device package
`pkgrel 54`) on slot b. The 4.14 server (OpenRC, chrony) is frozen as v0.2.2 on `main`; its old OpenRC notes are in the
git history of this page, not here.

## Reaching it

- **SSH accepts only two paths: the USB cable and the tailnet.** The file `/etc/nftables.d/02_hanoip-ssh.nft` (from the
  device package) loads before pmOS's `50_sshd.nft`, which would accept port 22 from every interface, and accepts it on
  `usb*` and `tailscale0` only. Ethernet and Wi-Fi get nothing. Keys only: password login is off
  (`60-hanoip-keys-only.conf`). Root login is off too; the login user administers with `sudo` (sudo-rs).
- `HANOIP_HOST` in `.env` is the address the scripts use: the USB gadget address while the cable is in, or the phone's
  Tailscale address (`100.x.y.z`) when it is out. `scripts/hanoip-ssh '<cmd>'` runs a command with the dedicated key;
  `scripts/hanoip-sudo '<cmd>'` runs it as root (the password goes over the ssh stdin and the command as base64, never in
  an argument list).
- On the same LAN `tailscale ping` is direct. When the PC is on another network it falls back to the DERP relay and long
  transfers can drop: Cloudflare WARP on the PC takes Tailscale's UDP. Pause it (`warp-cli disconnect`, then
  `warp-cli connect`) when working with the phone off the LAN, and keep copies short over a relay (`hanoip-ssh` gives up
  after about 30 s of silence). A relayed copy that stops can be finished by appending from the size the phone already has.
- Large transfers over the USB (NCM) network take it down until a reboot (issue #2). Throttle every copy to 1 MB/s with
  `pv -L 1m`; `push-firmware`, `push-modules` and `flash-boot-ssh` already do.

## Status dashboard

`hanoip-dashboard` (device package, `hanoip-dashboard.service`, a dynamic user, no root and no sudo) serves one page and
its data on the Tailscale address only, port 8080:

- `http://<tailnet-address>:8080/` is the page; it asks `/api/status` only while the tab is visible. The page's own
  period is `PERIOD_MS` in `dashboard/index.html` (1 s).
- The live figures (CPU and its clocks, GPU, RAM, battery, power, body and the temperatures, the network rates, uptime,
  zram, OOM kills, watchdog) are read from the files on each request, at most 0.9 s old: a fresh answer every second. CPU
  and network rates are deltas between two samples, so the first answer after start has them null.
- Ping, services, the NAS disk and its scrub, Wi-Fi, Tailscale, the boot slot and the `/status` text come from
  `hanoip-status` and the commands, at most every 10 s (`slow_at` in the JSON is when they were read). A request never
  waits for them after the first one; it gets the last result, and a refresh runs in the background only while a page asks.
- `/api/status` is the JSON: the `/status` text, system (uptime, reboots, OOM kills, RAM, zram), boot (slot from the ABL,
  and whether it is marked successful), storage (the NAS disk and its last scrub), network (Wi-Fi, Tailscale, Samba on
  the tailnet), services, watchdog and versions (kernel and the package's pkgrel).
- A field that cannot be read is `null` and the page shows `n/a`; nothing is approximated.
- The address is read at start (`tailscale ip -4`), so nothing about it is written in the repo.
- `hanoip-dashboard --data` prints the same JSON once, on the phone, for a check.

What the page shows: a health ring with six checks (SSH, the bot, Wi-Fi, Tailscale, the watchdog fed within 60 s, no
process killed for lack of memory), rings for the battery and the RAM, charts for the power draw, the CPU, the
temperatures, the ping, the network and the watchdog, bars for zram and the disks, the A/B slot, the services, and the
raw `/status` text. The charts hold only what the open page has seen, up to an hour; a reload starts them again. It warns
below 20 % of battery and from 85 % of RAM or of the NAS disk. One column on a phone, a 12-column grid from 960 px, light
and dark.

The first request after the service starts waits for the first slow collection (over 6 s); after that an answer takes a
fraction of a second.

Checked on 2026-11-12: answers from the tailnet; refused on the USB address; no answer on the LAN address; the figures
change each second (image `26111201`). The owner tested the page on a phone and reports it is good. Only the page's look
on the phone was checked by hand; the data and the access were checked by `scripts/test-dashboard` and
`scripts/test-dashboard-page`.

## Services

The device package's mainline subpackage (`device-motorola-hanoip-mainline`) installs the units, and its preset
(`hanoip.preset`) enables them. Each one that needs restored data or the NAS disk waits for it with a unit condition, so
a boot without them is still a clean boot.

| Service | What it does | Check |
|---|---|---|
| `hanoip-charge.service` | Holds the battery at the 75-80 % limit on the cable; the mainline gauge reports the percentage, so `hanoip-qgd` is not needed | `hanoip-status` shows the charge line |
| `hanoip-thermal` | From 40 °C pauses charging, from 45 °C caps the CPU, at 55 °C warns on Telegram and powers off | `docs/BATTERY.md` |
| `hanoip-bot` | The Telegram bot: `/up`, `/status`, `/status -h`, `/status -t=MIN -g=MIN`, `/stop`, `/notes` (lists them); only the owner's private chat. Its secrets are in `/etc/hanoip/telegram.env`, restored by `scripts/restore-server` | `hanoip-bot --alert TEXT` sends one message |
| `hanoip-led` | The notification LED | `systemctl status hanoip-led` |
| `hanoip-health` (a one-shot at boot, not a daemon) | Once sshd runs and the phone has an address (`usb0` 172.16.42.1, or `wlan0` or `eth0`), runs `qbootctl -m` for the slot the ABL started. If that has not happened within 600 s of the kernel start, it reboots (`reboot -f`), so the ABL counts a failed boot. The Ethernet-only path is still open (#28) | `journalctl -u hanoip-health`; `fastboot getvar slot-successful:b` |
| `hanoip-usb-gadget` | The USB network (NCM) to the PC | `ip link` shows `usb0` |
| `hanoip-modem` and its helpers (qrtr, pd-mapper, tqftpserv, rmtfs) | Brings the modem up, which runs the Wi-Fi firmware (subpackage `device-motorola-hanoip-modem`) | `dmesg` shows `icnss: WLAN FW is ready` |
| `tailscaled` | The tailnet | `tailscale status` |
| `hanoip-dashboard` | The status page above | `curl` to its tailnet address, port 8080 |
| `hanoip-screen-off` | Keeps the screen off (see "The screen") | the screen stays dark |
| `systemd-timesyncd` | The clock (chrony is not used on Duranium) | `timedatectl` |
| zram | A zstd swap, the mainline `postmarketos-zram-swap` | `swapon --show` |
| `adguardhome` | Secondary DNS, on loopback only, see "AdGuard Home" | `nslookup example.com 127.0.0.1` |
| `smb` and `syncthing` | The NAS shares; they wait for the disk (see "The NAS disk") | `systemctl status smb syncthing` |
| `mnt-hd\x2da.mount`, `mnt-hd\x2db.mount` | The two NAS partitions, `nofail` | `findmnt /mnt/hd-a /mnt/hd-b` |

Switched off by the preset, because a headless server does not need them: avahi (mDNS, port 5353), rtkit, systemd-oomd,
the login prompt on the tty (`getty@`). LLMNR and mDNS are off in resolved (its stub stays; AdGuard binds loopback only),
and core files are not written.

**Wi-Fi off while Ethernet carries the server.** `hanoip-ethernet-wifi.service` (script `/usr/bin/hanoip-ethernet-wifi`)
runs `nmcli radio wifi off` while eth0 has carrier, an IPv4 address and a default route through it, and `nmcli radio wifi on`
when any of the three is gone (cable out, hub unplugged, lease lost). It samples once a second and changes the radio only
after five samples in a row agree. The radio is a software block (rfkill): the Wi-Fi firmware stays loaded and `hanoip-modem`
is not touched. NetworkManager keeps the radio state across reboots, so the unit decides at boot too, and stopping it turns
Wi-Fi on. Expect an SSH session over Wi-Fi to drop about 5 s after eth0 comes up. Check: `nmcli radio wifi` and
`ip -4 -o addr show dev eth0`. Tested with stand-ins only (`scripts/test-ethernet-wifi`), not yet on the phone. Known limit:
an eth0 with an address and a route but no uplink keeps Wi-Fi off, and then the phone is reachable over eth0 or USB only.

`systemctl is-system-running` says `running` when nothing failed (checked 2026-11-12, no failed unit). `hanoip-status` prints
the health of the phone, one group of values per line (`hanoip-status -h` says what each is and where it is read).

**`/etc/local.d` on Duranium.** The device package installs four scripts there (`10-hanoip-console`,
`20-hanoip-qbootctl`, `30-hanoip-cpufreq`, `40-hanoip-tailscale-dns`). Nothing on systemd ran that directory (no `rc-local`
unit on the phone, checked 2026-11-12), so the console setting, the CPU governor and the Tailscale DNS switch were not
applied at boot. Now `hanoip-local.service` runs the first, third and fourth after `tailscaled`, and
`scripts/test-duranium-boot` checks the unit. `20-hanoip-qbootctl` is left out on purpose: `hanoip-health` marks the slot
the ABL started, and `qbootctl -m` would mark slot a on every boot. Not yet on the phone: it comes with the next image.
The slot marking by `hanoip-health` is what worked on the phone (the boot at 17:48 UTC marked `_b`).

The bot's secrets (token, owner id) live only on the phone in `/etc/hanoip/telegram.env`; they are never printed, logged or
committed. The bot runs fixed commands: text from a message never reaches a shell.

## Tailscale and DNS

The phone does not need MagicDNS. On the 4.14 server `tailscaled` once rewrote `/etc/resolv.conf` to MagicDNS
(`100.100.100.100`), failed to restore it, and looped its own DNS through `tailscale0` (about 7,000 connections in
`TIME_WAIT`, a full conntrack table). The fix was to turn the DNS setting off:

```sh
tailscale set --accept-dns=false     # kept in the Tailscale state
resolvconf -u                        # if Tailscale did not restore the file, do it
tailscale debug prefs | grep CorpDNS # must say false
```

`resolv.conf` then points at the router again. Undo with `tailscale set --accept-dns=true`. The device package ships the
firewall file for Tailscale (`52_hanoip-tailscale.nft`): everything arriving on `tailscale0`, and UDP 41641 (which lets
two peers connect directly) only on `eth*` and `wlan*`. Logging in is still by hand, once: `tailscale up --accept-dns=false`.
On Duranium (checked 2026-11-12): `CorpDNS` is false, and `/etc/resolv.conf` is systemd-resolved's stub (`127.0.0.53`,
search `lan`). The script that reapplies this after a boot (`/etc/local.d/40-hanoip-tailscale-dns.start`) does not run
(see "`/etc/local.d` does not run on Duranium" above), so a Tailscale that turns DNS back on is not undone at boot.
`hanoip-status` still warns if `resolv.conf` shows the Tailscale DNS.

## AdGuard Home (secondary DNS, issue #25)

Its unit is enabled and waits for its restored config (`scripts/restore-server` puts the 4.14's `AdGuardHome.yaml` back).
The config is owned by `adguardhome` and mode 600. The part that was written by hand on the 4.14:

```yaml
http:
  address: 127.0.0.1:3000
dns:
  bind_hosts:
    - 127.0.0.1
  port: 53
  upstream_dns:
    - 9.9.9.9
```

Do not add `bootstrap_dns` as a list in a minimal file: AdGuard rewrote it as a nested list (`- - 9.9.9.9`) and then refused
to start (`yaml: construct errors: cannot construct !!seq into string`). Remove the line and let AdGuard fill its defaults.

It listens on loopback only, so the LAN and Wi-Fi cannot reach it yet. Still to do, in issue #25: bind to the LAN address,
open port 53 on the LAN in its own `.nft` file (the existing `51_hotspot.nft` already accepts 53 on `wlan*`, check that
first), test a client, and check a reboot. The admin page is `127.0.0.1:3000` (reach it with an SSH tunnel).

## Firewall

`/etc/nftables.nft` is loaded at boot (enabled). It starts with `flush ruleset` and includes `/etc/nftables.d/*.nft` in name
order; the first match wins. Checked on the phone on 2026-11-12: all the files below are there, and the effective input
chain has policy `drop`, with SSH accepted only on `usb*` and `tailscale0` and dropped from any other interface, and
Samba (139, 445) accepted only on `tailscale0`. `50_sshd.nft` is a symlink to pmOS's `nftables.avail`.

| File | From | Does |
|---|---|---|
| `01_wwan.nft` | pmOS | Drops everything on `wwan*` and `qmapmux*` |
| `02_hanoip-ssh.nft` | this repo | SSH only over `usb*` and `tailscale0` |
| `03_hanoip-samba.nft` | this repo | Ports 139 and 445 only on `tailscale0` |
| `10_dhcp.nft` | pmOS | DHCP on `usb*` and `wlan*` |
| `50_ausweisapp2.nft`, `50_localsend.nft` | pmOS | Ports of those apps |
| `50_sshd.nft` | pmOS | Accepts port 22 from everywhere; the file above wins first |
| `51_hotspot.nft` | pmOS | Hotspot rules (accepts 53 on `wlan*`, used by AdGuard) |
| `51_usb_inet.nft` | pmOS | Accepts **everything** coming in over `usb*` |
| `52_hanoip-tailscale.nft` | this repo | Everything on `tailscale0`, UDP 41641 only on `eth*` and `wlan*` |

Do not open a port, weaken a rule, enable password login or add a public-facing service without a decision. To change a
rule on the running phone without wiping Tailscale's own tables, use `nft insert rule` and `nft replace rule`; a reload of
`/etc/nftables.nft` starts with `flush ruleset`.

## The screen

The screen is off from the start of the boot and stays off after every reboot (`hanoip-screen-off.service`). The kernel
cuts the panel supplies (`vddio`, `lab`, `ibb`, from the LCDB and the PM6150L) and the backlight, and the phone keeps
running, with SSH and Tailscale. The boot parameter `hanoip_splashfb.screen_off=1` does this; the image built by
`scripts/mkbootimg-kernel` carries it.

There is no command to switch the screen on a running system. It comes back with a reboot of an image built without the
parameter (the 4.14 way, `scripts/mkbootimg-kernel` with `KERNEL_CMDLINE="ignore_loglevel fbcon=font:SUN12x22"`, flashed
to `boot_b`). Do not use `echo 4 > /sys/class/graphics/fb0/blank`: it only touches the backlight and does not blank the
screen. The kernel log is still written; follow it over SSH.

## Memory, disks and what the kernel offers

- **zram**: zstd, 150 % of RAM, `swappiness=180`, `page-cluster=0`. On a sample of real files zstd packed 2.24x at about
  34 MiB/s, against lz4's 1.65x at about 180 MiB/s. Not measured under real memory pressure (issue #32).
- **btrfs** is built in; the NAS partitions are btrfs. Check `btrfs inspect-internal dump-super` before a mount.
- **Containers**: user, PID and network namespaces, the `pids` and `devices` cgroups, `br_netfilter` (built in), `overlayfs`,
  `veth`, `macvlan`. Use `crun`: the `runc` of Alpine edge panics on 4.14 (the mainline kernel has not been tested with it).
  Not available on the 4.14: WireGuard (use Tailscale), `CFS_BANDWIDTH`, swarm/overlay networks.
- **Watchdog**: `/dev/watchdog0` is `qcom_wdt` with a 20 s timeout, active (checked 2026-11-12). systemd feeds it: the
  device package sets `RuntimeWatchdogSec=20` (`90-hanoip-watchdog.conf`), and `systemctl show` reports
  `RuntimeWatchdogUSec=20s` and `WatchdogDevice=/dev/watchdog0`. `hanoip-watchdog`, the 4.14 feeder that also reset the
  phone when the network was lost, is not in the preset and does not run. The dashboard's watchdog ring reads the sysfs
  timeout, so it shows the time since the last feed; its resets count reads `n/a`, because that count is kept by the
  4.14 feeder. A stopped watchdog on Duranium is not tested (open item 4 in [`DURANIUM.md`](DURANIUM.md)).
- **Kernel modules** load (patch 0010). The module tree has to come from the same build as the running kernel;
  `scripts/push-modules` copies it and checks the release.

## The NAS disk

A 3.5" SAMSUNG HD502HJ (500 GB, 7200 rpm) behind a USB-SATA bridge. It is formatted on the PC. Its serial stays out of the
repo: identify it with `smartctl -i -d sat <dev>`.

**It has a damaged zone, so it is partitioned around it** (GPT, 512 B logical sectors, 1 MiB aligned):

| Partition | Label | Range | Size |
|---|---|---|---|
| 1 | `hd-a` | 1 MiB to 100 GB | 93 GiB |
| none | | 100 GB to 165 GB | ~65 GB, never partition or write it |
| 2 | `hd-b` | 165 GB to the end | 312 GiB |

Why: from LBA 220053504 (~112.7 GB) the surface fails every ~15 MB and each read of it drops the disk off USB. The rest that
was read (0 to ~105 GB and 150 GiB to the end) read clean at 110 MB/s. See "Storage" in [`PITFALLS.md`](PITFALLS.md).

How it is used on Duranium (2026-11-12):

- The disk appears on the hub when the hub is plugged in (its own power supply, USB host mode). Both partitions mount by
  label at boot (`nofail`, so a missing disk does not hold the boot), read-write with `compress=zstd`.
- **A disk that arrives after the boot is not mounted by itself.** Mount it by hand with `systemctl start` of the two mount
  units; `smb` and `syncthing` then start.
- Plugging the hub in with the phone booted does not take the USB role back: a boot leaves the port in the device role, so
  there is no Ethernet and no disk until the hub is plugged in again. Whatever sets the host role at boot must not take
  `usb0` away from a PC on the cable (issue #28, open).
- Samba (`hd-b`, the `cloud` share): port 445 answers over the tailnet and not on the LAN; an anonymous `smbclient -L` is
  refused. A login from a client with the share's password is not checked yet (issue #27).
- Syncthing (`hd-a`, the `obsidian` folder): its state stays on the internal flash, so it survives the disk dropping. It
  scanned the folder after the mount on 2026-11-12. A sync from a client is not checked yet (issue #27).
- Keep only data that is reproducible or has a second copy. The disk is wearing out: watch `Current_Pending_Sector` with
  `smartctl -A -d sat <dev>` on the PC now and then (148 on 2026-11-02, `Reallocated_Sector_Ct` 0). If it keeps rising,
  replace the disk. A SMART check on the phone is issue #29.
- Run `btrfs scrub start <mount point>` regularly: btrfs checksums data, so a bad read is reported instead of passing (the
  dashboard's scrub field shows `n/a` until a scrub has run).
- Unmount before powering the disk off. After a USB drop, power-cycling the drive is what brings it back.

## Updating the phone

Duranium updates are A/B images through `systemd-sysupdate`. The device package and the kernel are built on the PC with
pmbootstrap, which asks for your sudo password, so the person runs it. Done once for `26111201`:

```sh
scripts/setup-pmaports                                             # pmaports checkout + this repo's packages
scripts/pmb build --force device-motorola-hanoip                   # the device package (sudo: you run it)
scripts/duranium-image --image-version=YYMMDDNN                    # the image, from Nura's duranium-build (mkosi, podman)
```

Then look inside the image before sending it (`dump.erofs --path=/share/... --cat`; the paths are relative to `/usr`), check
that `pkgrel` reads the new value, compress the `usr` and `usr-verity` files with `zstd`, and copy them and the `.efi` to
`/var/tmp/hanoip-update/` on the phone at 1 MB/s (`pv -L 1m`). Compare the hashes on both ends, then:

```sh
scripts/hanoip-sudo '/usr/lib/systemd/systemd-sysupdate update'   # writes the free pair (the other slot)
scripts/hanoip-sudo reboot
```

- The new image goes to the **free** slot; the installed one stays as the fallback (`26111103` is the fallback for
  `26111201`). Judge the slot by `/usr/bin/hanoip-ml-abl-slot`, not `/proc/cmdline` (which always says `_a`).
- A flash of `boot`, `vendor_boot` or `userdata` is not part of an update. `userdata` is written only from the rescue
  root with `scripts/duranium-write-userdata`, which erases it: run `scripts/restore-server` after every write.
- Scripts that change something take `DRY_RUN=1` (print and stop). `scripts/check-repo` is the gate before every commit;
  `scripts/test-*` need no phone.

## Getting it back

- **Slot b is not marked successful** after a bad boot: the ABL then marks it unbootable and switches to the other slot.
  Check first with `fastboot getvar slot-unbootable:b`, then `fastboot set_active b` and `fastboot reboot`. `boot_a` still
  has the previous U-Boot, and the Alpine root in `super` is the last resort (see [`DURANIUM.md`](DURANIUM.md)).
- **A bad image**: the phone does not come back over the network. Hold Volume Down while powering on, or use fastboot,
  and set the other slot active. The backups of `boot_b` and `vendor_boot` are on the PC in `$HANOIP_BACKUP_DIR`; a phone
  that does not boot cannot hand its own backup back, which is why they live on the PC.
- **Never write the `misc` backup back as it is** (it starts with `bootonce-bootloader` and would drop the phone into the
  bootloader once); zero those bytes first. If you zero the in-use slot's `dtbo` for a test, restore it before any reboot.
- **The battery must not die**: it clears the PMIC data (the cycle counters). `scripts/restore-cycle-counts` puts the
  counters back; `restore-server` restores the cycle-count marker.
- **Rebooting with the raw `reboot()` syscall** (`scripts/reboot-bootloader` does) skips the clean shutdown, so the boot's
  log is not written. Use `reboot` when the log matters.
- **What recovers by itself, and what does not** (issue #17; the Duranium column is not tested yet):

  | Failure | 4.14 server | Duranium (2026-11-12; none of it tested on the phone) |
  |---|---|---|
  | Userspace hang | `hanoip-watchdog` fed `/dev/watchdog`; the phone reset in about a minute | systemd feeds `qcom_wdt` (20 s); a hang of PID 1 stops the feed, so the hardware resets the phone. A hung service on its own does not |
  | Network lost (gateway silent for 5 minutes, no USB link) | the watchdog service stopped feeding on purpose, up to 3 resets in a row, and told Telegram after the reset | not covered: no feeder runs |
  | Kernel hang | no, the Qualcomm watchdog had no device node | `qcom_wdt` is armed and nothing feeds it, so it should reset in about 20 s |
  | sshd down after a boot | not covered | `hanoip-health` reboots after 600 s without sshd and an address; the slot then counts a failed boot. `scripts/test-duranium-boot` covers the script, not the phone |
  | Power cut | not tested | not tested |

  On the 4.14, stopping the feeder was safe (the magic close was written); killing it with `kill -9` left the watchdog armed.
