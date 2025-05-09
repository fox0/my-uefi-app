use core::mem::size_of;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

use acpi::fadt::Fadt;
use acpi::sdt::{SdtHeader, Signature};

use crate::fox_uefi::ACPI;

/// Fixed ACPI Description Table (FADT).
///
/// Init [`init_fadt`]
pub static FADT: AtomicPtr<Fadt> = AtomicPtr::new(null_mut());

pub fn init_fadt() {
    // log::trace!("init_fadt");

    let rsdp = ACPI.load(Ordering::Relaxed);
    let rsdp = unsafe { rsdp.as_ref() }.expect("no init ACPI");

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
        let sdt_address = *sdt_address as usize;

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

    FADT.store(fadt_address as _, Ordering::Release);
}
