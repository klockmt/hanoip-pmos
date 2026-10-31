// Battery percentage for the downstream QG fuel gauge (qpnp-qg). The kernel driver only measures: it
// hands voltage and current samples to userspace through /dev/qg and expects userspace to compute the
// state of charge and write it back. Android ships a proprietary daemon for that; without one the
// percentage stays at the value the kernel computed at boot. This is a small replacement: coulomb
// counting on the samples, pulled toward the battery profile's OCV curve (looked up through
// /dev/qg_battery) while the current is low, and set to 100 % when the charger reports a full battery.
// shortcut: no ESR or capacity learning; the capacity is the profile's FCC scaled by charge_full.
// Built with rustc alone (no cargo, no crates); scripts/test-qgd runs the tests at the bottom.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::raw::{c_int, c_ulong};
use std::os::unix::io::AsRawFd;

const QG_MAX: usize = 18;
const QG_OCV_UV: usize = 1;
const QG_RBAT_MOHM: usize = 2;
const QG_GOOD_OCV_UV: usize = 4;
const QG_ESR: usize = 5;
const QG_BATT_SOC: usize = 8;
const QG_CC_SOC: usize = 9;
const QG_SYS_SOC: usize = 16;
const MAX_FIFO: usize = 16;

// struct qg_kernel_data and struct qg_user_data from include/uapi/linux/qg.h. `valid` is a C bool,
// read as u8 so any byte the kernel sends is a legal value.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Param {
    data: u32,
    valid: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Fifo {
    v: u32,
    i: u32,
    count: u32,
    interval: u32,
}

#[repr(C)]
#[derive(Default)]
struct KData {
    seq_no: u32,
    fifo_time: u32,
    fifo_length: u32,
    fifo: [Fifo; MAX_FIFO],
    param: [Param; QG_MAX],
}

#[repr(C)]
#[derive(Default)]
struct UData {
    param: [Param; QG_MAX],
}

// struct battery_params from include/uapi/linux/qg-profile.h
#[repr(C)]
#[derive(Default)]
struct BatteryParams {
    soc: i32,
    ocv_uv: i32,
    fcc_mah: i32,
    slope: i32,
    var: i32,
    batt_temp: i32,
    table_index: i32,
}

const BP_SIZE: c_ulong = std::mem::size_of::<BatteryParams>() as c_ulong;
const BPIOCXSOC: c_ulong = (3 << 30) | (BP_SIZE << 16) | ((b'B' as c_ulong) << 8) | 0x01;
const BPIOCXFCC: c_ulong = (3 << 30) | (BP_SIZE << 16) | ((b'B' as c_ulong) << 8) | 0x03;
const TABLE_SOC_OCV_CHARGE: i32 = 0;
const TABLE_SOC_OCV_DISCHARGE: i32 = 1;
const TABLE_FCC: i32 = 2;

const REST_UA: f64 = 500000.0; // below this current the voltage is close enough to the OCV to correct the count
const REST_PULL: f64 = 0.02; // fraction of the gap to the OCV curve closed per sample batch (about a minute)
const GOOD_OCV_PULL: f64 = 0.5; // the PMIC measured a real rest voltage: trust it much more
// The charger keeps reporting Full while the cable is suspended and the battery discharges, so Full
// sets 100 % only while the battery is not really discharging. At Full on the cable the kernel sends
// batches with no samples at all; those count as not discharging.
const FULL_MAX_DISCHARGE_UA: f64 = 100000.0;
// The charge limit can hold the battery at 80 % on the cable, and the charger may still say Full from the
// charge before. A really full battery reads at least 4.40 V (4.45 V under charge, 4.42 V at rest).
const FULL_MIN_UV: i64 = 4300000;
const BOOT_TRUST_S: f64 = 300.0; // the kernel's boot value is good only if we start soon after boot
const DEFAULT_RBAT_MOHM: i64 = 100;
const CUTOFF_UV: i64 = 3400000; // qcom,vbatt-cutoff-mv is not in this DT, so the driver uses its default

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

/// A sample as (uV, uA, ms) with the QG sign (positive = discharge).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sample {
    v: i64,
    i: i64,
    ms: i64,
}

/// lookup(ocv_uv, table) returns the SOC on the system scale, 0-10000.
type Lookup<'a> = &'a dyn Fn(f64, i32) -> i64;

/// The charger says Full and it can be true: not with the cable suspended (the phone runs from the
/// battery) and not at a voltage no full battery has.
fn is_full(status: Option<&str>, input_suspend: Option<&str>, voltage_uv: i64) -> bool {
    status == Some("Full") && input_suspend != Some("1") && voltage_uv >= FULL_MIN_UV
}

/// Profile SOC (0-10000, the cell's own scale) to the system scale the kernel reports, as its
/// boot code does: 0 at the cutoff voltage and 100 % at the float voltage (capped at 99 % raw).
fn normalize(raw: i64, full_raw: i64, cutoff_raw: i64) -> i64 {
    let full_raw = full_raw.min(9900);
    if full_raw <= cutoff_raw {
        return raw;
    }
    ((raw - cutoff_raw) * 10000).div_euclid(full_raw - cutoff_raw).clamp(0, 10000)
}

/// Returns the samples and the valid params.
fn parse_kdata(k: &KData) -> (Vec<Sample>, Vec<(usize, u32)>) {
    let n = (k.fifo_length as usize).min(MAX_FIFO);
    let fifo = k.fifo[..n]
        .iter()
        .map(|f| Sample { v: f.v as i64, i: f.i as i32 as i64, ms: f.count as i64 * f.interval as i64 })
        .collect();
    let params = (0..QG_MAX).filter(|&p| k.param[p].valid != 0).map(|p| (p, k.param[p].data)).collect();
    (fifo, params)
}

fn param(params: &[(usize, u32)], key: usize) -> Option<u32> {
    params.iter().find(|&&(k, _)| k == key).map(|&(_, v)| v)
}

/// Change of state of charge in percent over the samples: uA x ms / 3.6e9 = mAh.
fn coulomb_delta(fifo: &[Sample], fcc_mah: f64) -> f64 {
    let mah = fifo.iter().map(|s| s.i * s.ms).sum::<i64>() as f64 / 3.6e9;
    -100.0 * mah / fcc_mah
}

/// Time-weighted voltage and current, and the OCV they imply (V + I x R, I positive on discharge).
fn ocv_estimate(fifo: &[Sample], rbat_mohm: i64) -> Option<(f64, f64)> {
    let total = fifo.iter().map(|s| s.ms).sum::<i64>();
    if total == 0 {
        return None;
    }
    let v = fifo.iter().map(|s| s.v * s.ms).sum::<i64>() as f64 / total as f64;
    let i = fifo.iter().map(|s| s.i * s.ms).sum::<i64>() as f64 / total as f64;
    Some((v + i * rbat_mohm as f64 / 1000.0, i))
}

/// One update of the state of charge (percent). Returns it with the OCV used, if any.
fn step(soc: f64, fifo: &[Sample], params: &[(usize, u32)], full: bool, fcc_mah: f64, rbat_mohm: i64,
        lookup: Lookup) -> (f64, Option<f64>) {
    let mut soc = soc + coulomb_delta(fifo, fcc_mah);
    let est = ocv_estimate(fifo, rbat_mohm);
    let mut ocv = est.map(|(o, _)| o);
    let i = est.map(|(_, i)| i);
    if let Some(good) = param(params, QG_GOOD_OCV_UV) {
        ocv = Some(good as f64);
        soc += GOOD_OCV_PULL * (lookup(good as f64, TABLE_SOC_OCV_DISCHARGE) as f64 / 100.0 - soc);
    } else if let Some((o, i)) = est.filter(|&(_, i)| i.abs() < REST_UA) {
        let table = if i < 0.0 { TABLE_SOC_OCV_CHARGE } else { TABLE_SOC_OCV_DISCHARGE };
        soc += REST_PULL * (lookup(o, table) as f64 / 100.0 - soc);
    }
    if full && i.map_or(true, |i| i < FULL_MAX_DISCHARGE_UA) {
        soc = 100.0;
    }
    (soc.clamp(0.0, 100.0), ocv)
}

/// Where to start counting from.
///
/// saved: our last value this boot (the daemon restarted); kernel_raw: capacity_raw, 0-10000, which
/// at boot comes from the shutdown value or the power-on OCV; voltage/current: battery sysfs now
/// (current positive while charging, as the battery node reports it).
fn initial_soc(saved: Option<f64>, uptime_s: f64, kernel_raw: i64, voltage_uv: i64, current_ua: i64,
               rbat_mohm: i64, lookup: Lookup) -> f64 {
    if let Some(s) = saved {
        return s;
    }
    if uptime_s < BOOT_TRUST_S {
        return kernel_raw as f64 / 100.0;
    }
    // Started late: the boot value is stale. Estimate from the voltage, which is rough under load.
    let i = -current_ua;
    let table = if i < 0 { TABLE_SOC_OCV_CHARGE } else { TABLE_SOC_OCV_DISCHARGE };
    lookup(voltage_uv as f64 + i as f64 * rbat_mohm as f64 / 1000.0, table) as f64 / 100.0
}

fn pack_udata(soc: f64, ocv_uv: Option<f64>, rbat_mohm: i64) -> UData {
    let mut u = UData::default();
    let raw = (soc * 100.0).round_ties_even() as u32;
    for k in [QG_SYS_SOC, QG_CC_SOC, QG_BATT_SOC] {
        u.param[k] = Param { data: raw, valid: 1 };
    }
    if let Some(ocv) = ocv_uv {
        u.param[QG_OCV_UV] = Param { data: ocv as u32, valid: 1 };
    }
    u.param[QG_RBAT_MOHM] = Param { data: rbat_mohm as u32, valid: 1 };
    u
}

fn bytes_mut<T>(t: &mut T) -> &mut [u8] {
    // Only used on the repr(C) integer structs above, where every byte pattern is a valid value.
    unsafe { std::slice::from_raw_parts_mut(t as *mut T as *mut u8, std::mem::size_of::<T>()) }
}

fn read(base: &str, name: &str) -> Option<String> {
    fs::read_to_string(format!("{base}/{name}")).ok().map(|s| s.trim().to_string())
}

fn num(base: &str, name: &str, default: &str) -> i64 {
    let s = read(base, name).unwrap_or_else(|| default.to_string());
    s.parse().unwrap_or_else(|_| panic!("{base}/{name}: not a number: {s:?}"))
}

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn profile_ioctl(profile: &File, request: c_ulong, temp: i64, ocv_uv: i64, table: i32) -> BatteryParams {
    let mut bp = BatteryParams { ocv_uv: ocv_uv as i32, batt_temp: temp as i32, table_index: table, ..Default::default() };
    if unsafe { ioctl(profile.as_raw_fd(), request, &mut bp as *mut BatteryParams) } < 0 {
        panic!("/dev/qg_battery ioctl: {}", std::io::Error::last_os_error());
    }
    bp
}

fn main() {
    let bat = env("HANOIP_BATTERY_DIR", "/sys/class/power_supply/battery");
    let bms = env("HANOIP_BMS_DIR", "/sys/class/power_supply/bms");
    let state = env("HANOIP_QGD_STATE", "/run/hanoip-qgd.soc");
    let mut qg = OpenOptions::new().read(true).write(true).open("/dev/qg").expect("/dev/qg");
    let profile = OpenOptions::new().read(true).write(true).open("/dev/qg_battery").expect("/dev/qg_battery");

    let profile_soc = |ocv_uv: i64, table: i32, temp: i64| profile_ioctl(&profile, BPIOCXSOC, temp, ocv_uv, table).soc as i64;
    let lookup = |ocv_uv: f64, table: i32| {
        let temp = num(&bat, "temp", "250");
        let full = profile_soc(num(&bms, "voltage_max", "4450000"), TABLE_SOC_OCV_CHARGE, temp);
        let cutoff = profile_soc(CUTOFF_UV, TABLE_SOC_OCV_DISCHARGE, temp);
        normalize(profile_soc(ocv_uv as i64, table, temp), full, cutoff)
    };
    let fcc_mah = || {
        let fcc = profile_ioctl(&profile, BPIOCXFCC, num(&bat, "temp", "250"), 0, TABLE_FCC).fcc_mah as f64;
        let (learned, design) = (num(&bat, "charge_full", "0"), num(&bat, "charge_full_design", "0"));
        let scale = if learned > 0 && design > 0 { learned as f64 / design as f64 } else { 1.0 };
        fcc * scale
    };

    let mut rbat = DEFAULT_RBAT_MOHM;
    let saved = fs::read_to_string(&state).ok().and_then(|s| s.trim().parse().ok());
    let uptime: f64 = fs::read_to_string("/proc/uptime").expect("/proc/uptime")
        .split_whitespace().next().and_then(|s| s.parse().ok()).expect("/proc/uptime");
    let mut soc = initial_soc(saved, uptime, num(&bms, "capacity_raw", "0"), num(&bat, "voltage_now", "0"),
                              num(&bat, "current_now", "0"), rbat, &lookup);
    println!("hanoip-qgd: start at {soc:.2} %");
    loop {
        let mut k = KData::default();
        // blocks until the next batch, about a minute
        let n = qg.read(bytes_mut(&mut k)).expect("/dev/qg read");
        assert_eq!(n, std::mem::size_of::<KData>(), "/dev/qg: short read");
        let (fifo, params) = parse_kdata(&k);
        if let Some(esr) = param(&params, QG_ESR).filter(|e| (10..=1000).contains(e)) {
            rbat = esr as i64;
        }
        // A suspended cable means the phone runs from the battery, whatever the charger's status says.
        let full = is_full(read(&bat, "status").as_deref(), read(&bat, "input_suspend").as_deref(),
                           num(&bat, "voltage_now", "0"));
        let ocv;
        (soc, ocv) = step(soc, &fifo, &params, full, fcc_mah(), rbat, &lookup);
        qg.write_all(bytes_mut(&mut pack_udata(soc, ocv, rbat))).expect("/dev/qg write");
        fs::write(&state, format!("{soc:.4}\n")).expect("state file");
    }
}

// The same checks the Python version had: the arithmetic, the struct layout and the regressions.
#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// A profile that answers the same SOC (0-10000) for any voltage.
    fn flat(soc_pct: i64) -> impl Fn(f64, i32) -> i64 {
        move |_, _| soc_pct * 100
    }

    fn s(v: i64, i: i64, ms: i64) -> Sample {
        Sample { v, i, ms }
    }

    // Layout of the kernel structs (include/uapi/linux/qg.h): 412 and 144 bytes on arm64.
    #[test]
    fn layout() {
        assert_eq!(std::mem::size_of::<KData>(), 412);
        assert_eq!(std::mem::size_of::<UData>(), 144);
        assert_eq!(std::mem::size_of::<BatteryParams>(), 28);
        assert_eq!(BPIOCXSOC, 0xC01C4201);
    }

    // A real batch from the phone while charging at about 3 A: 5 blocks of 128 samples x 98 ms.
    #[test]
    fn parse() {
        let mut k = KData { seq_no: 28, fifo_time: 136118500, fifo_length: 2, ..Default::default() };
        k.fifo[0] = Fifo { v: 4195789, i: (-2865907i32) as u32, count: 128, interval: 98 };
        k.fifo[1] = Fifo { v: 4195595, i: (-2861025i32) as u32, count: 128, interval: 98 };
        k.param[QG_ESR] = Param { data: 106, valid: 1 };
        let (fifo, params) = parse_kdata(&k);
        assert_eq!(fifo.len(), 2, "two samples");
        assert_eq!(fifo[0], s(4195789, -2865907, 12544), "negative current while charging");
        assert_eq!(params, vec![(QG_ESR, 106)], "only valid params");
    }

    // Coulomb count: 2.865907 A for 12.544 s is 9.986 mAh, 0.1617 % of 6174 mAh, and charging raises the SOC.
    #[test]
    fn coulomb() {
        assert!(near(coulomb_delta(&[s(4200000, -2865907, 12544)], 6174.0), 0.16174, 1e-4), "charge raises SOC");
        assert!(coulomb_delta(&[s(3800000, 200000, 60000)], 6174.0) < 0.0, "discharge lowers SOC");
        assert!(near(coulomb_delta(&[s(3800000, 617400, 3600000)], 6174.0), -10.0, 1e-9), "one hour at 617.4 mA is 10 %");
    }

    // OCV estimate: V + I x R, with I in uA and R in mOhm.
    #[test]
    fn ocv() {
        let (ocv, i) = ocv_estimate(&[s(3800000, 200000, 1000)], 100).unwrap();
        assert!(near(ocv, 3820000.0, 1e-6) && i == 200000.0, "ocv under discharge");
        assert_eq!(ocv_estimate(&[], 100), None, "no samples, no ocv");
    }

    #[test]
    fn corrections() {
        let heavy = [s(4200000, -3000000, 1)]; // charging hard: the voltage says nothing, only the count moves
        let (soc, _) = step(40.0, &heavy, &[], false, 6174.0, 100, &flat(90));
        assert!(soc > 40.0 && soc < 40.01, "no pull under heavy current");
        let rest = [s(3800000, 150000, 1)];
        let (soc, _) = step(40.0, &rest, &[], false, 6174.0, 100, &flat(50));
        assert!(near(soc, 40.0 + 0.02 * 10.0, 1e-3), "slow pull toward the OCV curve at rest");
        let (soc, _) = step(40.0, &rest, &[(QG_GOOD_OCV_UV, 3800000)], false, 6174.0, 100, &flat(60));
        assert!(near(soc, 50.0, 1e-3), "strong pull on a measured rest OCV");
        let (soc, _) = step(73.0, &heavy, &[], true, 6174.0, 100, &flat(10));
        assert_eq!(soc, 100.0, "charger says full: 100 %");
        let (soc, _) = step(99.0, &[s(4400000, 0, 60000)], &[], true, 6174.0, 100, &flat(100));
        assert_eq!(soc, 100.0, "full at rest on the cable: 100 %");
        // Regression (2026-10-29): with the cable suspended the charger still says Full while the battery
        // discharges; resetting to 100 % there froze the percentage and the charge limit never resumed.
        let (soc, _) = step(100.0, &[s(4330000, 300000, 300000)], &[], true, 6174.0, 100, &flat(100));
        assert!(soc < 100.0, "full but discharging: the count goes down");
        let (soc, _) = step(98.4, &[], &[], true, 6174.0, 100, &flat(100));
        assert_eq!(soc, 100.0, "full with no samples (the kernel stops sampling at Full on the cable): 100 %");
        let (soc, _) = step(50.0, &[], &[], false, 6174.0, 100, &flat(100));
        assert_eq!(soc, 50.0, "no samples and not full changes nothing");
        let (soc, _) = step(0.01, &[s(3300000, 3000000, 3600000)], &[], false, 6174.0, 100, &flat(0));
        assert_eq!(soc, 0.0, "never below 0 %");
    }

    // Holding the level on the cable (charging_enabled=0, input on) can leave the charger saying Full after a
    // finished charge; at 4.2 V that is not a full battery, so it must not set 100 %.
    #[test]
    fn full() {
        assert!(is_full(Some("Full"), Some("0"), 4450000), "full at 4.45 V is full");
        assert!(!is_full(Some("Full"), Some("0"), 4200000), "full at 4.2 V is not");
        assert!(!is_full(Some("Full"), Some("1"), 4450000), "full with the cable suspended is not");
        assert!(!is_full(Some("Charging"), Some("0"), 4450000), "charging is not full");
        assert!(is_full(Some("Full"), None, 4450000), "no input_suspend file is not a suspended cable");
    }

    // Profile scale to system scale, like the kernel's boot code: the cell reads about 97 % at 4.4 V on
    // this profile, which the system calls 100 %.
    #[test]
    fn normalize_scale() {
        assert_eq!(normalize(9900, 9900, 150), 10000, "float voltage is 100 %");
        assert_eq!(normalize(150, 9900, 150), 0, "cutoff is 0 %");
        assert_eq!(normalize(50, 9900, 150), 0, "below cutoff stays 0 %");
        assert_eq!(normalize(9900, 9950, 150), 10000, "full capped at 99 % raw");
        assert_eq!(normalize(5025, 9900, 150), 5000, "midpoint");
        assert_eq!(normalize(4000, 100, 200), 4000, "broken profile passes through");
    }

    #[test]
    fn start() {
        let lk = flat(55);
        assert_eq!(initial_soc(Some(37.5), 9999.0, 100, 4200000, 3000000, 100, &lk), 37.5, "restart resumes the saved value");
        assert_eq!(initial_soc(None, 60.0, 4321, 4200000, 0, 100, &lk), 43.21, "early start trusts the kernel's boot value");
        assert_eq!(initial_soc(None, 3600.0, 100, 4200000, 3000000, 100, &lk), 55.0, "late start estimates from the voltage");
    }

    // What goes back to the kernel: SYS_SOC, CC_SOC and BATT_SOC in 0-10000, OCV and RBAT.
    #[test]
    fn udata() {
        let u = pack_udata(42.37, Some(3812345.6), 106);
        let val: Vec<(usize, u32)> = (0..QG_MAX).filter(|&k| u.param[k].valid != 0).map(|k| (k, u.param[k].data)).collect();
        assert_eq!(val, vec![(QG_OCV_UV, 3812345), (QG_RBAT_MOHM, 106), (QG_BATT_SOC, 4237), (QG_CC_SOC, 4237), (QG_SYS_SOC, 4237)]);
        // Python's round() rounds half to even; keep the same value the old daemon wrote.
        assert_eq!(pack_udata(0.125, None, 100).param[QG_SYS_SOC].data, 12);
    }
}
