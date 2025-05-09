#![no_main]
#![no_std]

use core::time::Duration;

use acpi::rsdp::Rsdp;
use acpi::sdt::{SdtHeader, Signature};
use uefi::helpers::init;
use uefi::system::{firmware_revision, firmware_vendor, uefi_revision, with_config_table};
use uefi::table::cfg::ConfigTableEntry;
use uefi::{Status, entry, println};

fn stall(duration: Duration) {
    use uefi::boot::stall;
    stall(duration.as_micros() as usize)
}

#[must_use]
fn get_acpi_address() -> usize {
    let mut acpi_address = None;

    with_config_table(|slice: &[ConfigTableEntry]| {
        for i in slice {
            match i.guid {
                ConfigTableEntry::ACPI_GUID => {
                    if acpi_address == None {
                        acpi_address = Some(i.address);
                    }
                }
                ConfigTableEntry::ACPI2_GUID => {
                    acpi_address = Some(i.address);
                }
                _ => {}
            }
        }
    });

    let acpi_address = acpi_address.expect("ACPI not found");
    let acpi_address = acpi_address as usize;
    println!("acpi_address = 0x{:08x}", acpi_address);
    acpi_address
}

fn find_acpi() {
    let acpi_address = get_acpi_address();

    let rsdp = unsafe { *(acpi_address as *mut Rsdp) };
    rsdp.validate().expect("invalid RSDP");
    println!("RSDP = {:?}", rsdp);

    // let rsdt_address = rsdp.rsdt_address() as usize;
    // let xsdt_address = if rsdp.revision() == 0 {
    // None
    // } else {
    // Some(rsdp.xsdt_address() as usize)
    // };
    assert!(rsdp.revision() > 0);
    // If the pointer to the XSDT is valid, the OS MUST use the XSDT.
    let xsdt_address = rsdp.xsdt_address() as usize;

    let ttt = unsafe { *(xsdt_address as *mut SdtHeader) };
    println!("ttt = {:?}", ttt);
    println!("ttt = {:?}", ttt.validate(Signature::XSDT)); // TODO SdtInvalidChecksum
}

#[entry]
fn main() -> Status {
    init().unwrap();

    println!("Firmware vendor: {}", firmware_vendor());
    println!("Firmware revision: {:#04x}", firmware_revision());
    println!("UEFI revision: {}", uefi_revision());

    find_acpi();

    // info!("Hello world!");

    stall(Duration::from_secs(600));
    Status::SUCCESS
}
