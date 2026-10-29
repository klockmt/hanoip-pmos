# Battery life, component safety and heat on the Moto G60 (hanoip)

Research of 2026-10-29 for the phone as an always-on server on the cable. Device readings come from the
running phone (downstream 4.14, Cybertron); kernel facts come from the downstream source in
`drivers/power/supply/qcom/` (`qpnp-smb5.c`, `smb5-lib.c`, `pmic-voter.c`). What was not measured on the
phone is marked **unverified**.

## Summary

| Question | Answer |
|---|---|
| Can the battery be bypassed? | **Probably, unverified.** The PMIC can keep the cable on and stop charging (`charging_enabled=0`, `input_suspend=0`), which is pass-through by design, but nobody measured it here. See section 1. |
| Limit the charge to about 10 W? | Done in `hanoip-charge-limit` with `main/force_main_icl` (1.1 A). **Not run on the phone yet.** See section 2. |
| Heat protection? | Done in `hanoip-thermal`: 40 °C pauses charging, 45 °C caps the CPU, 55 °C powers off. **Not run on the phone yet.** See section 3. |
| Discharge floor? | Already there: below 3.75 V the charge limit resumes charging. Heat does not override it. |
| `powersave` and fewer cores for a DNS server? | Use `schedutil`, keep all cores. See section 5. |

## 1. Bypass (pass-through)

Today the limiter cuts at 80 % with `charging_enabled=0` **and** `input_suspend=1`. The second one cuts the
cable, so the phone runs from the battery and the cell cycles 80 % to 60 % to 80 %. That is not bypass.

What the source says:

- `battery/charging_enabled` sets `chg_disable_votable` (USER_VOTER) only: it stops the charge and leaves the
  input alone (`smb5_batt_set_prop`, `POWER_SUPPLY_PROP_CHARGING_ENABLED`).
- `battery/input_suspend` votes 0 mA on `usb_icl_votable` and suspends DC (`smblib_set_prop_input_suspend`).
- So `charging_enabled=0` with `input_suspend=0` is a charger with the input on and the charge off. On a
  Qualcomm power path that is how phones with "bypass" or "idle mode" work in software. **Unverified** on this
  board: the earlier commit `fc05145` saw the battery stay at 100 % on the cable after a cut, but the gauge
  was frozen at the time, so it proves nothing.

Test to decide (needs root; stop the limiter first, or it cuts the cable again within 30 s):

1. `rc-service hanoip-charge-limit stop`, then with the cable connected and the battery under 80 %:
   `echo 0 > /sys/class/power_supply/battery/charging_enabled` (leave `input_suspend` at 0).
2. For a few minutes read `battery/current_now` (µA), `battery/voltage_now` and `usb/input_current_now`.
3. Bypass works if the cable keeps supplying the phone (`usb/input_current_now` clearly above 0) and
   `battery/current_now` stays near 0 with a stable voltage. If `current_now` is negative, the battery is
   feeding the phone and it is only a suspension.
4. `rc-service hanoip-charge-limit start` (stopping or starting it also restores `charging_enabled=1` and
   `input_suspend=0`).

If it works, the cell can sit at one chosen level instead of cycling, which is the gentlest state for a phone
that is always plugged in. The trade-off: the 10-cycle full-charge calibration in the limiter would be moot,
and the gauge only learns on real discharge. That redesign is not done.

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

1.1 A at the 9 V PD contract the phone negotiates is 9.9 W at the cable. **Unverified**: how the
`smb1390`/`smb1355` parallel stage reacts. Measure after installing, with the cable charging:
`usb/input_current_now` x `usb/voltage_now` should read about 10 W (`hanoip-status` shows it as "da tomada").

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
  change worth making: `echo schedutil > /sys/devices/system/cpu/cpu{0,6}/cpufreq/scaling_governor` at boot.
  Not applied yet.
- Offlining cores is not worth it. The kernel's hotplug page covers only the mechanics; whether an offlined core
  saves power depends on the hardware, and idle cores already enter cpuidle states (`state0-2` exist here)
  ([hotplug](https://www.kernel.org/doc/html/v6.8/core-api/cpu_hotplug.html),
  [Arm note](https://community.arm.com/oss-platforms/w/docs/529/cpuidle-hotplug)). It adds migration latency
  and gains little. A cap on the big cluster's top frequency is the safer lever if heat is ever a problem, and
  `hanoip-thermal` already does it past 45 °C.

DNS is I/O and latency bound, not compute bound; the real risks for it are the USB network drop and the
Tailscale DNS loop already in `NOTES.md`, not the CPU.

## Not verified

- Bypass (section 1) and the 10 W figure (section 2) on the phone.
- `hanoip-thermal` and the new charge-limit paths on the phone: only the fake-sysfs tests ran
  (`scripts/test-thermal`, `scripts/test-charge-limit`).
- The thresholds in section 3 and the SMB5 parallel-charger behaviour.
- The Moto G60 on any "bypass charging" list: no source found either way.
