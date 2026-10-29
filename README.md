# postmarketOS on the Moto G60 (hanoip)

Work-in-progress port of postmarketOS to the **Motorola Moto G60** (`hanoip`, XT2135-1, Snapdragon 732G / SM7150), aiming to run it as a headless server.

> **Warning:** this flashes `boot`, `vendor_boot` and `userdata` on a device with an unlocked bootloader, and **erases the phone**. Back it up first (step 2). Only slot `b` was tested. Nothing here is risk-free.

## Status

| Area | State |
| --- | --- |
| Downstream kernel (4.14, cool585 Cybertron) | **Works.** postmarketOS edge boots on its own with openrc, SSH over the USB network gadget, a 104 GB root on `userdata` and an nftables firewall |
| Wi-Fi (WCN3990) | **Not working yet.** The modem comes up and the Wi-Fi firmware connects to `icnss`, then crashes ~0.7 s later. See [`userspace/modem/README.md`](userspace/modem/README.md) |
| Mainline kernel | **No boot yet.** The DTS builds; the bootloader (ABL) only runs header-v3 boot images, which invalidated the early tests. See [`docs/STATUS.md`](docs/STATUS.md) |

Without Wi-Fi the phone is reachable only over the USB cable (or a USB Ethernet adapter, which the kernel supports: RTL8152, ASIX, CDC).

## Tutorial: reproduce the working image

Everything is scripted. Steps marked **[manual]** need your hands on the phone or an interactive prompt. The scripts that change something (`setup-pmaports`, `configure-pmbootstrap`, `build-image`, `setup-usb-net`, `flash-rootfs`, `flash-boot`) accept `DRY_RUN=1` (print the commands, change nothing), and `ASSUME_YES=1` skips their questions.

### What you need

| | |
| --- | --- |
| Phone | Moto G60 `hanoip` (XT2135-1) with an **unlocked bootloader** (Motorola's unlock procedure erases it) |
| On the phone | Root `adb` for the backup (KernelSU or Magisk), and a **recovery with adb that can write `userdata`**. The author's phone ran **LineageOS 22.1** and used its recovery, so install that first. Motorola's fastboot refuses to flash `userdata`, which is why a recovery is needed |
| Host | x86-64 Linux, internet, a USB cable, about 11 GB of free disk (4.5 GB kernel tree + 6 GB pmbootstrap), `sudo`. 16 threads recommended |
| Host tools | adb, fastboot, mkbootimg (android-tools), clang, lld, llvm, make, bc, bison, flex, dtc, openssl headers, pv, python3, NetworkManager (optional). `scripts/check-host` lists what is missing |
| pmbootstrap | 3.12.1: `python3 -m venv ~/.local/share/pmbs-venv && ~/.local/share/pmbs-venv/bin/pip install pmbootstrap==3.12.1` |

### 1. Get the repo and configure

```sh
git clone https://github.com/klockmt/hanoip-pmos.git hanoip && cd hanoip
cp .env.example .env      # [manual] set HANOIP_USER (the login name you will use) and adjust paths if you like
scripts/check-host
```

`.env` holds your user name and local paths. It is not in git. Scripts read it; variables already set in your shell win over it.

### 2. Back up the phone

Phone booted in Android, USB debugging on, root granted to adb:

```sh
scripts/backup-partitions
```

Saves `boot`, `vendor_boot`, `dtbo`, `vbmeta`, `persist`, `modemst`, `fsg`, `misc` and `abl` of both slots, plus `SHA256SUMS`, into `$HANOIP_BACKUP_DIR`. It only reads, refuses to run on anything but a hanoip and never overwrites an existing backup. **Keep these files and never publish them** (they hold your serial, MACs and IMEI). You will need `vendor_boot_b` to get back into the recovery.

### 3. Build the kernel

```sh
scripts/build-kernel
```

Downloads cool585's kernel at the pinned commit, applies the three patches in [`kernel/downstream-patches/`](kernel/downstream-patches), uses the config from [`pmaports/`](pmaports/device/downstream/linux-motorola-hanoip) and builds `Image.gz` with clang. About 5 minutes on 16 threads, including the 200 MB download.

### 4. Prepare pmbootstrap

```sh
scripts/setup-pmaports            # official pmaports at the pinned commit + our two packages
scripts/pmb init                  # [manual, once] pmbootstrap with the paths from .env; answers below
scripts/configure-pmbootstrap     # device, downstream kernel, openrc, SSH key, user
```

In `init`, pick device `motorola-hanoip`, the same user name as `HANOIP_USER`, and a console-only setup (no graphical UI). The other questions are yours. `configure-pmbootstrap` then forces the settings this port depends on (the downstream kernel, openrc because 4.14 cannot run systemd, and the SSH key) and creates a dedicated passwordless SSH key at `$HANOIP_SSH_KEY` if you have none.

### 5. Build the image

```sh
scripts/build-image
```

Packages the kernel from step 3, builds the device package and runs `pmbootstrap install`. It asks for your `sudo` password and for the password of the phone user **[manual]**. Result in `$PMB_WORK`: the rootfs (~2 GB; pmOS grows it to fill `userdata` on first boot), `boot.img` (only the kernel) and `vendor_boot.img` (initramfs, cmdline and DTB).

### 6. Flash

**6a. The rootfs, through the recovery.**

1. **[manual]** Boot the LineageOS recovery. From Android: `adb reboot recovery`. If you already flashed pmOS before, restore the recovery first: `fastboot flash vendor_boot_b "$HANOIP_BACKUP_DIR/vendor_boot_b.img"` and `fastboot reboot recovery`. Do not zero or replace `dtbo`: that recovery needs it.
2. **[manual]** In the recovery menu: *Advanced → Enable ADB*. `adb devices` must list the phone.
3. Run:

```sh
scripts/flash-rootfs
```

It checks the phone is a hanoip, asks before erasing, writes the image to `userdata`, then reads it back and compares SHA-256.

**6b. vendor_boot and boot, with a test first.**

```sh
adb reboot bootloader            # or hold Volume Down while powering on
scripts/flash-boot
```

It flashes `vendor_boot_b`, then does a **temporary** `fastboot boot` of the new `boot.img` and waits for SSH (it sets up the PC's USB network by itself once the interface appears). Only if SSH answers does it reboot to the bootloader (the phone's `sudo` asks for the user password **[manual]**), flash `boot_b` and reboot. If SSH never comes up, `boot_b` is left untouched.

### 7. Use it

```sh
scripts/hanoip-ssh                # log in as HANOIP_USER at 172.16.42.1
scripts/hanoip-ssh 'uname -r'
```

The PC address is `172.16.42.2/24` on the phone's USB interface (`scripts/setup-usb-net` recreates it; it is a NetworkManager connection named `hanoip-usb`).

On the phone, `hanoip-status` prints the latency to the gateway, the traffic on the default link, battery, RAM, root disk and CPU use. `hanoip-charge-limit` (an OpenRC service, on by default) keeps the battery between 60 % and 80 %: it cuts the charger and suspends the cable above 80 %, so the phone runs from the battery, and turns both back on below 60 %. Stopping the service turns the charger back on. While the cable charges it holds the input near 10 W (`main/force_main_icl`, 1.1 A). `hanoip-thermal` (also on by default) watches the battery temperature: from 40 °C it pauses charging, from 45 °C it also caps the CPU at 60 % of its top frequency, and at 55 °C it tells the owner on Telegram and powers off. Check the decisions without hardware with `scripts/test-charge-limit` and `scripts/test-thermal`; both ran on the phone from the repository (not yet as an installed package), see `docs/BATTERY.md` for what was measured.

### 8. Update only the kernel later

```sh
scripts/build-kernel
scripts/mkbootimg-kernel                 # boot.img with only the kernel
scripts/reboot-bootloader
fastboot boot boot.img                   # temporary test, nothing flashed
fastboot flash boot_b boot.img           # if it came up fine
```

`vendor_boot` only changes when you change the initramfs, the cmdline or the DTB (then re-run `scripts/build-image` and `scripts/flash-boot`).

### Going back

Restore from your backups: `fastboot flash boot_b "$HANOIP_BACKUP_DIR/boot_b.img"`, same for `vendor_boot_b`, then reflash or restore Android as usual. Do not write the `misc` backup back as it is (it can send the phone to the bootloader once); zeroing it is safer.

## What was verified, and how

I want to be clear about what has and has not been run end to end.

| Step | How it was checked |
| --- | --- |
| `check-host` | Ran on the author's machine |
| `build-kernel` | **Ran from scratch** in an empty directory from the official download: the 3 patches applied, the kernel built (4 min 19 s) and the result **booted pmOS to SSH** with `fastboot boot` |
| `setup-pmaports` | **Ran for real**; the checkout it builds is byte-identical to the author's pmaports that produced the working image |
| `mkbootimg-kernel`, `reboot-bootloader`, `hanoip-ssh` | Used on the real phone in that boot test |
| `build-image`, `configure-pmbootstrap` | Dry-run only (they need `sudo` and an interactive `init`). The commands are the ones the author ran to produce the images that boot. The package build **without** `--envkernel` (letting pmbootstrap compile the kernel itself) was never tried |
| `backup-partitions`, `flash-rootfs`, `flash-boot` | Tested against a simulated phone, including the refusals (wrong device, not in recovery, corrupted write, no SSH). The underlying `dd`, `adb` and `fastboot` commands are the ones used on the real phone |
| `setup-usb-net` | Interface detection and the `nmcli` command checked; same command the author used |
| A completely fresh machine, start to finish | **Not done.** Expect to hit something |

## Layout

| Path | What it is |
| --- | --- |
| `pmaports/` | `device-motorola-hanoip` and `linux-motorola-hanoip` packages, and `PMAPORTS_COMMIT`, the official pmaports commit they were built on |
| `kernel/downstream-patches/` | Patches for the 4.14 kernel (loop partition rescan, `clk.c` for `DEBUG_FS`, `icnss` BDF/REGDB/`host_cap`) |
| `kernel/dts/` | Mainline DTS for the hanoip |
| `userspace/modem/` | Modem/Wi-Fi userspace for 4.14 (experimental, Wi-Fi still crashes): `qrtr-ns`, `pd-mapper`, `tqftpserv` (patched), openrc and udev files |
| `scripts/` | All the steps above, `check-repo` (self-check to run before every commit: checksums, patch copies, private-data scan), `push-firmware` and `mount-vendor` for the Wi-Fi investigation, and `coletar-g60.sh` (runs in Termux on the phone) |
| `docs/` | Project state and test matrix (`STATUS.md`), verified device facts and sanitized dumps (`device-info/`) |
| `NOTES.md` | Detailed working notes: pitfalls already solved, boot test results |

## Pitfalls already solved (details in `NOTES.md`)

- The bootloader only runs **Android boot header v3**; the DTB, initramfs and cmdline live in `vendor_boot`, not in `boot`.
- The ramdisk must be **LZ4 legacy**; gzip or zstd leave the phone stuck on the Motorola logo.
- The UFS has **4096-byte sectors**: the rootfs GPT must match, and the 4.14 loop driver needs patch `0001` to see its partitions.
- Large continuous transfers over the USB network gadget bring it down; `scripts/push-firmware` limits itself to 1 MB/s.
- **The A/B slot must be marked successful at every boot** (`qbootctl`, installed by the device package), or after ~7 reboots the bootloader invalidates the slot and switches to the other one. Recover with `fastboot set_active b`.
- The bootloader rewrites every `console=` to `console=null`; patch `0004` turns that into the screen console, and `0005` draws it on the bootloader's framebuffer (the MSM fbdev emulation cannot work with this driver).
- The kernel must not use `CONFIG_EFI_ZBOOT`, and `FW_LOADER_USER_HELPER_FALLBACK` must be off (otherwise every missing firmware file costs 60 s of boot).

## What is not in this repo

Partition backups, the device collection (serial, MAC and IMEI), and the proprietary firmware from `/vendor`. Take them from your own phone with `scripts/backup-partitions` and `scripts/coletar-g60.sh`.

## License

GPL-2.0-only, see [`LICENSE`](LICENSE). The kernel patches and the device-tree sources are derived from the Linux kernel; the packaging, scripts and documentation use the same license. Proprietary firmware and device identifiers are not part of the repository.
