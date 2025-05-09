use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

use acpi::rsdp::Rsdp;
use uefi::system::with_config_table;
use uefi::table::cfg::ConfigTableEntry;

/// Init [`init_rsdp`]
pub static ACPI: AtomicPtr<Rsdp> = AtomicPtr::new(null_mut());

pub fn init_acpi() {
    // log::trace!("init_acpi");

    let mut acpi_address = None;

    with_config_table(|slice: &[ConfigTableEntry]| {
        for i in slice {
            match i.guid {
                ConfigTableEntry::ACPI_GUID => {
                    log::debug!("0x{:08x} - found ACPI1", i.address as usize);
                    if acpi_address.is_none() {
                        acpi_address = Some(i.address as usize);
                    }
                }
                ConfigTableEntry::ACPI2_GUID => {
                    log::debug!("0x{:08x} - found ACPI2", i.address as usize);
                    acpi_address = Some(i.address as usize);
                    break;
                }
                _ => {
                    // log::debug!("0x{:08x} - found GUID", i.address as usize);
                }
            }
        }
    });

    let acpi_address = acpi_address.expect("ACPI not found");

    log::debug!("0x{:08x} - found RSDP", acpi_address);

    let rsdp = acpi_address as *mut Rsdp;
    let rsdp = unsafe { rsdp.as_ref() }.unwrap();
    rsdp.validate().expect("invalid RSDP");
    // println!("RSDP = {:?}", rsdp);

    ACPI.store(acpi_address as _, Ordering::Release);
}
