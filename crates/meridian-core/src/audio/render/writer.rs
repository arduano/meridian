use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};

use hound::{SampleFormat, WavSpec, WavWriter};
use xsynth_core::AudioStreamParams;

use crate::MeridianError;

pub(crate) enum AudioSampleWriter {
    Wav(WavWriter<BufWriter<File>>),
    RawF32(BufWriter<File>),
}

impl AudioSampleWriter {
    pub(crate) fn create_wav(
        output: &Path,
        audio_params: AudioStreamParams,
    ) -> Result<Self, MeridianError> {
        let spec = WavSpec {
            channels: audio_params.channels.count(),
            sample_rate: audio_params.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let writer = WavWriter::create(output, spec).map_err(|error| {
            MeridianError::Platform(format!(
                "failed to create wav writer {}: {error}",
                output.display()
            ))
        })?;
        Ok(Self::Wav(writer))
    }

    #[cfg(windows)]
    pub(crate) fn from_raw_file(file: File) -> Self {
        Self::RawF32(BufWriter::new(file))
    }

    #[cfg(unix)]
    pub(crate) fn create_raw_pipe(
        pipe_path: &Path,
        cancel: &AtomicBool,
    ) -> Result<Self, MeridianError> {
        create_unix_raw_pipe(pipe_path, cancel)
    }
}

#[cfg(unix)]
fn create_unix_raw_pipe(
    pipe_path: &Path,
    cancel: &AtomicBool,
) -> Result<AudioSampleWriter, MeridianError> {
    use std::{
        ffi::CString,
        os::{fd::FromRawFd, unix::ffi::OsStrExt},
        thread,
        time::Duration,
    };

    let c_path = CString::new(pipe_path.as_os_str().as_bytes()).map_err(|_| {
        MeridianError::Platform(format!(
            "audio pipe path contains interior nulls: {}",
            pipe_path.display()
        ))
    })?;

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(MeridianError::Cancelled(format!(
                "cancelled while opening audio pipe {}",
                pipe_path.display()
            )));
        }

        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK) };
        if fd >= 0 {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 {
                let error = std::io::Error::last_os_error();
                unsafe { libc::close(fd) };
                return Err(MeridianError::Platform(format!(
                    "failed to read audio pipe flags {}: {error}",
                    pipe_path.display()
                )));
            }
            if unsafe { libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK) } < 0 {
                let error = std::io::Error::last_os_error();
                unsafe { libc::close(fd) };
                return Err(MeridianError::Platform(format!(
                    "failed to set blocking audio pipe mode {}: {error}",
                    pipe_path.display()
                )));
            }
            let writer = unsafe { File::from_raw_fd(fd) };
            return Ok(AudioSampleWriter::RawF32(BufWriter::new(writer)));
        }

        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(code) if code == libc::ENXIO || code == libc::ENOENT || code == libc::EINTR => {
                thread::sleep(Duration::from_millis(10));
            }
            _ => {
                return Err(MeridianError::Platform(format!(
                    "failed to open audio pipe {}: {error}",
                    pipe_path.display()
                )));
            }
        }
    }
}

impl AudioSampleWriter {
    pub(crate) fn write_samples(&mut self, samples: &[f32]) -> Result<(), MeridianError> {
        match self {
            Self::Wav(writer) => {
                for sample in samples {
                    writer.write_sample(*sample).map_err(|error| {
                        MeridianError::Platform(format!("failed to write wav sample: {error}"))
                    })?;
                }
            }
            Self::RawF32(writer) => {
                for sample in samples {
                    writer.write_all(&sample.to_le_bytes()).map_err(|error| {
                        MeridianError::Platform(format!(
                            "failed to write audio pipe sample: {error}"
                        ))
                    })?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finalize(self) -> Result<(), MeridianError> {
        match self {
            Self::Wav(writer) => writer.finalize().map_err(|error| {
                MeridianError::Platform(format!("failed to finalize xsynth wav render: {error}"))
            }),
            Self::RawF32(mut writer) => writer.flush().map_err(Into::into),
        }
    }
}
