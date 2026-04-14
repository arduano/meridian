use std::{
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(windows)]
use std::fs::File;

#[cfg(windows)]
use std::{thread, time::Duration};

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
    Foundation::{
        ERROR_NO_DATA, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, GetLastError,
        INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::PIPE_ACCESS_OUTBOUND,
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_TYPE_BYTE,
        PIPE_WAIT, SetNamedPipeHandleState,
    },
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn spawn_ffmpeg_command(
    action: &str,
    command: &mut Command,
) -> Result<Child, MeridianError> {
    command
        .spawn()
        .map_err(|error| ffmpeg_spawn_error(action, &error))
}

pub(crate) fn ffmpeg_spawn_error(action: &str, error: &std::io::Error) -> MeridianError {
    MeridianError::ExternalTool(match error.kind() {
        std::io::ErrorKind::NotFound => format!(
            "ffmpeg is required for {action}, but Meridian could not find it.\n\
Install ffmpeg and make sure the `ffmpeg` command is available on your PATH, then restart Meridian and try again.\n\
On Windows, this usually means installing ffmpeg and adding the folder containing `ffmpeg.exe` to your PATH.\n\
Meridian does not bundle ffmpeg for you.\n\
Original error: {error}"
        ),
        _ => format!(
            "Meridian failed to start ffmpeg for {action}: {error}\n\
Make sure ffmpeg is installed and runnable from the same shell or desktop session."
        ),
    })
}

pub(crate) fn ffmpeg_exit_error(action: &str, status: ExitStatus) -> MeridianError {
    MeridianError::ExternalTool(format!(
        "ffmpeg failed during {action} with exit status {status}.\n\
If you supplied custom ffmpeg flags, codec settings, or container settings, check them first."
    ))
}

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
                PIPE_TYPE_BYTE | PIPE_NOWAIT,
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

    pub(crate) fn connect_writer(
        &mut self,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<File, MeridianError> {
        let handle = self.handle.take().ok_or_else(|| {
            MeridianError::Platform(format!(
                "named audio pipe {} was already connected",
                self.path.display()
            ))
        })?;
        let raw_handle = handle.as_raw_handle() as *mut core::ffi::c_void;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(MeridianError::Cancelled(format!(
                    "cancelled while waiting for named audio pipe {}",
                    self.path.display()
                )));
            }
            let connected = unsafe { ConnectNamedPipe(raw_handle, std::ptr::null_mut()) };
            if connected != 0 {
                break;
            }
            let error = unsafe { GetLastError() };
            match error {
                ERROR_PIPE_CONNECTED => break,
                ERROR_PIPE_LISTENING | ERROR_NO_DATA => {
                    thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    return Err(MeridianError::Platform(format!(
                        "failed to connect named audio pipe {}: {}",
                        self.path.display(),
                        std::io::Error::from_raw_os_error(error as i32)
                    )));
                }
            }
        }
        let mut wait_mode = PIPE_WAIT;
        let changed = unsafe {
            SetNamedPipeHandleState(
                raw_handle,
                &mut wait_mode,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if changed == 0 {
            let error = unsafe { GetLastError() };
            return Err(MeridianError::Platform(format!(
                "failed to switch named audio pipe {} to blocking mode: {}",
                self.path.display(),
                std::io::Error::from_raw_os_error(error as i32)
            )));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ffmpeg_message_explains_install_requirements() {
        let error = ffmpeg_spawn_error(
            "video export",
            &std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        );
        let message = error.to_string();
        assert!(message.contains("ffmpeg is required for video export"));
        assert!(message.contains("Install ffmpeg"));
        assert!(message.contains("PATH"));
        assert!(message.contains("Windows"));
    }
}
