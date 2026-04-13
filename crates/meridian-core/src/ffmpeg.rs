use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(windows)]
use std::fs::File;

#[cfg(unix)]
use std::{
    ffi::CString,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use crate::MeridianError;

#[cfg(windows)]
use std::os::windows::{
    ffi::OsStrExt,
    io::{AsRawHandle, FromRawHandle, OwnedHandle},
};

#[cfg(windows)]
use crate::MeridianError;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{ERROR_PIPE_CONNECTED, GetLastError, INVALID_HANDLE_VALUE},
    Storage::FileSystem::PIPE_ACCESS_OUTBOUND,
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_TYPE_BYTE, PIPE_WAIT,
    },
};

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
pub(crate) struct AudioPipeGuard {
    path: PathBuf,
}

#[cfg(unix)]
impl AudioPipeGuard {
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
impl Drop for AudioPipeGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
pub(crate) struct AudioPipeGuard {
    path: PathBuf,
    handle: Option<OwnedHandle>,
}

#[cfg(windows)]
impl AudioPipeGuard {
    pub(crate) fn create(output: &Path, label: &str) -> Result<Self, MeridianError> {
        let id = TEMP_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let stem = output
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(sanitize_pipe_component)
            .unwrap_or_else(|| "render".to_string());
        let label = sanitize_pipe_component(label);
        let path = PathBuf::from(format!(r"\\.\pipe\meridian-{stem}-{id}-{label}"));
        let wide_path = path_to_wide(&path);
        let handle = unsafe {
            CreateNamedPipeW(
                wide_path.as_ptr(),
                PIPE_ACCESS_OUTBOUND,
                PIPE_TYPE_BYTE | PIPE_WAIT,
                1,
                64 * 1024,
                64 * 1024,
                0,
                std::ptr::null(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(MeridianError::Platform(format!(
                "failed to create named audio pipe {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            )));
        }
        let handle = unsafe { OwnedHandle::from_raw_handle(handle as _) };
        Ok(Self {
            path,
            handle: Some(handle),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn connect_writer(&mut self) -> Result<File, MeridianError> {
        let handle = self.handle.take().ok_or_else(|| {
            MeridianError::Platform(format!(
                "named audio pipe {} was already connected",
                self.path.display()
            ))
        })?;
        let raw_handle = handle.as_raw_handle() as *mut core::ffi::c_void;
        let connected = unsafe { ConnectNamedPipe(raw_handle, std::ptr::null_mut()) };
        if connected == 0 {
            let error = unsafe { GetLastError() };
            if error != ERROR_PIPE_CONNECTED {
                return Err(MeridianError::Platform(format!(
                    "failed to connect named audio pipe {}: {}",
                    self.path.display(),
                    std::io::Error::from_raw_os_error(error as i32)
                )));
            }
        }
        Ok(File::from(handle))
    }
}

#[cfg(windows)]
impl Drop for AudioPipeGuard {
    fn drop(&mut self) {
        if let Some(handle) = &self.handle {
            let _ =
                unsafe { DisconnectNamedPipe(handle.as_raw_handle() as *mut core::ffi::c_void) };
        }
    }
}

#[cfg(windows)]
fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn sanitize_pipe_component(component: &str) -> String {
    let mut sanitized = String::with_capacity(component.len());
    for ch in component.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }
    if sanitized.is_empty() {
        "pipe".to_string()
    } else {
        sanitized
    }
}
