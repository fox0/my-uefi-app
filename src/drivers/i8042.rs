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
#[derive(Default, Debug)]
pub struct I8042 {
    port1: Option<DeviceType>,
    port2: Option<DeviceType>,
    is_exists_port2: bool,
    config: dto::ControllerConfigurationByte,
}

#[derive(Debug)]
pub enum DeviceType {
    /// Standard PS/2 mouse
    StandardMouse,
    /// MF2 keybaord
    StandardKeyboard,
}

impl DeviceType {
    pub fn log(&self) {
        match self {
            Self::StandardMouse => {
                log::info!("{}: Found standard PS/2 mouse", I8042::DRIVER_NAME)
            }
            Self::StandardKeyboard => {
                log::info!("{}: Found standard PS/2 keyboard", I8042::DRIVER_NAME)
            }
        }
    }
}

impl Driver for I8042 {
    const DRIVER_NAME: &str = "i8042";

    fn probe() -> Result<(), ()> {
        // log::trace!("I8042::probe()");

        let fadt = fadt_raw().expect("no init FADT");
        let fadt = unsafe { fadt.as_ref() };

        // Step 1: Initialize USB Controllers

        // Step 2: Determine if the PS/2 Controller Exists
        let flags = fadt.iapc_boot_arch;
        if !flags.motherboard_implements_8042() {
            log::warn!("{}: No controller found", I8042::DRIVER_NAME);
            Err(())
        } else {
            log::info!("{}: Found PS/2 controller", I8042::DRIVER_NAME);
            Ok(())
        }
    }

    fn init(&mut self) {
        // log::trace!("I8042::init()");

        // Step 3: Disable Devices
        // log::trace!("step 3");
        port::PortCommandRegister::disable_port1();
        port::PortCommandRegister::disable_port2();

        // Step 4: Flush The Output Buffer
        // log::trace!("step 4");
        while port::PortDataPort::try_read().is_some() {}

        // Step 5: Set the Controller Configuration Byte
        // log::trace!("step 5");
        self.config = port::PortCommandRegister::get_controller_configuration_byte();
        // log::debug!("{:?}", self.config);
        assert!(self.config.system_flag());
        self.config.set_is_enable_interrupt1(false);
        self.config.set_is_enable_interrupt2(false);
        self.config.set_is_disabled_clock1(true);
        self.config.set_is_disabled_clock2(true);
        self.config.set_is_enabled_translation1(false);
        port::PortCommandRegister::set_controller_configuration_byte(self.config);
        // let config = port::PortCommandRegister::get_controller_configuration_byte();
        // log::debug!("{:?}", config);

        // Step 6: Perform Controller Self Test
        // log::trace!("step 6");
        port::PortCommandRegister::test_controller().expect("test failed");
        // This can reset the PS/2 controller on some hardware (tested on a 2016 laptop).
        port::PortCommandRegister::set_controller_configuration_byte(self.config);

        // Step 7: Determine If There Are 2 Channels
        // log::trace!("step 7");
        // пробуем включить порт 2
        port::PortCommandRegister::enable_port2();
        let cfg = port::PortCommandRegister::get_controller_configuration_byte();
        if !cfg.is_disabled_clock2() {
            self.is_exists_port2 = true;
            // выключаем обратно
            port::PortCommandRegister::disable_port2();
            port::PortCommandRegister::set_controller_configuration_byte(self.config);
        }

        // Step 8: Perform Interface Tests
        // log::trace!("step 8");
        // At this stage, check to see how many PS/2 ports are left.
        port::PortCommandRegister::test_port1().expect("test failed");
        if self.is_exists_port2 {
            port::PortCommandRegister::test_port2().expect("test failed");
        }

        // Step 9: Enable Devices
        // log::trace!("step 9");
        port::PortCommandRegister::enable_port1();
        if self.is_exists_port2 {
            port::PortCommandRegister::enable_port2();
        }

        // Step 10: Reset Devices
        // log::trace!("step 10");
        port::PortDataPort::reset(false).expect("reset failed");
        if self.is_exists_port2 {
            port::PortDataPort::reset(true).expect("reset failed");
        }

        // Detecting PS/2 Device Types
        // log::trace!("step 11");
        self.port1 = port::PortDataPort::get_dev_type(false);
        if let Some(dev) = &self.port1 {
            dev.log();
        }

        if self.is_exists_port2 {
            self.port2 = port::PortDataPort::get_dev_type(true);
            if let Some(dev) = &self.port2 {
                dev.log();
            }
        }
    }

    fn remove(&mut self) {
        // log::trace!("I8042::remove()");

        // todo!()
    }
}

mod port {
    pub struct PortCommandRegister;
    pub struct PortStatusRegister;
    pub struct PortDataPort;
}

mod dto {
    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum ControllerCommands {
        /// Read "byte 0" from internal RAM [`ControllerConfigurationByte`]
        ReadByte0 = 0x20,
        // 0x21 to 0x3F Read "byte N" from internal RAM (where 'N' is the command byte & 0x1F).
        // Response Byte: Unknown (only the first byte of internal RAM has a standard purpose)
        /// Write next byte to "byte 0" of internal RAM  [`ControllerConfigurationByte`]
        WriteByte0 = 0x60,
        // 0x61 to 0x7F Write next byte to "byte N" of internal RAM (where 'N' is the command byte & 0x1F)
        /// Disable second PS/2 port (only if 2 PS/2 ports supported)
        DisablePort2 = 0xA7,
        /// Enable second PS/2 port (only if 2 PS/2 ports supported)
        EnablePort2 = 0xA8,
        /// Test second PS/2 port (only if 2 PS/2 ports supported)
        TestPort2 = 0xA9,
        /// Test PS/2 Controller
        TestController = 0xAA,
        /// Test first PS/2 port
        TestPort1 = 0xAB,
        // 0xAC Diagnostic dump (read all bytes of internal RAM). Response Byte: Unknown
        /// Disable first PS/2 port
        DisablePort1 = 0xAD,
        /// Enable first PS/2 port
        EnablePort1 = 0xAE,
        // ...
        /// Write next byte to second PS/2 port input buffer (only if 2 PS/2 ports supported)
        /// (sends next byte to the second PS/2 port)
        WriteByteInputPort2 = 0xD4,
        // ...
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum DeviceCommands {
        Identify = 0xF2,
        DisableScanning = 0xF5,
        /// Reset command, supported by all PS/2 devices
        Reset = 0xFF,
    }

    /// The Status Register contains various flags that show the state of the PS/2 controller
    #[derive(Copy, Clone)]
    pub struct StatusRegister(pub u8);

    #[derive(Copy, Clone, Default)]
    pub struct ControllerConfigurationByte(pub u8);
}

#[allow(dead_code)]
impl port::PortCommandRegister {
    const PORT: PortGeneric<u8, WriteOnlyAccess> = PortWriteOnly::<u8>::new(0x0064);

    pub fn disable_port1() {
        Self::write(dto::ControllerCommands::DisablePort1.into());
        // Response Byte: None
    }

    pub fn disable_port2() {
        Self::write(dto::ControllerCommands::DisablePort2.into());
        // Response Byte: None
    }

    pub fn enable_port1() {
        Self::write(dto::ControllerCommands::EnablePort1.into());
        // Response Byte: None
    }

    pub fn enable_port2() {
        Self::write(dto::ControllerCommands::EnablePort2.into());
        // Response Byte: None
    }

    #[allow(clippy::let_and_return)]
    pub fn get_controller_configuration_byte() -> dto::ControllerConfigurationByte {
        Self::write(dto::ControllerCommands::ReadByte0.into());
        // TODO spinlock
        let config = dto::ControllerConfigurationByte(unsafe { port::PortDataPort::read() });
        // log::trace!("< {:?}", config);
        config
    }

    pub fn set_controller_configuration_byte(config: dto::ControllerConfigurationByte) {
        Self::write(dto::ControllerCommands::WriteByte0.into());
        // log::trace!("> {:?}", config);
        port::PortDataPort::write(config.into());
        // Response Byte: None
    }

    pub fn test_controller() -> Result<(), ()> {
        Self::write(dto::ControllerCommands::TestController.into());
        match unsafe { port::PortDataPort::read() } {
            0x55 => Ok(()),
            0xFC => Err(()),
            _ => Err(()),
        }
    }

    pub fn test_port1() -> Result<(), ()> {
        Self::write(dto::ControllerCommands::TestPort1.into());
        Self::test_port()
    }

    pub fn test_port2() -> Result<(), ()> {
        Self::write(dto::ControllerCommands::TestPort2.into());
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
        // log::trace!("CMD> {:#02X}", value);
        unsafe { Self::PORT.write(value) };
    }
}

impl port::PortStatusRegister {
    const PORT: PortGeneric<u8, ReadOnlyAccess> = PortReadOnly::<u8>::new(0x0064);

    #[allow(clippy::let_and_return)]
    pub fn read() -> dto::StatusRegister {
        let value = unsafe { Self::PORT.read() };
        // log::trace!("CMD< {:#02X}", value);
        dto::StatusRegister(value)
    }
}

impl port::PortDataPort {
    const PORT: PortGeneric<u8, ReadWriteAccess> = Port::<u8>::new(0x0060);

    /// Reset Device
    pub fn reset(is_port2: bool) -> Result<(), ()> {
        Self::send_to_device(is_port2, dto::DeviceCommands::Reset);
        let resp1 = unsafe { Self::read() };
        let resp2 = unsafe { Self::read() };
        match (resp1, resp2) {
            (0xFA, 0xAA) => Ok(()),
            _ => Err(()),
        }
    }

    /// Detecting PS/2 Device Types
    pub fn get_dev_type(is_port2: bool) -> Option<DeviceType> {
        // log::trace!("PortDataPort::get_dev_type(is_port2={})", is_port2);

        Self::send_to_device(is_port2, dto::DeviceCommands::DisableScanning);
        if unsafe { Self::read() } != 0xFA {
            // что-то с первого раза не работает...
            Self::send_to_device(is_port2, dto::DeviceCommands::DisableScanning);
            if unsafe { Self::read() } != 0xFA {
                return None;
            }
        }
        Self::send_to_device(is_port2, dto::DeviceCommands::Identify);
        if unsafe { Self::read() } != 0xFA {
            return None;
        }

        // Wait for the device to send up to 2 bytes of reply, with a time-out to determine when it's finished (e.g. in case it only sends 1 byte)
        let resp1 = unsafe { Self::read() };
        let resp2 = Self::try_read(); // TODO timeout
        match (resp1, resp2) {
            (0x00, None) => Some(DeviceType::StandardMouse),
            (0xAB, Some(0x83)) => Some(DeviceType::StandardKeyboard),
            v => {
                log::warn!(
                    "{}: Found unknown device {:#02X}, {:?}",
                    I8042::DRIVER_NAME,
                    v.0,
                    v.1
                );
                None
            }
        }
    }

    fn send_to_device(is_port2: bool, value: dto::DeviceCommands) {
        if is_port2 {
            port::PortCommandRegister::write(dto::ControllerCommands::WriteByteInputPort2.into());
        }
        // log::trace!("> {:?}", value);
        Self::write(value.into());
    }

    pub unsafe fn read() -> u8 {
        // TODO spinlock
        let mut count = 0;
        loop {
            if let Some(value) = Self::try_read() {
                return value;
            }
            count += 1;
            if count > 10 {
                panic!("read_spinlock");
            }
        }
    }

    pub fn try_read() -> Option<u8> {
        // must be set before attempting to read data from IO port 0x60
        if port::PortStatusRegister::read().output_buffer_is_full() {
            let value = unsafe { Self::PORT.read() };
            // log::trace!("< {:#02X}", value);
            Some(value)
        } else {
            None
        }
    }

    pub fn write(value: u8) {
        // TODO spinlock
        let mut count = 0;
        loop {
            if !port::PortStatusRegister::read().input_buffer_is_full() {
                // log::trace!("> {:#02X}", value);
                unsafe { Self::PORT.write(value) };
                break;
            }
            count += 1;
            if count > 10 {
                panic!("write_spinlock");
            }
        }
    }
}

impl From<dto::ControllerCommands> for u8 {
    fn from(value: dto::ControllerCommands) -> Self {
        value as _
    }
}

impl From<dto::DeviceCommands> for u8 {
    fn from(value: dto::DeviceCommands) -> Self {
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
    pub fn is_enable_interrupt1(&self) -> bool {
        self.0.get_bit(0)
    }

    /// Second PS/2 port interrupt (1 = enabled, 0 = disabled, only if 2 PS/2 ports supported)
    pub fn is_enable_interrupt2(&self) -> bool {
        self.0.get_bit(1)
    }

    /// System Flag (1 = system passed POST, 0 = your OS shouldn't be running)
    pub fn system_flag(&self) -> bool {
        self.0.get_bit(2)
    }

    // 3 Should be zero

    /// First PS/2 port clock (1 = disabled, 0 = enabled)
    pub fn is_disabled_clock1(&self) -> bool {
        self.0.get_bit(4)
    }

    /// Second PS/2 port clock (1 = disabled, 0 = enabled, only if 2 PS/2 ports supported)
    pub fn is_disabled_clock2(&self) -> bool {
        self.0.get_bit(5)
    }

    /// First PS/2 port translation (1 = enabled, 0 = disabled)
    pub fn is_enabled_translation1(&self) -> bool {
        self.0.get_bit(6)
    }

    // 7 Must be zero

    /// First PS/2 port interrupt (1 = enabled, 0 = disabled)
    pub fn set_is_enable_interrupt1(&mut self, value: bool) {
        self.0.set_bit(0, value);
    }

    /// Second PS/2 port interrupt (1 = enabled, 0 = disabled, only if 2 PS/2 ports supported)
    pub fn set_is_enable_interrupt2(&mut self, value: bool) {
        self.0.set_bit(1, value);
    }

    /// System Flag (1 = system passed POST, 0 = your OS shouldn't be running)
    pub fn set_system_flag(&mut self, value: bool) {
        self.0.set_bit(2, value);
    }

    /// First PS/2 port clock (1 = disabled, 0 = enabled)
    pub fn set_is_disabled_clock1(&mut self, value: bool) {
        self.0.set_bit(4, value);
    }

    /// Second PS/2 port clock (1 = disabled, 0 = enabled, only if 2 PS/2 ports supported)
    pub fn set_is_disabled_clock2(&mut self, value: bool) {
        self.0.set_bit(5, value);
    }

    /// First PS/2 port translation (1 = enabled, 0 = disabled)
    pub fn set_is_enabled_translation1(&mut self, value: bool) {
        self.0.set_bit(6, value);
    }
}

impl fmt::Debug for dto::ControllerConfigurationByte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControllerConfigurationByte")
            .field("is_enable_interrupt1", &self.is_enable_interrupt1())
            .field("is_enable_interrupt2", &self.is_enable_interrupt2())
            .field("system_flag", &self.system_flag())
            .field("is_disabled_clock1", &self.is_disabled_clock1())
            .field("is_disabled_clock2", &self.is_disabled_clock2())
            .field("is_enabled_translation1", &self.is_enabled_translation1())
            .finish()
    }
}

impl From<dto::ControllerConfigurationByte> for u8 {
    fn from(value: dto::ControllerConfigurationByte) -> Self {
        value.0
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
