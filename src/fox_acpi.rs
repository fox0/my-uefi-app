// use core::iter::Step;
use core::mem::size_of;
use core::ptr::{NonNull, null_mut};
use core::sync::atomic::{AtomicPtr, Ordering};

use acpi::fadt::Fadt;
use acpi::sdt::{SdtHeader, Signature};
use x86_64::VirtAddr;

use crate::fox_uefi::rsdp_raw;

/// Fixed ACPI Description Table (FADT).
///
/// Init [`init_fadt`]
static FADT: AtomicPtr<Fadt> = AtomicPtr::new(null_mut());

pub fn fadt_raw() -> Option<NonNull<Fadt>> {
    let ptr = FADT.load(Ordering::Acquire);
    NonNull::new(ptr)
}

pub fn init_fadt() {
    // log::trace!("init_fadt");

    let rsdp = rsdp_raw().expect("no init ACPI");
    let rsdp = unsafe { rsdp.as_ref() };

    // let rsdt_address = rsdp.rsdt_address() as u64;
    // let xsdt_address = if rsdp.revision() == 0 {
    // None
    // } else {
    // Some(rsdp.xsdt_address() as u64)
    // };
    assert!(rsdp.revision() > 0);

    // If the pointer to the XSDT is valid, the OS MUST use the XSDT.
    let xsdt_address = VirtAddr::new(rsdp.xsdt_address());
    log::debug!("Found XSDT");

    // System Descriptor tables
    // struct XSDT {
    //     struct ACPISDTHeader h;
    //     uint64_t PointerToOtherSDT[(h.Length - sizeof(h)) / 8];
    // };
    let xsdt = xsdt_address.as_u64() as *mut SdtHeader;
    let xsdt = unsafe { xsdt.as_ref() }.unwrap();
    xsdt.validate(Signature::XSDT).expect("invalid XSDT");
    // println!("XSDT = {:?}", xsdt);

    let length = xsdt.length as u64;
    const LENGTH_SDT_HEADER: u64 = size_of::<SdtHeader>() as u64;
    const LENGTH_U64: usize = size_of::<u64>();
    // let entries = (length - LENGTH_SDT_HEADER) / LENGTH_U64;
    // log::debug!("entries = {}", entries);

    let mut fadt_address = None;

    for others_address in
        (xsdt_address + LENGTH_SDT_HEADER..xsdt_address + length).step_by(LENGTH_U64)
    {
        let sdt_address = others_address.as_u64() as *mut u64;
        let sdt_address = unsafe { sdt_address.as_ref() }.unwrap();
        let sdt_address = *sdt_address;

        let sdt = sdt_address as *mut SdtHeader;
        let sdt = unsafe { sdt.as_ref() }.unwrap();
        match sdt.signature {
            Signature::FADT => {
                log::debug!("Found FADT");
                fadt_address = Some(VirtAddr::new(sdt_address));
                break;
            }
            _ => {
                // log::debug!("0x{:08x} - found SDT", sdt_address);
                // sdt.validate(sdt.signature).expect("invalid DST");
            }
        }
    }

    let fadt_address = fadt_address.expect("FADT not found");

    let fadt = fadt_address.as_u64() as *mut Fadt;
    let fadt = unsafe { fadt.as_ref() }.unwrap();
    fadt.validate().expect("invalid FADT");

    FADT.store(fadt_address.as_u64() as _, Ordering::Release);
}

// #[must_use]
// pub fn is_enable() -> bool {
//     let fadt = FADT.load(Ordering::Relaxed);
//     let fadt = unsafe { fadt.as_ref() }.expect("no init FADT");

//     // On some PCs, this is already done for you if...
//     // the SMI command field in the FADT is 0
//     // the ACPI enable and ACPI disable fields in the FADT are both 0
//     // bit 0 (value 1) of the PM1a control block I/O port is set
//     let t1 = fadt.smi_cmd_port;
//     log::debug!("{} {} {}", t1, fadt.acpi_enable, fadt.acpi_disable);
//     let rrr = fadt.pm1a_control_block().unwrap();
//     let _rrr = rrr.address;
//     todo!()
// }

// /// Switching to ACPI Mode
// pub fn enable() {
//     let fadt = FADT.load(Ordering::Relaxed);
//     let fadt = unsafe { fadt.as_ref() }.expect("no init FADT");

//     let rrr = fadt.pm1a_control_block().unwrap();
//     let _rrr = rrr.address;
//     todo!()
//     // outb(fadt->smi_command,fadt->acpi_enable);
//     // while (inw(fadt->pm1a_control_block) & 1 == 0);
// }
