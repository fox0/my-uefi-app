use core::sync::atomic::Ordering;

use crate::fox_acpi::FADT;

pub fn init_i8042() {
    // log::trace!("init_i8042");

    let fadt = FADT.load(Ordering::Relaxed);
    let fadt = unsafe { fadt.as_ref() }.expect("no init FADT");

    // Step 2: Determine if the PS/2 Controller Exists
    let flags = fadt.iapc_boot_arch;
    if !flags.motherboard_implements_8042() {
        log::warn!("i8042: No controller found");
    }
}
