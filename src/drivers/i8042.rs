#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

//! I8042 PS/2 Controller
//!
//! * [`CommandRegister`]
//! * [`StatusRegister`]
//! * [`DataPort`]

// https://wiki.osdev.org/I8042_PS/2_Controller

use core::fmt;

use bit_field::BitField;

use super::Driver;
use crate::fox_acpi::fadt_raw;
use crate::fox_port::{
    Port, PortGeneric, PortReadOnly, PortWriteOnly, ReadOnlyAccess, ReadWriteAccess,
    WriteOnlyAccess,
};

/// I8042 PS/2 Controller
pub struct I8042;

impl Driver for I8042 {
    fn probe() -> Result<(), ()> {
        // log::trace!("I8042::probe()");

        let fadt = fadt_raw().expect("no init FADT");
        let fadt = unsafe { fadt.as_ref() };

        // Step 1: Initialize USB Controllers

        // Step 2: Determine if the PS/2 Controller Exists
        let flags = fadt.iapc_boot_arch;
        if !flags.motherboard_implements_8042() {
            log::warn!("i8042: No controller found");
            Err(())
        } else {
            log::info!("i8042: Found controller");
            Ok(())
        }
    }

    fn init() {
        // Step 3: Disable Devices
        CommandRegister::disable_first_port();
        CommandRegister::disable_second_port();

        // Step 4: Flush The Output Buffer
        while DataPort::read().is_some() {}

        // Step 5: Set the Controller Configuration Byte
        let t = CommandRegister::get_controller_configuration_byte();
        log::debug!("{:?}", t);

        // todo!()
    }

    fn remove() {
        // todo!()
    }
}

// // An example interrupt based on https://os.phil-opp.com/hardware-interrupts/. The ps2 mouse is configured to fire
// // interrupts at PIC offset 12.
// extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
//     let mut port = PortReadOnly::new(0x60);
//     let packet = unsafe { port.read() };
//     MOUSE.lock().process_packet(packet);

//     unsafe {
//         PICS.lock()
//             .notify_end_of_interrupt(InterruptIndex::Mouse.into());
//     }
// }

struct CommandRegister;

impl CommandRegister {
    const PORT: PortGeneric<u8, WriteOnlyAccess> = PortWriteOnly::<u8>::new(0x0064);

    pub fn disable_first_port() {
        Self::run_cmd(Commands::DisableFirstPort);
        // Response Byte: None
    }

    pub fn disable_second_port() {
        Self::run_cmd(Commands::DisableSecondPort);
        // Response Byte: None
    }

    pub fn get_controller_configuration_byte() -> ControllerConfigurationByte {
        Self::run_cmd(Commands::ReadByte0);
        let resp = DataPort::read();
        ControllerConfigurationByte(resp.unwrap())
    }

    fn run_cmd(cmd: Commands) {
        unsafe {
            Self::PORT.write(cmd.into());
        }
    }
}

#[allow(dead_code)]
#[repr(u8)]
enum Commands {
    /// Read "byte 0" from internal RAM
    ReadByte0 = 0x20,
    // ...
    /// Disable second PS/2 port (only if 2 PS/2 ports supported)
    DisableSecondPort = 0xA7,
    /// Enable second PS/2 port (only if 2 PS/2 ports supported)
    EnableSecondPort = 0xA8,
    // ...
    /// Disable first PS/2 port
    DisableFirstPort = 0xAD,
    /// Enable first PS/2 port
    EnableFirstPort = 0xAE,
    // ...
}

impl From<Commands> for u8 {
    fn from(value: Commands) -> Self {
        value as _
    }
}

/// The Status Register contains various flags that show the state of the PS/2 controller
struct StatusRegister(u8);

impl StatusRegister {
    const PORT: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::<u8>::new(0x0064);

    pub fn read() -> Self {
        // log::trace!("read from port 0x0064");
        Self(unsafe { Self::PORT.read() })
    }

    /// Output buffer status (0 = empty, 1 = full)
    /// (must be set before attempting to read data from IO port 0x60)
    pub fn output_buffer_is_full(&self) -> bool {
        self.0.get_bit(0)
    }

    /// Input buffer status (0 = empty, 1 = full)
    /// (must be clear before attempting to write data to IO port 0x60 or IO port 0x64)
    pub fn input_buffer_is_full(&self) -> bool {
        self.0.get_bit(1)
    }

    /// System Flag
    /// Meant to be cleared on reset and set by firmware (via. PS/2 Controller Configuration Byte) if the system passes self tests (POST)
    pub fn system_flag(&self) -> bool {
        self.0.get_bit(2)
    }

    // TODO
}

impl fmt::Debug for StatusRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatusRegister")
            .field("output_buffer_is_full", &self.output_buffer_is_full())
            .field("input_buffer_is_full", &self.input_buffer_is_full())
            .field("system_flag", &self.system_flag())
            .finish()
    }
}

struct DataPort;

impl DataPort {
    const PORT: PortGeneric<u8, ReadWriteAccess> = Port::<u8>::new(0x0060);

    fn read() -> Option<u8> {
        let status = StatusRegister::read();
        // must be set before attempting to read data from IO port 0x60
        if status.output_buffer_is_full() {
            Some(unsafe { Self::PORT.read() })
        } else {
            None
        }
    }
}

struct ControllerConfigurationByte(u8);

impl ControllerConfigurationByte {
    /// First PS/2 port interrupt (1 = enabled, 0 = disabled)
    pub fn first_port_interrupt_is_enable(&self) -> bool {
        self.0.get_bit(0)
    }

    /// Second PS/2 port interrupt (1 = enabled, 0 = disabled, only if 2 PS/2 ports supported)
    pub fn second_port_interrupt_is_enable(&self) -> bool {
        self.0.get_bit(1)
    }

    /// System Flag (1 = system passed POST, 0 = your OS shouldn't be running)
    pub fn system_flag(&self) -> bool {
        self.0.get_bit(2)
    }

    // TODO
}

impl fmt::Debug for ControllerConfigurationByte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControllerConfigurationByte")
            .field(
                "first_port_interrupt_is_enable",
                &self.first_port_interrupt_is_enable(),
            )
            .field(
                "second_port_interrupt_is_enable",
                &self.second_port_interrupt_is_enable(),
            )
            .field("system_flag", &self.system_flag())
            .finish()
    }
}
