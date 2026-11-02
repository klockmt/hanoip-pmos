// Restarts the phone straight into the bootloader (fastboot), without the buttons. busybox's reboot takes no
// argument, so this calls reboot(2) with LINUX_REBOOT_CMD_RESTART2 and the string "bootloader" itself. Needs root.
// scripts/reboot-bootloader runs it. Built with rustc alone, like hanoip-qgd.
use std::os::raw::{c_char, c_long};

// The syscall number below is arm64's; on another architecture 142 is a different call, so refuse to build there.
#[cfg(not(target_arch = "aarch64"))]
compile_error!("hanoip-reboot-bootloader is for the phone (aarch64) only");

extern "C" {
    fn syscall(num: c_long, ...) -> c_long;
    fn sync();
}

const SYS_REBOOT_ARM64: c_long = 142;
const MAGIC1: c_long = 0xfee1dead;
const MAGIC2: c_long = 672274793;
const CMD_RESTART2: c_long = 0xA1B2C3D4;

fn main() {
    let arg = b"bootloader\0";
    // SAFETY: plain system calls; the string is NUL-terminated and outlives the call.
    unsafe {
        sync();
        syscall(SYS_REBOOT_ARM64, MAGIC1, MAGIC2, CMD_RESTART2, arg.as_ptr() as *const c_char);
    }
    // only reached when the call failed (not root, say)
    eprintln!("hanoip-reboot-bootloader: reboot failed: {}", std::io::Error::last_os_error());
    std::process::exit(1);
}
