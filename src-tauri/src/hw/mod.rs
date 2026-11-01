//! Access to the firmware's Gigabyte ACPI-WMI methods. Logic modules only
//! see the `GbWmi` trait, so they can be tested without the hardware.

#[cfg(test)]
pub mod fake;
pub mod com;
pub mod hid;

#[derive(Debug, thiserror::Error)]
pub enum HwError {
    #[error("firmware interface unavailable: {0}")]
    Unavailable(String),
    #[error("{method} failed: {detail}")]
    Call { method: String, detail: String },
}

pub trait GbWmi: Send + Sync {
    fn set(&self, method: &str, value: u32) -> Result<(), HwError>;
    fn get(&self, method: &str) -> Result<u32, HwError>;
}
