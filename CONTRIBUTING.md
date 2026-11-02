# Contributing

This is a postmarketOS port to one phone (Motorola Moto G60, `hanoip`), run as a headless server. Help is welcome:
reports from your own unit, fixes, tests and docs. Read [`docs/STATUS.md`](docs/STATUS.md) (what works),
[`docs/PITFALLS.md`](docs/PITFALLS.md) (what broke and where it is kept from breaking again) and
[`docs/HOMELAB.md`](docs/HOMELAB.md) (how the phone is run) first.

## Never put these in an issue, a commit, a log or a fixture

An IMEI, a serial number, a MAC address, a battery id, a Wi-Fi name, a token or key, your user name, a home path, a
partition dump or a device dump. Personal values live in your `.env` (copy `.env.example`; `.env` is git-ignored) and
the scripts and docs refer to the variable, never to the value. `scripts/check-repo` looks for these patterns and fails
when it finds one; run it before every commit. If you find a value that is already in the repository, say so in an issue
**without repeating it**.

## Reports

Use the templates: *Bug* for something that does not work, *Hardware report* for what happened when you tried a step on
your own phone. Say what you ran, what you saw (sanitized), and what you did not check. Report failures as they are.

## Changes

- **Small commits, one per change**, `type(scope): summary` in English, imperative, lowercase after the colon, no
  trailing period, 72 characters at most (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`). The body says why the
  change exists. The author is your own git identity.
- **Every bug gets an automated test that fails before the fix and passes after**, in the same commit or right before.
  Tests live in `scripts/test-*`, need no phone, and `scripts/check-repo` runs all of them. Show the test failing on the
  old behaviour, then passing.
- **Scripts that change something take `DRY_RUN=1`** (print the commands, change nothing). Keep the README honest about
  which scripts were run end to end and which only in a dry run.
- **Do not flash partitions** beyond what the change needs, never wipe or factory reset, and back up to the PC before any
  flash (`scripts/flash-boot-ssh` does). Do not weaken the firewall, open a port or enable password login in a change
  without saying so in the description.
- Docs: each fact has one home (see the three files above); link to it instead of copying it. Say what was and was not
  tested.

## Pull requests

Open them against `beta`. `main` only receives what was tested on the phone and approved by the owner, and releases are
tagged from it. Run `scripts/check-repo` and paste its last line.

## Licence

GPL-2.0-only for everything (see `LICENSE` and `NOTICE`): the kernel patches and device-tree sources derive from Linux, and
the packaging, scripts and docs use the same licence.
