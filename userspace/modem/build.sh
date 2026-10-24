#!/bin/sh
# Builds static (aarch64) qrtr-ns v1.0 and pd-mapper/tqftpserv with the fallback patches for the 4.14 kernel.
set -eu
CC=${CC:-aarch64-linux-gnu-gcc}
HERE=$(cd "$(dirname "$0")" && pwd)
W=${1:-$HERE/build}
mkdir -p "$W" && cd "$W"
get() { [ -d "$2" ] || curl -sfL "https://codeload.github.com/linux-msm/$1" | tar -xz; }
get qrtr/tar.gz/refs/tags/v1.0 qrtr-1.0
get qrtr/tar.gz/refs/heads/master qrtr-master
get pd-mapper/tar.gz/refs/heads/master pd-mapper-master
get tqftpserv/tar.gz/refs/tags/v1.3 tqftpserv-1.3
patch -d pd-mapper-master -p1 -N < "$HERE/patches/pd-mapper-subsys-fallback.patch" || true
patch -d tqftpserv-1.3 -p1 -N < "$HERE/patches/tqftpserv-subsys-fallback.patch" || true
Q=qrtr-master
LIBQ="$Q/lib/qrtr.c $Q/lib/qmi.c $Q/lib/logging.c"
$CC -static -O2 -s -Iqrtr-1.0/lib -o qrtr-ns qrtr-1.0/lib/logging.c qrtr-1.0/src/addr.c qrtr-1.0/src/ns.c qrtr-1.0/src/map.c qrtr-1.0/src/hash.c qrtr-1.0/src/waiter.c qrtr-1.0/src/util.c
(cd pd-mapper-master && $CC -static -O2 -s -I../$Q/include -o ../pd-mapper pd-mapper.c assoc.c json.c servreg_loc.c $(echo $LIBQ | sed 's|qrtr-master|../qrtr-master|g'))
(cd tqftpserv-1.3 && $CC -static -O2 -s -I../$Q/include -o ../tqftpserv tqftpserv.c translate.c $(echo $LIBQ | sed 's|qrtr-master|../qrtr-master|g'))
ls -l qrtr-ns pd-mapper tqftpserv
