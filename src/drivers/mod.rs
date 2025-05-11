#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod i8042;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use i8042::I8042;

pub trait Driver {
    const DRIVER_NAME: &str;
    fn probe() -> Result<(), ()>;
    fn init(&mut self);
    fn remove(&mut self);
}
