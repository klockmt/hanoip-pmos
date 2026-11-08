// SPDX-License-Identifier: GPL-2.0-only
/*
 * bounded-loop LOOPDEV FILE OFFSET SIZE
 *
 * Attaches FILE to LOOPDEV starting OFFSET bytes in, limited to SIZE bytes, with one LOOP_CONFIGURE (Linux 5.8+).
 * busybox losetup has no size limit, and the mainline root may only ever be written inside its approved region of
 * super (AGENTS.md), so the initramfs uses this instead. Built static for aarch64 by scripts/mainline-ram-image.
 */
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <linux/loop.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

static int parse(const char *s, uint64_t *v)
{
	char *end;

	errno = 0;
	*v = strtoull(s, &end, 0);
	return errno || *end || end == s;
}

int main(int argc, char **argv)
{
	struct loop_config cfg;
	uint64_t off, size;
	int lfd, ffd;

	if (argc != 5 || parse(argv[3], &off) || parse(argv[4], &size) || size == 0) {
		fprintf(stderr, "usage: %s LOOPDEV FILE OFFSET SIZE (SIZE > 0)\n", argv[0]);
		return 2;
	}
	lfd = open(argv[1], O_RDWR | O_CLOEXEC);
	ffd = open(argv[2], O_RDWR | O_CLOEXEC);
	if (lfd < 0 || ffd < 0) {
		perror("open");
		return 1;
	}
	memset(&cfg, 0, sizeof(cfg));
	cfg.fd = ffd;
	cfg.info.lo_offset = off;
	cfg.info.lo_sizelimit = size;
	if (ioctl(lfd, LOOP_CONFIGURE, &cfg) < 0) {
		perror("LOOP_CONFIGURE");
		return 1;
	}
	return 0;
}
