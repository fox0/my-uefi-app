#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

//! I8042 PS/2 Controller
//!
//! https://wiki.osdev.org/I8042_PS/2_Controller
//!
//! TODO:
//! - [`port::PortDataPort::read`] - spinlock

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
        log::trace!("I8042::init()");
        // Step 3: Disable Devices
        // log::trace!("step 3");
        port::PortCommandRegister::disable_first_port();
        port::PortCommandRegister::disable_second_port();

        // Step 4: Flush The Output Buffer
        // log::trace!("step 4");
        while port::PortDataPort::try_read().is_some() {}

        // Step 5: Set the Controller Configuration Byte
        // log::trace!("step 5");
        let mut config = port::PortCommandRegister::get_controller_configuration_byte();
        // log::debug!("{:?}", config);
        assert!(config.system_flag());
        config.set_first_port_interrupt_is_enable(false);
        config.set_second_port_interrupt_is_enable(false);
        config.set_first_port_clock_disabled(true);
        config.set_second_port_clock_disabled(true);
        port::PortCommandRegister::set_controller_configuration_byte(config);
        // let config = port::PortCommandRegister::get_controller_configuration_byte();
        // log::debug!("{:?}", config);

        // Step 6: Perform Controller Self Test
        log::trace!("step 6");
        port::PortCommandRegister::test_controller().expect("test failed");
        // This can reset the PS/2 controller on some hardware (tested on a 2016 laptop).
        port::PortCommandRegister::set_controller_configuration_byte(config);

        // Step 7: Determine If There Are 2 Channels
        log::trace!("step 7");
        // пробуем включить порт 2
        port::PortCommandRegister::enable_second_port();
        let cfg = port::PortCommandRegister::get_controller_configuration_byte();
        // TODO сохранять во внутреннем состоянии драйвера!
        let mut is_dual = false;
        if !cfg.second_port_clock_disabled() {
            is_dual = true;
            // выключаем обратно
            port::PortCommandRegister::disable_second_port();
            port::PortCommandRegister::set_controller_configuration_byte(config);
        }

        // Step 8: Perform Interface Tests
        log::trace!("step 8");
        // At this stage, check to see how many PS/2 ports are left.
        port::PortCommandRegister::test_first_port().expect("test failed");
        if is_dual {
            port::PortCommandRegister::test_second_port().expect("test failed");
        }

        // Step 9: Enable Devices
        log::trace!("step 9");

        // todo!()
    }

    fn remove() {
        log::trace!("I8042::remove()");
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

mod port {
    pub struct PortCommandRegister;
    pub struct PortStatusRegister;
    pub struct PortDataPort;
}

mod dto {
    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum Commands {
        /// Read "byte 0" from internal RAM [`ControllerConfigurationByte`]
        ReadByte0 = 0x20,
        // 0x21 to 0x3F Read "byte N" from internal RAM (where 'N' is the command byte & 0x1F).
        // Response Byte: Unknown (only the first byte of internal RAM has a standard purpose)
        /// Write next byte to "byte 0" of internal RAM  [`ControllerConfigurationByte`]
        WriteByte0 = 0x60,
        // 0x61 to 0x7F Write next byte to "byte N" of internal RAM (where 'N' is the command byte & 0x1F)
        /// Disable second PS/2 port (only if 2 PS/2 ports supported)
        DisableSecondPort = 0xA7,
        /// Enable second PS/2 port (only if 2 PS/2 ports supported)
        EnableSecondPort = 0xA8,
        /// Test second PS/2 port (only if 2 PS/2 ports supported)
        TestSecondPort = 0xA9,
        /// Test PS/2 Controller
        TestController = 0xAA,
        /// Test first PS/2 port
        TestFirstPort = 0xAB,
        // 0xAC Diagnostic dump (read all bytes of internal RAM). Response Byte: Unknown
        /// Disable first PS/2 port
        DisableFirstPort = 0xAD,
        /// Enable first PS/2 port
        EnableFirstPort = 0xAE,
        // ...
    }

    /// The Status Register contains various flags that show the state of the PS/2 controller
    #[derive(Copy, Clone)]
    pub struct StatusRegister(pub u8);

    #[derive(Copy, Clone)]
    pub struct ControllerConfigurationByte(pub u8);
}

#[allow(dead_code)]
impl port::PortCommandRegister {
    const PORT: PortGeneric<u8, WriteOnlyAccess> = PortWriteOnly::<u8>::new(0x0064);

    pub fn disable_first_port() {
        Self::write(dto::Commands::DisableFirstPort.into());
        // Response Byte: None
    }

    pub fn disable_second_port() {
        Self::write(dto::Commands::DisableSecondPort.into());
        // Response Byte: None
    }

    pub fn enable_first_port() {
        Self::write(dto::Commands::EnableFirstPort.into());
        // Response Byte: None
    }

    pub fn enable_second_port() {
        Self::write(dto::Commands::EnableSecondPort.into());
        // Response Byte: None
    }

    pub fn get_controller_configuration_byte() -> dto::ControllerConfigurationByte {
         log::trace!("PortCommandRegister::get_controller_configuration_byte");
        Self::write(dto::Commands::ReadByte0.into());
        dto::ControllerConfigurationByte(unsafe { port::PortDataPort::read() })
    }

    pub fn set_controller_configuration_byte(config: dto::ControllerConfigurationByte) {
        Self::write(dto::Commands::WriteByte0.into());
        port::PortDataPort::write(config.into());
        // Response Byte: None
    }

    pub fn test_controller() -> Result<(), ()> {
        Self::write(dto::Commands::TestController.into());
        match unsafe { port::PortDataPort::read() } {
            0x55 => Ok(()),
            0xFC => Err(()),
            _ => Err(()),
        }
    }

    pub fn test_first_port() -> Result<(), ()> {
        Self::write(dto::Commands::TestFirstPort.into());
        Self::test_port()
    }

    pub fn test_second_port() -> Result<(), ()> {
        Self::write(dto::Commands::TestSecondPort.into());
        Self::test_port()
    }

    fn test_port() -> Result<(), ()> {
        match unsafe { port::PortDataPort::read() } {
            0x00 => Ok(()),
            0x01 => Err(()), // clock line stuck
            0x02 => Err(()), // clock line stuck high
            0x03 => Err(()), // data line stuck low
            0x04 => Err(()), // data line stuck high
            _ => Err(()),
        }
    }

    fn write(value: u8) {
        log::trace!("port_write(0x64, {:#02x})", value);
        unsafe { Self::PORT.write(value) };
    }
}

impl port::PortStatusRegister {
    const PORT: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::<u8>::new(0x0064);

    pub fn read() -> dto::StatusRegister {
        log::trace!("port_read(0x64)");
        dto::StatusRegister(unsafe { Self::PORT.read() })
    }
}

impl port::PortDataPort {
    const PORT: PortGeneric<u8, ReadWriteAccess> = Port::<u8>::new(0x0060);

    pub unsafe fn read() -> u8 {
        // TODO spinlock
        let mut count = 0;
        loop {
            if let Some(value) = Self::try_read() {
                return value;
            }
            count += 1;
            if count > 2 {
                panic!("spinlock");
            }
        }
    }

    pub fn try_read() -> Option<u8> {
        // must be set before attempting to read data from IO port 0x60
        if port::PortStatusRegister::read().output_buffer_is_full() {
            log::trace!("port_read(0x60)");
            Some(unsafe { Self::PORT.read() })
        } else {
            None
        }
    }

    pub fn write(value: u8) {
        log::trace!("port_write(0x60, {:#02x})", value);
        unsafe { Self::PORT.write(value) };
    }
}

impl From<dto::Commands> for u8 {
    fn from(value: dto::Commands) -> Self {
        value as _
    }
}

impl dto::StatusRegister {
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

    /// Command/data (0 = data written to input buffer is data for PS/2 device, 1 = data written to input buffer is data for PS/2 controller command)
    pub fn is_command(&self) -> bool {
        self.0.get_bit(3)
    }

    // 4 Unknown (chipset specific)
    // May be "keyboard lock" (more likely unused on modern systems)

    // 5 Unknown (chipset specific)
    // May be "receive time-out" or "second PS/2 port output buffer full"

    /// Time-out error (0 = no error, 1 = time-out error)
    pub fn is_timeout_error(&self) -> bool {
        self.0.get_bit(6)
    }

    /// Parity error (0 = no error, 1 = parity error)
    pub fn is_parity_error(&self) -> bool {
        self.0.get_bit(7)
    }
}

impl fmt::Debug for dto::StatusRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatusRegister")
            .field("output_buffer_is_full", &self.output_buffer_is_full())
            .field("input_buffer_is_full", &self.input_buffer_is_full())
            .field("system_flag", &self.system_flag())
            .field("is_command", &self.is_command())
            .field("is_timeout_error", &self.is_timeout_error())
            .field("is_parity_error", &self.is_parity_error())
            .finish()
    }
}

impl dto::ControllerConfigurationByte {
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

    // 3 Should be zero

    /// First PS/2 port clock (1 = disabled, 0 = enabled)
    pub fn first_port_clock_disabled(&self) -> bool {
        self.0.get_bit(4)
    }

    /// Second PS/2 port clock (1 = disabled, 0 = enabled, only if 2 PS/2 ports supported)
    pub fn second_port_clock_disabled(&self) -> bool {
        self.0.get_bit(5)
    }

    /// First PS/2 port translation (1 = enabled, 0 = disabled)
    pub fn first_port_translation_enabled(&self) -> bool {
        self.0.get_bit(6)
    }

    // 7 Must be zero

    /// First PS/2 port interrupt (1 = enabled, 0 = disabled)
    pub fn set_first_port_interrupt_is_enable(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    /// Second PS/2 port interrupt (1 = enabled, 0 = disabled, only if 2 PS/2 ports supported)
    pub fn set_second_port_interrupt_is_enable(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    /// System Flag (1 = system passed POST, 0 = your OS shouldn't be running)
    pub fn set_system_flag(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    /// First PS/2 port clock (1 = disabled, 0 = enabled)
    pub fn set_first_port_clock_disabled(&mut self, value: bool) {
        self.0.set_bit(4, value);
    }

    /// Second PS/2 port clock (1 = disabled, 0 = enabled, only if 2 PS/2 ports supported)
    pub fn set_second_port_clock_disabled(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }

    /// First PS/2 port translation (1 = enabled, 0 = disabled)
    pub fn set_first_port_translation_enabled(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }
}

impl fmt::Debug for dto::ControllerConfigurationByte {
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
            .field(
                "first_port_clock_disabled",
                &self.first_port_clock_disabled(),
            )
            .field(
                "second_port_clock_disabled",
                &self.second_port_clock_disabled(),
            )
            .field(
                "first_port_translation_enabled",
                &self.first_port_translation_enabled(),
            )
            .finish()
    }
}

impl From<dto::ControllerConfigurationByte> for u8 {
    fn from(value: dto::ControllerConfigurationByte) -> Self {
        value.0
    }
}
