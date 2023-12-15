pub mod cpufreq;

mod utils;

pub(crate) use utils::{impl_sysfs_read, SYSFS_MAX_ATTR_BYTES};

pub type Result<T> = std::result::Result<T, SysfsError>;

#[derive(Debug, thiserror::Error)]
pub enum SysfsError {
    /// Kernel documentation says that if you get os error 2 that
    /// means a feature is unavailable.
    #[error("the requested sysfs attribute does not exist")]
    MissingAttribute,
    #[error("encountered IO error: {0}")]
    Io(#[from] std::io::Error),
}
