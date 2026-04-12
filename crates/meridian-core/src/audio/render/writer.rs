use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

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

    pub(crate) fn create_raw_pipe(pipe_path: &Path) -> Result<Self, MeridianError> {
        let writer = OpenOptions::new()
            .write(true)
            .open(pipe_path)
            .map_err(|error| {
                MeridianError::Platform(format!(
                    "failed to open audio pipe {}: {error}",
                    pipe_path.display()
                ))
            })?;
        Ok(Self::RawF32(BufWriter::new(writer)))
    }

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
