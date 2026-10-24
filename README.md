# postmarketOS on the Moto G60 (hanoip)

Work-in-progress port of postmarketOS to the **Motorola Moto G60** (`hanoip`, XT2135-1, Snapdragon 732G / SM7150), aiming to run it as a headless server.

> **Warning:** this involves flashing `boot`, `vendor_boot` and `userdata` on a device with an unlocked bootloader. Back up every partition first (`scripts/coletar-g60.sh` helps) and expect trial and error. Nothing here is a one-click install.

## Status

| Area | State |
| --- | --- |
| Downstream kernel (4.14, cool585 Cybertron) | **Works.** postmarketOS edge boots on its own with openrc, SSH over the USB network gadget, a 104 GB root on `userdata` and an nftables firewall |
| Wi-Fi (WCN3990) | **Not working yet.** The modem comes up and the Wi-Fi firmware connects to `icnss`, then crashes ~0.7 s later. See [`userspace/modem/README.md`](userspace/modem/README.md) |
| Mainline kernel | **No boot yet.** The DTS builds; the bootloader (ABL) only runs header-v3 boot images, which invalidated the early tests. See [`docs/STATUS.md`](docs/STATUS.md) |

## Layout

| Path | What it is |
| --- | --- |
| `pmaports/` | `device-motorola-hanoip` and `linux-motorola-hanoip` packages for a local pmaports checkout |
| `kernel/downstream-patches/` | Patches for the 4.14 kernel (loop partition rescan, `clk.c` for `DEBUG_FS`, `icnss` BDF/REGDB/`host_cap`) |
| `kernel/dts/` | Mainline DTS for the hanoip |
| `userspace/modem/` | Modem/Wi-Fi userspace for 4.14: `qrtr-ns`, `pd-mapper`, `tqftpserv` (patched), openrc and udev files |
| `scripts/` | Helpers driven by `.env`: ssh, reboot to bootloader, `boot.img` builder, firmware push, vendor mount, device collection |
| `docs/` | Project state and test matrix (`STATUS.md`), verified device facts and sanitized dumps (`device-info/`) |
| `NOTES.md` | Detailed working notes: the pmOS flow, pitfalls already solved, boot test results |

## Configuration

Local paths and the SSH user are not in the repo. Copy `.env.example` to `.env` and fill it in; the scripts read it.

## What is not in this repo

Partition backups, the device collection (serial, MAC and IMEI), and the proprietary firmware from `/vendor`. Collect them from your own device with `scripts/coletar-g60.sh`.
