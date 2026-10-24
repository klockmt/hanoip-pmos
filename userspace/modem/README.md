# Modem and Wi-Fi on the downstream kernel (4.14)

The hanoip Wi-Fi (WCN3990, `qcacld-3.0` driver + `icnss`) runs as a protection domain (`wlan_pd`) inside the modem, so the modem has to be brought up. The Linux tools for that (`qrtr`, `pd-mapper`, `tqftpserv`, `rmtfs`) expect a mainline kernel, and three things are missing on 4.14:

| Missing on 4.14 | Effect | What we do |
| --- | --- | --- |
| QRTR name server in the kernel | no QMI service finds another | `qrtr-ns` from tag v1.0 (current qrtr no longer ships the daemon) |
| `/sys/class/remoteproc` (here it is `msm_subsys`) | `pd-mapper` cannot find the `.jsn` files; `tqftpserv` rejects `/readonly/...` | patches with a fallback to `/lib/firmware` |
| nothing starts the modem by itself | the subsystem only comes up while `/dev/subsys_modem` is open | `hanoip-modem` service |

## Files

- `build.sh`: builds static (aarch64) `qrtr-ns`, `pd-mapper` and `tqftpserv` from the linux-msm sources plus `patches/`. Install them in `/usr/local/bin/`.
- `etc/init.d/`: `qrtr-ns`, `pd-mapper`, `tqftpserv-legacy` (replaces Alpine's `tqftpserv`; take Alpine's out of the `boot` runlevel) and `hanoip-modem`.
- `etc/conf.d/rmtfs`: Alpine's `rmtfs` (`apk add rmtfs rmtfs-openrc`) without `-s` and with `-r` (read-only: never writes to `modemst1/2`/`fsg`).
- `etc/udev/rules.d/90-hanoip-modem.rules`: the `/dev/qcom_rmtfs_uio1` and `by-partlabel/fsg` (to `fsg_b`) links.

## Data that comes from the device (never goes in the repo)

- Firmware in `/lib/firmware`, from `/vendor/firmware_mnt/image` (`modem.*`, `wlanmdsp.mbn`, `bdwlan.*`, `regdb.bin`, `ipa_fws.*`, `*.jsn`, `mcfg_*`, `mbn_*`, `qdsp6m.qdb`, `adsp.*`, `cdsp.*`) and `WCNSS_qcom_cfg.ini` in `/lib/firmware/wlan/qca_cld/`. `scripts/push-firmware` copies them.
- `/var/lib/tqftpserv/` (the modem's `/readwrite`): a copy of `rfs/msm/mpss/` from the `persist` partition, read with `mount -o ro,noload`. It holds device data (IMEI SV, simlock).

## Bring-up order (what `hanoip-modem` does)

1. `echo 1 > /dev/ipa` and wait for `rmnet_ipa0` to appear: without IPA ready first, the modem crashes (`EX:kernel:0x0:tmr_slave1`).
2. Set the modem's `restart_level` to `RELATED`: a modem crash does not reboot the device.
3. Open `/dev/subsys_modem` and keep it open.

## State (2026-10-24)

- Works: modem `ONLINE`, EFS read by `rmtfs`, `servloc`, `sysmon` and `service-notifier` connected, IPA initialized, `wlan_pd` loaded through TFTP and WLFW connected to `icnss`.
- **Does not work yet:** about 0.7 s after `icnss_qmi: QMI Server Connected`, the firmware sends `Received early crash indication from FW` and the modem crashes (`EX:kernel:0x0:tmr_slave1:0x1d1:PC=0xc06ddec0`), in a loop. The `icnss` state reaches `MSA0_ASSIGNED`/`POWER_ON` but not `FW_READY`. On Android, `FW ready` arrives 2.6 s after the connection.
  - Already ruled out: the DT (equal to Android's, `wlan_msa_region` included), IPA order, and a missing `fsg` or `/readwrite`.
- With `CONFIG_DEBUG_FS` (test kernel #6; it needs `kernel/downstream-patches/0002-clk-drop-duplicate-all_lists-for-debugfs.patch`), `/sys/kernel/debug/icnss/stats` and the IPC log `ipc_logging/icnss/log` show:
  - the QMI handshake finishes **completely and without errors**: `ind_register`, `msa_info` (MSA `0x97500000`, regions `0x4000` + `0x17c000`), `msa_ready` with its indication, `cap` and `dynamic feature mask`;
  - the firmware capabilities: chip `0x140`, `board_id 0xff`, `WLAN.HL.3.2.2-00947`;
  - the firmware crashes by itself about 650 ms later, without asking the kernel or TFTP for anything.
- `wlanmdsp.mbn` arrives whole through TFTP (`tqftpserv -d`: 519 blocks, 3,983,056 bytes, last block acknowledged). `bdwlan.bin` is identical to `bdwlan_hanoip_ipa.bin`.
- Dumps: `ramdump_wcss_msa0` comes back all zeros and `microdump_modem` is empty. The full modem ramdump is not produced because the modem stays in `restart in progress` instead of restarting by itself.
- The mainline SM7150 ports (sweet, sunfish) use `ath10k`, where the kernel sends the board data over QMI. Their userspace is just `rmtfs`/`pd-mapper`/`tqftpserv`/`qrtr`, so the firmware does not depend on Android services.
- The Android vendor was read read-only (`vendor_b` is a linear extent of `super`: sector 18258944, 1313712 sectors of 512 B; `losetup -r -o ... --sizelimit ...` + `mount -o ro,noload`, ext4; `scripts/mount-vendor` does it). On Android, `cnss-daemon` is what sends the board data and the REGDB to the firmware over QMI. The kernel's `icnss` does not.
- `kernel/downstream-patches/0003-icnss-...patch` plays the same role in the kernel:
  - `host_cap` with `daemon_support = 0`, encoded in 4 bytes: the 4.14 `ei` uses 1 byte, and with it the firmware answers `MALFORMED`;
  - `regdb.bin` (`bdf_type 4`), then `bdwlan.bin`, then `cal_report`.
  - The firmware **accepts** all of it (test kernel #10), but it **still crashes** about 0.7 s after the connection. The crash does not depend on the QMI messages.
- Also ruled out: ADSP and CDSP started before the modem (firmware copied to `/lib/firmware`; nothing changed); `rmtfs -r` (it keeps writes in a buffer and hands them back on reads); the `cxo_ref_clk_pin` clock (it does not exist in the DT, nor on Android).
- The exception PC (`0xc06ddec0`) is in the modem's own code segment (QuRT, `EX:kernel`), not in `wlanmdsp`.
- QMI services that Android hosted and pmOS does not: `PerMgrSrv` (`pm-service`) and the IMS ones. `cnss-daemon` is only a client.
- That is why `hanoip-modem` is **outside** the `default` runlevel. To test it: `rc-service hanoip-modem start`. The modem does not restart well within the same session, so every test needs a clean boot.
