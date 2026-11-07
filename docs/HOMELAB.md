# Running the phone as a server

How the Moto G60 is operated once pmOS is on it: how to reach it, what runs, how it is updated and how to get it
back. What works is in [`STATUS.md`](STATUS.md); what broke and why is in [`PITFALLS.md`](PITFALLS.md); the
investigations are in [`../NOTES.md`](../NOTES.md). Paths and addresses are variables from `.env`
(`.env.example` lists them): nothing about your phone goes in the repository.

## Reaching it

- **SSH accepts only two paths: the USB cable and the tailnet.** The file `/etc/nftables.d/02_hanoip-ssh.nft` (from the
  device package) loads before pmOS's own `50_sshd.nft`, which would accept port 22 from every interface, and accepts it
  on `usb*` and `tailscale0` only. Ethernet and Wi-Fi get nothing. Keys only: password login is off
  (`60-hanoip-keys-only.conf`).
- `HANOIP_HOST` in `.env` is the address the scripts use: the USB gadget address while the cable is in, or the phone's
  Tailscale address (`100.x.y.z`) when it is out. `scripts/hanoip-ssh '<cmd>'` runs a command with the dedicated key;
  `scripts/hanoip-sudo '<cmd>'` runs it as root (the password goes over the ssh stdin and the command as base64, never in
  an argument list; the command cannot read stdin).
- On the same LAN `tailscale ping` is direct. When the PC is on another network it falls back to the DERP relay and long
  transfers can drop: Cloudflare WARP on the PC takes Tailscale's UDP. Pause it (`warp-cli disconnect`, then
  `warp-cli connect`) when working with the phone off the LAN, and keep copies short over a relay (`hanoip-ssh` gives up
  after about 30 s of silence).
- A phone with Termux reaches it the same way: over the tailnet, with its own public key in `~/.ssh/authorized_keys`.
- Large transfers over the USB (NCM) network take it down until a reboot (issue #2). Throttle every copy to 1 MB/s with
  `pv -L 1m`; `push-firmware`, `push-modules` and `flash-boot-ssh` already do.

## Services

All OpenRC, on by default, started by the device package.

| Service | What it does | Check |
|---|---|---|
| `hanoip-qgd` | The battery percentage: the kernel gauge only measures, this daemon computes the charge and writes it back | `cat /run/hanoip-qgd.soc` moves about once a minute |
| `hanoip-charge-limit` (needs `hanoip-qgd`) | Holds the battery near 80 % on the cable (bypass), resumes below 60 %, one charge to 100 % every 10 cycles | `hanoip-status` line 8 says `bypass` |
| `hanoip-thermal` | From 40 °C pauses charging, from 45 °C caps the CPU, at 55 °C warns on Telegram and powers off | `docs/BATTERY.md` |
| `hanoip-watchdog` | Feeds `/dev/watchdog` and lets it reset the phone if the network stays down (below, "Getting it back") | `ls /var/lib/hanoip/watchdog` (`count` is the resets in a row) |
| `hanoip-bot` | The Telegram bot: `/up`, `/status`, `/status -h`, `/status -t=MIN -g=MIN`, `/stop`, `/notes` (lists them); only the owner's private chat | `hanoip-bot --alert TEXT` sends one message |
| `hanoip-modem` and its helpers (`hanoip-qrtr-ns`, `-pd-mapper`, `-tqftpserv`, `-rmtfs`) | Brings the modem up, which runs the Wi-Fi firmware (subpackage `device-motorola-hanoip-modem`) | `dmesg` shows `icnss: WLAN FW is ready` about 16 s after boot |
| `postmarketos-zram-swap` | A zstd zram swap, 150 % of RAM, priority 300, above the 1 GB swapfile | `swapon --show` |
| `tailscale` | The tailnet. DNS from the tailnet is off (below) | `tailscale status` |
| `adguardhome` (not in the device package: `apk add adguardhome adguardhome-openrc` by hand) | Secondary DNS on loopback only, see "AdGuard Home" below | `nslookup example.com 127.0.0.1` |
| `local` (not a daemon) | Runs `/etc/local.d`: `qbootctl -m` marks the A/B slot successful at every boot, the cpufreq governor, the console | keep `local` in `default` |

`hanoip-status` prints the health of the phone in ten lines, plus a Warning line when `resolv.conf` uses the Tailscale DNS (`hanoip-status -h` says what each is and where it is read).
The bot's `/status` is the same output.

The bot's secrets (token, owner id) live only on the phone in `/etc/hanoip/telegram.env`; they are never printed,
logged or committed. The bot runs fixed commands: text from a message never reaches a shell.

## Tailscale and the DNS loop

`tailscaled` once rewrote `/etc/resolv.conf` to its own MagicDNS (`100.100.100.100`) and failed to restore the original
(`resolvconf: signature mismatch`). Its own lookups then went through `tailscale0` back to itself and kept being reset:
about 7,000 DNS connections in `TIME_WAIT`, more than 2,000 packets a second, a load of 3-4 and a full conntrack table
(65,536 entries) that dropped new connections, SSH included. The phone does not need MagicDNS, so:

```sh
tailscale set --accept-dns=false     # kept in the Tailscale state
resolvconf -u                        # Tailscale did not restore the file, so do it
tailscale debug prefs | grep CorpDNS # must say false
```

`resolv.conf` then points at the router again. Undo with `tailscale set --accept-dns=true`. The device package depends on
`tailscale` and ships the firewall file for it (`52_hanoip-tailscale.nft`): everything arriving on `tailscale0`, and UDP
41641 (what lets two peers connect directly) only on `eth*` and `wlan*`. Logging in is still by hand, once:
`tailscale up --accept-dns=false`; the package does not start Tailscale or sign in for you. If DNS was left on, the
boot script `/etc/local.d/40-hanoip-tailscale-dns.start` turns it off and runs `resolvconf -u` at the next boot (it does
nothing while Tailscale is not running or not signed in). `hanoip-status` still warns if `resolv.conf` shows the
Tailscale DNS.

## AdGuard Home (secondary DNS, issue #25)

Installed by hand on 2026-11-07 (`apk add adguardhome adguardhome-openrc`, version 0.107.79) and set to start at boot
(`rc-update add adguardhome default`). Its config is `/var/lib/adguardhome/AdGuardHome.yaml`, owned by `adguardhome`
and mode 600. The part that was written by hand:

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

Do not add `bootstrap_dns` as a list in a minimal file: AdGuard rewrote it as a nested list (`- - 9.9.9.9`) and then
refused to start (`yaml: construct errors: cannot construct !!seq into string`). Remove the line and let AdGuard fill
its defaults.

It listens on loopback only, so the LAN and Wi-Fi cannot reach it yet. Still to do, in issue #25: bind to the LAN
address, open port 53 on the LAN in its own `.nft` file (the existing `51_hotspot.nft` already accepts 53 on `wlan*`,
check that first), test a client, and check a reboot. The checks done so far: `nslookup example.com 127.0.0.1` resolves;
a test rule `||blocked-test.example^` (set through `/control/filtering/set_rules`, then removed) answered `0.0.0.0`.
The admin page is `127.0.0.1:3000` (reach it with an SSH tunnel). The default filter list (about 190,000 rules) is
downloaded from AdGuard on the first run.

## Firewall

Files in `/etc/nftables.d`, loaded in name order, first match wins:

| File | From | Does |
|---|---|---|
| `01_wwan.nft` | pmOS | Drops everything on `wwan*` and `qmapmux*` |
| `02_hanoip-ssh.nft` | this repo | SSH only over `usb*` and `tailscale0` |
| `10_dhcp.nft` | pmOS | DHCP on `usb*` and `wlan*` |
| `50_sshd.nft` | pmOS | Accepts port 22 from everywhere; the file above wins first |
| `51_usb_inet.nft` | pmOS | Accepts **everything** coming in over `usb*` |
| `52_hanoip-tailscale.nft` | this repo | Everything on `tailscale0`, UDP 41641 only on `eth*` and `wlan*` |

Do not open a port, weaken a rule, enable password login or add a public-facing service without a decision. To change
a rule on the running phone without wiping Tailscale's own tables, use `nft insert rule` and `nft replace rule`; a
reload of `/etc/nftables.nft` starts with `flush ruleset`.

## The screen

The screen is off from the start of the boot and stays off after every reboot. The kernel cuts the panel supplies
(`vddio`, `lab`, `ibb`, from the LCDB and the PM6150L) and the backlight, and the phone keeps running, with SSH and
Tailscale. This comes from the boot parameter `hanoip_splashfb.screen_off=1`, read by the framebuffer driver (kernel
patch 0005); the image `scripts/mkbootimg-kernel` builds carries it by default.

To turn the screen on again, build the image without the parameter and write `boot_b`:

```sh
KERNEL_CMDLINE="ignore_loglevel fbcon=font:SUN12x22" scripts/mkbootimg-kernel "" boot-screen-on.img
scripts/flash-boot-ssh boot-screen-on.img
scripts/hanoip-sudo reboot
```

Turning it off again is the same with the default image and a reboot. There is no command to switch the screen on a
running system:

- The driver cannot bring the panel back after the supplies are cut, because the DRM/DSI pipeline is not active on this
  kernel. It only returns with a reboot.
- The driver's `fb_blank` hook only touches the backlight and does not blank the screen. Do not use
  `echo 4 > /sys/class/graphics/fb0/blank` to turn it off.
- `dpms` under `/sys/class/drm/card0-DSI-1/` is read-only, and the connector's DPMS property does nothing because no
  CRTC is active.
- The kernel log is still written, it just does not show on the screen. Follow it over SSH.

## Memory, disks and what the kernel offers

- **zram**: zstd, 150 % of RAM, `swappiness=180`, `page-cluster=0`. On a sample of real files zstd packed 2.24x at about
  34 MiB/s of compression, against lz4's 1.65x at about 180 MiB/s; set `deviceinfo_zram_swap_algo` to change it. Not
  measured under real memory pressure.
- **btrfs** is built in and mounts with `compress=zstd`, `zlib` or `lzo` (tested on a loop image; the NAS disk itself is
  issue #19). Avoid RAID5/6 and features a 4.14 btrfs lacks; check `btrfs inspect-internal dump-super` on a disk first.
- **Containers**: user, PID and network namespaces, the `pids` and `devices` cgroups, `br_netfilter` (built in), `overlayfs`,
  `veth`, `macvlan`. Use `crun`: the `runc` of Alpine edge panics on 4.14. Not available: WireGuard (use Tailscale),
  `CFS_BANDWIDTH`, swarm/overlay networks.
- **Watchdog**: `/dev/watchdog` is a software one. `busybox watchdog -T 10 -t 60 /dev/watchdog` (never fed in time)
  restarted the phone in about 45 s. Nothing feeds it at boot yet (issue #17), and it only catches a userspace hang.
- **Kernel modules** load now (patch 0010). The module tree has to come from the same build as the running kernel;
  `scripts/push-modules` copies it and checks the release.

## The NAS disk

A 3.5" SAMSUNG HD502HJ (500 GB, 7200 rpm) behind a USB-SATA bridge. It is formatted on the PC. Its serial stays out of
the repo: identify it with `smartctl -i -d sat <dev>`. The tests so far ran on the PC only; plugging it into the phone
(USB host mode and the drive's own power supply) is still untested (issue #19).

**It has a damaged zone, so it is partitioned around it** (GPT, 512 B logical sectors, 1 MiB aligned):

| Partition | Label | Range | Size |
|---|---|---|---|
| 1 | `hd-a` | 1 MiB to 100 GB | 93 GiB |
| none | | 100 GB to 165 GB | ~65 GB, never partition or write it |
| 2 | `hd-b` | 165 GB to the end | 312 GiB |

Why: from LBA 220053504 (~112.7 GB) the surface fails every ~15 MB and each read of it drops the disk off USB. The rest
that was read (0 to ~105 GB and 150 GiB to the end) read clean at 110 MB/s. The part of the gap from ~113 to ~161 GB was
never mapped. See "Storage" in [`PITFALLS.md`](PITFALLS.md) for the symptoms.

Using it as the homelab storage:

- **Before the first mount on the phone**, check `btrfs inspect-internal dump-super` on each partition: `compat_ro_flags`
  must not have `0x8` (`block-group-tree`), or the 4.14 kernel mounts it read-only. Both partitions had it and were
  converted on the phone on 2026-11-04 with `btrfstune --convert-from-block-group-tree` (the `btrfs-progs-extra`
  package) while unmounted; they now read `0x3`.
- Mount with `compress=zstd,noatime,nofail`, so a missing disk does not hold up the boot. `/etc/fstab` uses
  `LABEL=hd-a` on `/mnt/hd-a` and `LABEL=hd-b` on `/mnt/hd-b`, and both mount points are `chattr +i` while empty.
- Keep only data that is reproducible or has a second copy. The disk is wearing out: watch `Current_Pending_Sector` with
  `smartctl -A -d sat <dev>` on the PC now and then (it was 148 on 2026-11-02, `Reallocated_Sector_Ct` 0). If it keeps
  rising, replace the disk.
- Run `btrfs scrub start <mount point>` regularly: btrfs checksums data, so a bad read is reported instead of passing.
- Unmount before powering the disk off. After a USB drop, power-cycling the drive is what brings it back.

### Sharing the disk (set up 2026-11-04, not yet tested from a client)

- **`hd-a`, Syncthing**: the `obsidian` folder at `/mnt/hd-a/obsidian`, run by the `syncthing` OpenRC service as the
  `syncthing` user. Its state stays on the internal flash (`/var/lib/syncthing`), so it survives the disk dropping. The
  GUI listens on `127.0.0.1:8384` only: reach it with `ssh -L 8384:127.0.0.1:8384`. Global discovery, local discovery,
  relays and NAT mapping are off; peers use the tailnet address (`tcp://<tailscale address>:22000`) of the phone. A
  `.stignore` skips Obsidian's per-device `workspace.json`. The service has `rc_need="localmount"`.
- **`hd-b`, Samba**: the `cloud` share at `/mnt/hd-b/cloud`, `samba.smbd` only (no NetBIOS, no printing), user
  `hdshare` with no shell. Set its password once, interactively: `HANOIP_SSH_TTY=1 scripts/hanoip-ssh 'sudo smbpasswd -a hdshare'`.
  Clients connect to `smb://<tailscale address>/cloud` (Windows: `\\<address>\cloud`), with the Tailscale app on.
  On Android, Material Files (F-Droid) adds an SMB server.
- **Only the tailnet reaches it**: `03_hanoip-samba.nft` accepts 139/445 on `tailscale0` and drops them elsewhere
  (`51_usb_inet.nft` would otherwise let the cable in), and `smb.conf` has `hosts allow` for the Tailscale ranges.
  `smbd` cannot bind to the point-to-point `tailscale0`, so it listens on all addresses and the firewall is the barrier.
  Checked on 2026-11-04: port 445 opens over the tailnet and times out on the LAN address.
- The `hd-b` share holds files that exist nowhere else only until the disk fails: keep a second copy of what matters.

## Updating the phone

The device package (`device-motorola-hanoip`, with the subpackages `-modem` and `-kernel-downstream`) is built on the PC
and copied to the phone; `pmbootstrap` asks for your sudo password, so the person runs it.

```sh
scripts/setup-pmaports                                   # pmaports checkout + this repo's packages
scripts/pmb build --force device-motorola-hanoip         # about a minute; the C and Rust parts build under qemu
pv -L 1m <apk> | scripts/hanoip-ssh 'cat > /tmp/<apk>'   # copy, then compare sha256sum on both ends
scripts/hanoip-sudo 'apk add --allow-untrusted /tmp/<apk> ...'
```

- Install the main apk **together with its subpackages**: the subpackage `-kernel-downstream` owns
  `/usr/share/deviceinfo/deviceinfo`, and a stale one makes a kernel upgrade regenerate the boot images from an old file.
- `apk add` runs `boot-deploy` and `mkinitfs`, and the device package's `deviceinfo` says
  `deviceinfo_flash_kernel_on_update="true"`, so a kernel or device package update writes `boot` and `vendor_boot`
  (there is no `/etc/deviceinfo` override any more). Check after each one: compare
  `head -c <size> /dev/disk/by-partlabel/boot_b | sha256sum` with the image you flashed, and the hash of
  `vendor_boot_b` with your backup.
- **A new kernel**: `scripts/build-kernel` (applies `kernel/downstream-patches` to the pinned source, builds `Image.gz`
  and stages the module tree), `scripts/mkbootimg-kernel "" boot.img`, `scripts/flash-boot-ssh boot.img` (backs up
  `boot_b` to `$HANOIP_BACKUP_DIR` on the PC first, sends the image at 1 MB/s, writes it, reads it back and compares),
  `scripts/hanoip-sudo reboot`, then `scripts/push-modules`. Keep the backup it saved: it is the way back.
- `pmbootstrap` does not need the phone for a kernel package: `scripts/pmb build --envkernel linux-motorola-hanoip`
  packages the output of `scripts/build-kernel`. Do not activate envkernel (it runs `mrproper`).
- Firmware and the modem's data come from the phone itself: `scripts/push-firmware` copies them, never the repository.
- Scripts that change something take `DRY_RUN=1` (print and stop). `scripts/check-repo` is the gate before every
  commit; `scripts/test-*` need no phone.

## Getting it back

- **Slot b is not marked successful**: after about 7 boots the ABL makes it unbootable and switches to slot a, which
  holds the old Lineage. The `local` service runs `qbootctl -m` at every boot to prevent that. If it happened:
  `fastboot getvar slot-unbootable:b` (check first), `fastboot set_active b`, `fastboot reboot`.
- **A bad kernel**: the phone does not come back over the network. Hold Volume Down while powering on, then
  `fastboot flash boot_b <backup>` with the backup `flash-boot-ssh` saved on the PC. A phone that does not boot cannot
  hand its own backup back, which is why they live on the PC.
- **Never write the `misc` backup back as it is** (it starts with `bootonce-bootloader` and would drop the phone into
  the bootloader once); zero those bytes first. If you zero the in-use slot's `dtbo` for a test, restore it before any
  reboot.
- **The battery must not die**: it clears the PMIC data (the cycle counters). The charge limit resumes charging if the
  gauge goes quiet for more than 15 minutes (except while holding on the cable) or the battery is under 3.75 V.
  `scripts/restore-cycle-counts` puts the counters back.
- **Rebooting with the raw `reboot()` syscall** (`scripts/reboot-bootloader` does) skips the clean shutdown, so the boot's
  log is not written. Use `reboot` when the log matters.
- **What recovers by itself, and what does not** (issue #17):

  | Failure | Recovers? | How |
  |---|---|---|
  | Userspace hang | yes | `hanoip-watchdog` feeds `/dev/watchdog` (the kernel's software watchdog, 60 s); when nothing feeds it the phone resets. Tested once: the feeder stopped, the phone reset about a minute later and was back in about 30 s with the services up, slot b successful |
  | Network lost (gateway silent for 5 minutes, no USB link) | yes, up to 3 times in a row | The same service stops feeding on purpose. Not in the first 15 minutes of uptime. After a reset it tells you on Telegram once the network is back; the count clears after 10 minutes of a good network. Tested like the line above, with the gateway forced unreachable; a real router outage was not |
  | Kernel hang | no | A software watchdog needs the kernel; the Qualcomm hardware watchdog has no device node. Power key or fastboot |
  | Power cut | no, not tested | The battery keeps the phone on, so a plug does not reset it. Whether a plugged-in charger turns the phone on from off is untested (needs the phone in hand) |

  Stopping the service is safe: it writes the watchdog's magic close first (tested: the phone stayed up 100 s after a
  stop). Killing it with `kill -9` leaves the watchdog armed and the phone resets in a minute.
