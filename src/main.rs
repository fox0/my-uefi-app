#![no_main]
#![no_std]

use core::time::Duration;

use uefi::helpers::init;
use uefi::{Status, entry, println};

use crate::fox_acpi::init_fadt;
use crate::fox_i8042::init_i8042;
use crate::fox_uefi::init_acpi;

mod fox_acpi;
mod fox_i8042;
mod fox_uefi;

fn stall(duration: Duration) {
    use uefi::boot::stall;
    stall(duration.as_micros() as usize)
}

#[entry]
fn main() -> Status {
    init().unwrap();
    println!();
    init_acpi();
    init_fadt();
    init_i8042();

    stall(Duration::from_secs(600));
    Status::SUCCESS
}
