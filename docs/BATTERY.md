# Battery life, component safety and heat on the Moto G60 (hanoip)

Research of 2026-10-29 for the phone as an always-on server on the cable. Device readings come from the
running phone (downstream 4.14, Cybertron); kernel facts come from the downstream source in
`drivers/power/supply/qcom/` (`qpnp-smb5.c`, `smb5-lib.c`, `pmic-voter.c`). What was not measured on the
phone is marked **unverified**.

## Summary

| Question | Answer |
|---|---|
| Can the battery be bypassed? | **Yes, measured (2026-10-29).** With `charging_enabled=0` and `input_suspend=0` the battery current is 0 µA and the cable feeds the phone. See section 1. |
| Limit the charge to about 10 W? | Done in `hanoip-charge-limit` with `main/force_main_icl` (1.1 A). The knob works on the phone; the 1.1 A figure itself was not seen biting. See section 2. |
| Heat protection? | Done in `hanoip-thermal`: 40 °C pauses charging, 45 °C caps the CPU, 55 °C powers off. Pause, cap and alert ran on the phone with lowered thresholds; the power-off did not. See section 3. |
| Discharge floor? | Already there: below 3.75 V the charge limit resumes charging. Heat does not override it. |
| `powersave` and fewer cores for a DNS server? | Use `schedutil` (applied), keep all cores. See section 5. |

## 1. Bypass (pass-through)

Today the limiter cuts at 80 % with `charging_enabled=0` **and** `input_suspend=1`. The second one cuts the
cable, so the phone runs from the battery and the cell cycles 80 % to 60 % to 80 %. That is not bypass.

What the source says:

- `battery/charging_enabled` sets `chg_disable_votable` (USER_VOTER) only: it stops the charge and leaves the
  input alone (`smb5_batt_set_prop`, `POWER_SUPPLY_PROP_CHARGING_ENABLED`).
- `battery/input_suspend` votes 0 mA on `usb_icl_votable` and suspends DC (`smblib_set_prop_input_suspend`).
- So `charging_enabled=0` with `input_suspend=0` is a charger with the input on and the charge off. On a
  Qualcomm power path that is how phones with "bypass" or "idle mode" work in software.

**Measured on 2026-10-29** (cable on, battery at 81 %, limiter stopped): with `charging_enabled=0` and
`input_suspend=0`, `battery/current_now` read 0 µA on four samples 5 s apart, the status was "Not charging",
and the cable supplied the phone (about 1 W at the cable, the phone's own draw). The battery voltage only
relaxed (4.24 V falling slowly, no load on it). With `charging_enabled=1` the same cable gave 1.0 A at 8.5 V
(about 9 W) and the battery took 1.5 A. So bypass works on this phone and the `charging_enabled` switch is
enough. `hanoip-charge-limit` now uses it (see below).

The test, for repeating (needs root; stop the limiter first, or it cuts the cable again within 30 s):

1. `rc-service hanoip-charge-limit stop`, then with the cable connected and the battery under 80 %:
   `echo 0 > /sys/class/power_supply/battery/charging_enabled` (leave `input_suspend` at 0).
2. For a few minutes read `battery/current_now` (µA), `battery/voltage_now` and `usb/input_current_now`.
3. Bypass works if the cable keeps supplying the phone (`usb/input_current_now` clearly above 0) and
   `battery/current_now` stays near 0 with a stable voltage. If `current_now` is negative, the battery is
   feeding the phone and it is only a suspension.
4. `rc-service hanoip-charge-limit start` (stopping or starting it also restores `charging_enabled=1` and
   `input_suspend=0`).

**Limiter, as built.** At 81-82 % with the cable on it sets `charging_enabled=0` and keeps the input on (hold).
Above 82 % (the end of a full charge, for example) it first suspends the cable so the phone drains the battery
into that band, then holds. Below 60 % charging comes back. The 10-cycle full-charge calibration stays, but a
cycle now only happens when the cable was out or the load really sat on the battery.

Run live on the phone (the real script, 7 minutes): the charge ran at 9.26 W at the cable with the 1.1 A cap
(battery 1.6 A), and at 81 % it stopped by itself: battery current 0, status "Not charging", the cable feeding the
phone at 0.75-1.2 W, the battery voltage relaxing from 4.25 to 4.22 V and the battery temperature falling from
31 to 30 °C. Two things came out of it, both handled and tested:

- `hanoip-qgd` writes nothing while the battery does not move (the kernel sends no samples): the file stood
  still for 7 minutes. The staleness guard (15 minutes) now ignores that while holding on the cable.
- The charger can keep saying Full after a finished charge, so `hanoip-qgd` counts Full only from 4.30 V.

The Fairphone forum thread on idle mode proposes the same `charging_enabled` test and warns that some
phones do not support it ([thread](https://forum.fairphone.com/t/help-wanted-battery-idle-mode-with-root-but-without-acc/93545)),
and Android Authority lists the feature per phone; the Moto G60 is not confirmed in any source found.

## 2. Charge speed (about 10 W)

Nodes on this kernel (read on the phone):

| Node | Result |
|---|---|
| `battery/current_max`, `battery/constant_charge_current_max`, `usb/current_max` | read-only |
| `battery/system_temp_level`, `charge_control_limit` | writable, but the boot log says `usb-thermal-mitigation is not set`, so there is no table: writes fail with EINVAL |
| `main/force_main_fcc` | writable: only forces the main charger's share of the charge current |
| **`main/force_main_icl`** | **writable: overrides the input current limit** |

`force_main_icl` calls `vote_override(usb_icl_votable, CC_MODE_VOTER, ...)` (`qpnp-smb5.c`). Three facts from
`pmic-voter.c` and `smb5-lib.c` shape the code:

- An override replaces the election result. It therefore **beats the input_suspend vote** (0 mA). The limiter
  releases it (`-1`) before every cut.
- A negative value releases it, and the kernel clears it on unplug (`typec_src_removal`), so the
  limiter writes it again on every pass.
- It also beats the type limit. On a 0.5 A source it would raise the current, so it is only written when
  `usb/hw_current_max` is above the limit.

The phone negotiates PD at about 8.5-9 V, so 1.1 A is about 9.4-9.9 W at the cable.

**Measured on 2026-10-29**, the real `hanoip-charge-limit` running on the phone with the limit set to 0.7 A:
the cable drew 0.69 A, 6.0 W, against 1.0 A, about 8.6 W, with no limit (the register reads back what was
written, and 2000000 once released). So the knob works and `usb/input_current_now` x `usb/voltage_now` is the
way to read it. The battery was at the top of the charge (4.45 V under charge, current already tapering), so
the charge only drew 1.0 A even without a limit, and the 1.1 A cap did not change it (8.3-8.7 W against 8.5-9.1 W).
Check it biting at 60-70 %, where the charge is not tapering and the unlimited cable should draw about 1.9 A.
`smb1390`/`smb1355` stayed out of the way in these readings.

`usb/input_current_now` also reads 0.3-0.8 A while the cable is suspended and the battery is supplying the
phone (battery current negative), so it is not a measure of power drawn while `input_suspend=1`.

The kernel-side alternative is the SMB5 thermal mitigation table, which needs a device tree change. In mainline,
Casey Connolly's `qcom_smbx` series lets userspace stop the charge by writing 0 to `status`, and says current
limiting by temperature is missing from the kernel
([series](https://lkml.indiana.edu/hypermail/linux/kernel/2506.2/06610.html)).

## 3. Heat

Measured on the phone, idle: battery 28 °C, CPU 33-38 °C, charger die 30 °C (limit 45 °C).

- The kernel trip points are far too late for a cell: CPU zones 115-125 °C, `pm6150-tz` 95 °C, and the
  `battery` and `bms` zones have no trip points at all. No userspace thermal daemon runs.
- Soft JEITA and step charging are off (`sw_jeita_enabled=0`, `step_charging_enabled=0`, and the boot log says
  `Couldn't get fcc values for soft JEITA`). The PMIC's hardware JEITA is the only charge-side protection.
- Battery University gives 0 to 45 °C as the lithium-ion charging window, 10 to 30 °C as best, and 60 °C as the
  discharge limit ([BU-410](https://batteryuniversity.com/article/bu-410-charging-at-high-and-low-temperatures)).
  Time at high charge and high temperature ages a cell faster than cycling
  ([BU-808](https://hn.jakechampion.name/reader/https://www.batteryuniversity.com/article/bu-808-how-to-prolong-lithium-based-batteries)).

`hanoip-thermal` therefore watches `battery/temp` every 30 s (values are guesses to calibrate with real data):

| Battery temp | Action |
|---|---|
| 40 °C | flag on: `hanoip-charge-limit` cuts charger and cable (not counted as a cycle) |
| 45 °C | CPU capped at 60 % of its top frequency |
| 55 °C, two readings in a row | alert through the bot, then `poweroff` |

On the phone, with the real scripts and the thresholds lowered to 25 °C (hot) and 26 °C (cap) against a 27 °C
battery: the cap wrote 1382400 kHz (60 % of 2304000) on the big cluster, the charger and the cable were cut,
the override was released, one Telegram-style alert line was produced, and when the flag went away charging
came back with the 0.7 A limit applied again. The 55 °C power-off and the real bot alert did not run there.

Each level lifts 3 °C under its threshold. The flag counts only while fresh (5 minutes), and under 3.75 V the
charge comes first, so a hot reading cannot drain the battery into the PMIC data loss of 2026-10-28.
The CPU die temperature is not used: the load is low, so the battery is the slow and critical sensor. Adding
die thresholds (say 85 °C cap, 100 °C off) is a follow-up if a heavy workload ever runs.

## 4. Discharge floor and voltage

The 3.75 V resume (about 20 %) already exists and now also wins over the heat pause. Battery University puts
the longevity-optimal peak near 3.92 V per cell and says every 0.10 V less on the peak roughly doubles the
cycle life. This cell charges to 4.45 V, so the 80 % ceiling is already well below its peak. `hanoip-status`
(and so Telegram `/status`) now shows the voltage.

## 5. CPU: governor and cores for a DNS server

Readings: the governor is `performance` on both clusters, so the CPU sits at 1.8 GHz and 2.3 GHz all the time,
with 8 cores online and a load of 0.03. Governors available: `conservative ondemand userspace powersave
performance schedutil`.

- `performance` pins the top frequency and `powersave` the lowest, within the min and max limits
  ([cpufreq governors](https://www.kernel.org/doc/Documentation/cpu-freq/governors.txt)). `powersave` here means
  300 MHz always: fine for the average DNS query, but slower on bursts and anything cryptographic (DoT/DoH).
  Not recommended.
- **`schedutil`** follows the scheduler's load and is the default governor on arm64
  ([Kconfig](https://tangled.org/tjh.dev/kernel/blob/v6.19/drivers/cpufreq/Kconfig)). At idle it drops to the
  lowest step and climbs on demand, so the idle heat and draw fall and DNS latency barely moves. This is the
  change worth making. Applied on the phone on 2026-10-29: the little cluster dropped to 1.0 GHz and the big one
  to 300 MHz at idle. A boot script (`hanoip-cpufreq.start`) makes it stick.
- Offlining cores is not worth it. The kernel's hotplug page covers only the mechanics; whether an offlined core
  saves power depends on the hardware, and idle cores already enter cpuidle states (`state0-2` exist here)
  ([hotplug](https://www.kernel.org/doc/html/v6.8/core-api/cpu_hotplug.html),
  [Arm note](https://community.arm.com/oss-platforms/w/docs/529/cpuidle-hotplug)). It adds migration latency
  and gains little. A cap on the big cluster's top frequency is the safer lever if heat is ever a problem, and
  `hanoip-thermal` already does it past 45 °C.

DNS is I/O and latency bound, not compute bound; the real risks for it are the USB network drop and the
Tailscale DNS loop already in `NOTES.md`, not the CPU.

## Not verified

- The 1.1 A cap biting during the constant-current part of the charge (section 2).
- The 55 °C power-off and the real Telegram alert on the phone; the thresholds in section 3 are guesses.
- The new scripts installed as a package: the live tests ran them from the repository, not from `pmbootstrap`.
- The Moto G60 on any "bypass charging" list: no source found either way.
