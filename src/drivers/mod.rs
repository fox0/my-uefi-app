#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod i8042;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use i8042::I8042;

pub trait Driver {
    fn probe() -> Result<(), ()>;
    fn init();
    fn remove();
}
