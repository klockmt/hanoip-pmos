#!/data/data/com.termux/files/usr/bin/sh
# Read-only collection of the Moto G60 (hanoip) for the postmarketOS port.
# Usage in Termux: sh coletar-g60.sh
# Nothing is written to the partitions; it only reads them with dd and copies system files.
set -u

# An empty DEST (DEST= sh ...) keeps everything only in $STAGE; the default copies to shared storage.
: "${DEST=/storage/emulated/0/Download/hanoip-kernel}"
STAGE=/data/local/tmp/hanoip-kernel
MIN_FREE_KB=2097152
SELF=$(readlink -f "$0")
ERRORS="$STAGE/errors.txt"

decompress() {
  if command -v zcat >/dev/null 2>&1; then
    zcat "$1"
  else
    gzip -dc "$1"
  fi
}

log_error() {
  echo "$1" >> "$ERRORS"
}

collect_identity() {
  ID="$STAGE/identity"
  mkdir -p "$ID"
  getprop > "$ID/getprop.txt" 2>>"$ERRORS"
  uname -a > "$ID/uname.txt" 2>>"$ERRORS"
  cat /proc/version /proc/cmdline /proc/cpuinfo /proc/meminfo > "$ID/proc.txt" 2>>"$ERRORS"
  tr '\0' '\n' < /sys/firmware/devicetree/base/compatible > "$ID/dt-compatible.txt" 2>>"$ERRORS"
  tr '\0' '\n' < /sys/firmware/devicetree/base/model > "$ID/dt-model.txt" 2>>"$ERRORS"
  cp /sys/firmware/fdt "$ID/fdt.dtb" 2>>"$ERRORS" || log_error "fdt.dtb unavailable"
  for f in /sys/class/power_supply/battery/*; do
    [ -f "$f" ] && echo "$(basename "$f")=$(cat "$f" 2>/dev/null)"
  done > "$ID/battery.txt"
  for f in /sys/class/graphics/fb0/*; do
    [ -f "$f" ] && echo "$(basename "$f")=$(cat "$f" 2>/dev/null)"
  done > "$ID/display.txt"
}

collect_kernel() {
  K="$STAGE/kernel"
  mkdir -p "$K"
  if [ -r /proc/config.gz ]; then
    decompress /proc/config.gz > "$K/config.txt" 2>>"$ERRORS"
  else
    log_error "/proc/config.gz missing: the config will have to be extracted from boot.img"
  fi
  cat /proc/modules /proc/devices /proc/partitions /proc/iomem /proc/interrupts /proc/bus/input/devices > "$K/hardware.txt" 2>>"$ERRORS"
  dmesg > "$K/dmesg.txt" 2>>"$ERRORS" || log_error "dmesg unavailable"
  ls -l /sys/class/input > "$K/input-classes.txt" 2>>"$ERRORS"
  ls -l /sys/class/power_supply > "$K/power-supply-classes.txt" 2>>"$ERRORS"
}

collect_partitions() {
  P="$STAGE/partitions"
  mkdir -p "$P"
  ls -l /dev/block/by-name/ > "$P/list.txt" 2>>"$ERRORS" || log_error "by-name not accessible"
  # A/B device: boot, vendor_boot, dtbo, fsg etc. only exist with a suffix (_a/_b); without it the loop skipped everything.
  suffix=$(getprop ro.boot.slot_suffix)
  for name in boot vendor_boot dtbo vbmeta recovery persist efs modemst1 modemst2 fsg misc; do
    src="/dev/block/by-name/$name"
    [ -e "$src" ] || src="/dev/block/by-name/$name$suffix"
    [ -e "$src" ] || { log_error "partition $name not found"; continue; }
    dd if="$src" of="$P/$name.img" bs=4M 2>>"$P/dd-log.txt" || log_error "failed to read $name"
  done
  (cd "$P" && sha256sum ./*.img > sha256.txt) 2>>"$ERRORS"
}

collect_firmware() {
  F="$STAGE/firmware"
  mkdir -p "$F"
  for src in /vendor/firmware /vendor/firmware_mnt /vendor/lib/firmware /vendor/etc/wifi /vendor/etc/bluetooth; do
    [ -d "$src" ] || continue
    mkdir -p "$F$src"
    cp -r "$src/." "$F$src/" 2>>"$ERRORS" || log_error "partial copy of $src"
  done
  find "$F" -type f > "$F/list.txt" 2>>"$ERRORS"
}

collect_device_tree() {
  mkdir -p "$STAGE/device-tree"
  cp -r /sys/firmware/devicetree/base "$STAGE/device-tree/" 2>>"$ERRORS" || log_error "partial device tree"
}

run_as_root() {
  rm -rf "$STAGE"
  mkdir -p "$STAGE"
  : > "$ERRORS"
  collect_identity
  collect_kernel
  collect_partitions
  collect_firmware
  collect_device_tree
  if [ -n "$DEST" ]; then
    rm -rf "$DEST"
    mkdir -p "$(dirname "$DEST")"
    cp -r "$STAGE" "$DEST" || log_error "failed to copy to $DEST"
  fi
  echo "Collection finished."
  if [ -s "$ERRORS" ]; then
    echo "Warnings:"
    sed 's/^/  - /' "$ERRORS"
  fi
}

main() {
  if [ "${1:-}" = "--root" ]; then
    run_as_root
    exit $?
  fi

  free_kb=$(df -k /data | awk 'NR==2 {print $4}')
  if [ "${free_kb:-0}" -lt "$MIN_FREE_KB" ]; then
    echo "Not enough space in /data: ${free_kb:-?} KB free, ${MIN_FREE_KB} KB needed." >&2
    exit 1
  fi

  echo "Requesting root (grant access in the root manager)..."
  if ! su -c "/system/bin/sh $SELF --root"; then
    echo "Failed to get root or the collection was interrupted." >&2
    exit 1
  fi
  echo "Files in: $DEST"
}

main "$@"
