use std::{
    fs::File,
    io::{Read, Write},
};

use flate2::{Compression, write::GzEncoder};

use crate::midi::parsed::ParsedMidiFile;

use super::MidiAnalysisFileMetrics;

pub fn analyze_file_metrics(
    parsed: &ParsedMidiFile,
    actual_track_count: usize,
) -> MidiAnalysisFileMetrics {
    let header = parsed.header();
    let source_bytes = parsed.signature().length_in_bytes;
    let gzip_bytes = parsed.cached_gzip_size().unwrap_or(0);
    MidiAnalysisFileMetrics {
        source_bytes,
        gzip_bytes,
        gzip_ratio: if source_bytes > 0 {
            gzip_bytes as f64 / source_bytes as f64
        } else {
            0.0
        },
        format: header.format,
        declared_track_count: header.declared_track_count,
        actual_track_count,
        ticks_per_quarter: ((header.time_division & 0x8000) == 0).then_some(header.time_division),
        total_event_count: parsed.total_event_count().unwrap_or(0),
    }
}

pub fn gzip_size_for_path(path: &std::path::Path) -> std::io::Result<u64> {
    let mut reader = File::open(path)?;
    let mut writer = CountingWriter::default();
    {
        let mut encoder = GzEncoder::new(&mut writer, Compression::fast());
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            encoder.write_all(&buffer[..read])?;
        }
        let _ = encoder.finish()?;
    }
    Ok(writer.bytes_written)
}

#[derive(Default)]
struct CountingWriter {
    bytes_written: u64,
}

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes_written += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
