//! in-place replace [`x86_64::instructions::port`]
//!
//! # Example
//!
//! ```rust,not_run
//! use crate::fox_port::{
//!     Port, PortGeneric, PortReadOnly, PortWriteOnly, ReadOnlyAccess, ReadWriteAccess,
//!     WriteOnlyAccess,
//! };
//! ```

#![allow(dead_code)]

use core::marker::PhantomData;

pub use x86_64::instructions::port::{
    PortReadAccess, PortWriteAccess, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess,
};

use x86_64::structures::port::{PortRead, PortWrite};

/// An I/O port.
///
/// The port reads or writes values of type `T` and has read/write access specified by `A`.
///
/// Use the provided marker types or aliases to get a port type with the access you need:
/// * `PortGeneric<T, ReadWriteAccess>` -> `Port<T>`
/// * `PortGeneric<T, ReadOnlyAccess>` -> `PortReadOnly<T>`
/// * `PortGeneric<T, WriteOnlyAccess>` -> `PortWriteOnly<T>`
pub struct PortGeneric<T, A> {
    port: u16,
    phantom: PhantomData<(T, A)>,
}

/// A read-write I/O port.
pub type Port<T> = PortGeneric<T, ReadWriteAccess>;

/// A read-only I/O port.
pub type PortReadOnly<T> = PortGeneric<T, ReadOnlyAccess>;

/// A write-only I/O port.
pub type PortWriteOnly<T> = PortGeneric<T, WriteOnlyAccess>;

impl<T, A> PortGeneric<T, A> {
    /// Creates an I/O port with the given port number.
    #[inline]
    pub const fn new(port: u16) -> PortGeneric<T, A> {
        PortGeneric {
            port,
            phantom: PhantomData,
        }
    }
}

impl<T: PortRead, A: PortReadAccess> PortGeneric<T, A> {
    /// Reads from the port.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because the I/O port could have side effects that violate memory
    /// safety.
    #[inline]
    #[must_use]
    pub unsafe fn read(&self) -> T {
        unsafe { T::read_from_port(self.port) }
    }
}

impl<T: PortWrite, A: PortWriteAccess> PortGeneric<T, A> {
    /// Writes to the port.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because the I/O port could have side effects that violate memory
    /// safety.
    #[inline]
    pub unsafe fn write(&self, value: T) {
        unsafe { T::write_to_port(self.port, value) }
    }
}

// #[cfg(test)]
mod tests {
    use super::*;

    #[deny(const_item_mutation)]
    #[allow(dead_code)]
    #[allow(clippy::assertions_on_constants)]
    fn deny_const_item_mutation_compile_test() {
        assert!(false, "compile test only");

        const DATA_PORT: PortGeneric<u8, ReadWriteAccess> = Port::<u8>::new(0x0060);
        const COMMAND_REGISTER: PortGeneric<u8, WriteOnlyAccess> = PortWriteOnly::<u8>::new(0x0064);
        const STATUS_REGISTER: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::<u8>::new(0x0064);

        unsafe {
            let _ = DATA_PORT.read();
            DATA_PORT.write(0x42);
            COMMAND_REGISTER.write(0x42);
            let _ = STATUS_REGISTER.read();
        }
    }
}
