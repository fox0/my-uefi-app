#![no_main]
#![no_std]

use core::mem::size_of;
use core::time::Duration;

use acpi::fadt::Fadt;
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
                    log::debug!("0x{:08x} - found ACPI1", i.address as usize);
                    if acpi_address == None {
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

    acpi_address.expect("ACPI not found")
}

fn find_acpi() {
    let acpi_address = get_acpi_address();
    log::debug!("0x{:08x} - found RSDP", acpi_address);

    let rsdp = acpi_address as *mut Rsdp;
    let rsdp = unsafe { rsdp.as_ref() }.unwrap();
    rsdp.validate().expect("invalid RSDP");
    // println!("RSDP = {:?}", rsdp);

    // let rsdt_address = rsdp.rsdt_address() as usize;
    // let xsdt_address = if rsdp.revision() == 0 {
    // None
    // } else {
    // Some(rsdp.xsdt_address() as usize)
    // };
    assert!(rsdp.revision() > 0);
    // If the pointer to the XSDT is valid, the OS MUST use the XSDT.
    let xsdt_address = rsdp.xsdt_address() as usize;
    log::debug!("0x{:08x} - found XSDT", xsdt_address);

    // System Descriptor tables
    // struct XSDT {
    //     struct ACPISDTHeader h;
    //     uint64_t PointerToOtherSDT[(h.Length - sizeof(h)) / 8];
    // };
    let xsdt = xsdt_address as *mut SdtHeader;
    let xsdt = unsafe { xsdt.as_ref() }.unwrap();
    xsdt.validate(Signature::XSDT).expect("invalid XSDT");
    // println!("XSDT = {:?}", xsdt);

    let length = xsdt.length as usize;
    const LENGTH_SDT_HEADER: usize = size_of::<SdtHeader>();
    const LENGTH_U64: usize = size_of::<u64>();
    // let entries = (length - LENGTH_SDT_HEADER) / LENGTH_U64;
    // log::debug!("entries = {}", entries);

    let mut fadt_address = None;

    for others_address in
        (xsdt_address + LENGTH_SDT_HEADER..xsdt_address + length).step_by(LENGTH_U64)
    {
        let sdt_address = others_address as *mut u64;
        let sdt_address = unsafe { sdt_address.as_ref() }.unwrap();
        let sdt_address = sdt_address.clone() as usize;

        let sdt = sdt_address as *mut SdtHeader;
        let sdt = unsafe { sdt.as_ref() }.unwrap();
        match sdt.signature {
            Signature::FADT => {
                log::debug!("0x{:08x} - found FADT", sdt_address);
                fadt_address = Some(sdt_address);
                break;
            }
            _ => {
                // log::debug!("0x{:08x} - found SDT", sdt_address);
            }
        }
        sdt.validate(sdt.signature).expect("invalid DST");
    }

    let fadt_address = fadt_address.expect("FADT not found");

    let fadt = fadt_address as *mut Fadt;
    let fadt = unsafe { fadt.as_ref() }.unwrap();
    fadt.validate().expect("invalid FADT");

    // Step 2: Determine if the PS/2 Controller Exists
    let flags = fadt.iapc_boot_arch;
    println!(
        "motherboard_implements_8042 = {:?}",
        flags.motherboard_implements_8042()
    );
}

#[entry]
fn main() -> Status {
    init().unwrap();

    println!();
    println!("Firmware vendor: {}", firmware_vendor());
    println!("Firmware revision: {:#04x}", firmware_revision());
    println!("UEFI revision: {}", uefi_revision());

    find_acpi();

    // info!("Hello world!");

    stall(Duration::from_secs(600));
    Status::SUCCESS
}
