// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>
// <https://www.kernel.org/doc/html/latest/admin-guide/sysfs-rules.html>
//
// If you see unchecked string functions being called,
// it's because *sysfs* is guaranteed to be ASCII (where we expect text).

// The maximum number of bytes that can be read from any given
// *sysfs* attribute. Generally there should be nothing larger than this.

use std::fs::OpenOptions;
use std::io::Read;

const SYSFS_MAX_ATTR_BYTES: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum SysfsError {
    #[error("the requested sysfs attribute does not exist")]
    MissingAttribute,
    #[error("encountered IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) unsafe fn sysfs_read_file(path: &str) -> Result<String, SysfsError> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    let mut buf = [0; SYSFS_MAX_ATTR_BYTES];
    let bytes_read = file.read(&mut buf)?;
    // Unchecked conversion is safe because this attribute is ASCII.
    let buf = std::str::from_utf8_unchecked(&buf[..bytes_read]);
    let buf = buf.trim_end_matches('\n');
    Ok(buf.to_owned())
}
