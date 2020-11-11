//! RISCV64 module. Implements abstractions necessary to compile and use goOSe on
//! RISCV64

use super::*;
use crate::kmain;
use crate::println;
use cfg_if::cfg_if;

/// UART0 address
pub static UART0: usize = 0x10000000;

cfg_if! {
    if #[cfg(test)] {
        use qemu_exit;
        pub static QEMU_EXIT: qemu_exit::RISCV64 = qemu_exit::RISCV64::new(0x100000);
    }
}

/// Entry point of the kernel. Setup the stack address, call `init` and call `kmain`.
#[no_mangle]
unsafe extern "C" fn kstart() -> ! {
    #[cfg(target_arch = "riscv64")]
    asm!(
        "la sp, STACK_START
        call {}", sym init
    );

    kmain();
}

/// Initialize proper rust execution environement.
#[no_mangle]
fn init() {
    println!("\nRISCV64 Init"); // Separation from OpenSBI boot info
    clear_bss();
}

/// Clear the BSS. Should already be done by some bootloaders but just in case.
fn clear_bss() {
    let _bss_start = unsafe { (&BSS_START as *const ()) as usize };
    let _bss_end = unsafe { (&BSS_END as *const ()) as usize };

    for addr in _bss_start.._bss_end {
        let addr = addr as *mut u8;
        unsafe {
            *addr = 0;
        }
    }

    println!("BSS cleared ({:#X} -> {:#X})", _bss_start, _bss_end);
}

/// Some architecture (x86...) have a specific instruction to write on some specific
/// address (IO ports). RISCV does not, so this is just a stub for writing at
/// a specified address.
pub fn outb(addr: usize, byte: u8) {
    let addr = addr as *mut u8;
    unsafe {
        *addr = byte;
    }
}
