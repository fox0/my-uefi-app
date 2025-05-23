// #![feature(step_trait)]
#![no_main]
#![no_std]

use core::time::Duration;

use uefi::boot::stall;
use uefi::helpers::init;
use uefi::{Status, entry, println};

use crate::drivers::{Driver, I8042};
use crate::fox_acpi::init_fadt;
use crate::fox_uefi::init_acpi;

mod drivers;
mod fox_acpi;
mod fox_uefi;

#[entry]
fn main() -> Status {
    init().unwrap();
    println!();
    init_acpi();
    init_fadt();

    if I8042::probe().is_ok() {
        let mut i8042 = I8042::default();
        i8042.init();
        log::debug!("{:?}", i8042);
        i8042.remove();
    };

    stall(Duration::from_secs(600));
    Status::SUCCESS
}
