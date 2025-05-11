use core::ptr::{NonNull, null_mut};
use core::sync::atomic::{AtomicPtr, Ordering};

use acpi::rsdp::Rsdp;
use uefi::system::with_config_table;
use uefi::table::cfg::ConfigTableEntry;
use x86_64::VirtAddr;

/// Init [`init_rsdp`]
static ACPI: AtomicPtr<Rsdp> = AtomicPtr::new(null_mut());

pub fn rsdp_raw() -> Option<NonNull<Rsdp>> {
    let ptr = ACPI.load(Ordering::Acquire);
    NonNull::new(ptr)
}

pub fn init_acpi() {
    // log::trace!("init_acpi");
    let mut acpi_address = None;

    with_config_table(|slice: &[ConfigTableEntry]| {
        for i in slice {
            match i.guid {
                ConfigTableEntry::ACPI_GUID => {
                    log::debug!("Found ACPI1");
                    if acpi_address.is_none() {
                        acpi_address = Some(VirtAddr::from_ptr(i.address));
                    }
                }
                ConfigTableEntry::ACPI2_GUID => {
                    log::debug!("Found ACPI2");
                    acpi_address = Some(VirtAddr::from_ptr(i.address));
                    break;
                }
                _ => {
                    // log::debug!("0x{:08x} - found GUID", i.address as u64);
                }
            }
        }
    });

    let acpi_address = acpi_address.expect("ACPI not found");

    log::debug!("Found RSDP");

    let rsdp = acpi_address.as_u64() as *mut Rsdp;
    let rsdp = unsafe { rsdp.as_ref() }.unwrap();
    rsdp.validate().expect("invalid RSDP");
    // println!("RSDP = {:?}", rsdp);

    ACPI.store(acpi_address.as_u64() as _, Ordering::Release);
}
