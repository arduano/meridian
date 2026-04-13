use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::{
    ffi::CString,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use crate::MeridianError;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_temp_path(output: &Path, label: &str, extension: &str) -> PathBuf {
    let id = TEMP_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let stem = output
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("render");
    let dir = output
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join(format!(".{stem}.meridian-{id}.{label}.{extension}"))
}

#[cfg(not(unix))]
pub(crate) struct TempPathGuard {
    path: PathBuf,
}

#[cfg(not(unix))]
impl TempPathGuard {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[cfg(not(unix))]
impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
pub(crate) struct FifoGuard {
    path: PathBuf,
}

#[cfg(unix)]
impl FifoGuard {
    pub(crate) fn create(output: &Path, label: &str) -> Result<Self, MeridianError> {
        use std::os::unix::ffi::OsStrExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| MeridianError::Platform(format!("system clock error: {error}")))?
            .as_nanos();
        let path = next_temp_path(output, &format!("{label}-{unique}"), "fifo");
        let bytes = path.as_os_str().as_bytes();
        let c_path = CString::new(bytes).map_err(|_| {
            MeridianError::Platform(format!(
                "fifo path contains interior nulls: {}",
                path.display()
            ))
        })?;
        let result = unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) };
        if result != 0 {
            return Err(MeridianError::Platform(format!(
                "failed to create fifo {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            )));
        }
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(unix)]
impl Drop for FifoGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
