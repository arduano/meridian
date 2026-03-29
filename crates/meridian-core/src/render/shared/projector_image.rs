use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ProjectorImageConfig {
    Builtin { name: String },
    PngFile { path: String },
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinProjectorImage {
    pub name: &'static str,
    pub png_bytes: &'static [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedProjectorImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ProjectorImageError {
    #[error("builtin image '{0}' not found")]
    UnknownBuiltin(String),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("png decode error: {0}")]
    Png(#[from] png::DecodingError),
    #[error("unsupported png color type {0:?}")]
    UnsupportedColor(png::ColorType),
}

pub fn load_projector_image(
    selection: &ProjectorImageConfig,
    builtins: &[BuiltinProjectorImage],
) -> Result<LoadedProjectorImage, ProjectorImageError> {
    match selection {
        ProjectorImageConfig::Builtin { name } => {
            let builtin = builtins
                .iter()
                .find(|image| image.name == name)
                .ok_or_else(|| ProjectorImageError::UnknownBuiltin(name.clone()))?;
            decode_png_rgba(Cursor::new(builtin.png_bytes))
        }
        ProjectorImageConfig::PngFile { path } => {
            decode_png_rgba(BufReader::new(File::open(Path::new(path))?))
        }
    }
}

fn decode_png_rgba<R: std::io::BufRead + std::io::Seek>(
    reader: R,
) -> Result<LoadedProjectorImage, ProjectorImageError> {
    let mut decoder = png::Decoder::new(reader);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let out_size = reader
        .output_buffer_size()
        .expect("png reader should know output size after read_info");
    let mut buf = vec![0; out_size];
    let info = reader.next_frame(&mut buf)?;
    let src = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => rgb_to_rgba(src),
        png::ColorType::Grayscale => grayscale_to_rgba(src),
        png::ColorType::Indexed => src.to_vec(),
        png::ColorType::GrayscaleAlpha => grayscale_alpha_to_rgba(src),
    };
    Ok(LoadedProjectorImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn rgb_to_rgba(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() / 3 * 4);
    for rgb in src.chunks_exact(3) {
        out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    out
}

fn grayscale_to_rgba(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 4);
    for &value in src {
        out.extend_from_slice(&[value, value, value, 255]);
    }
    out
}

fn grayscale_alpha_to_rgba(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() / 2 * 4);
    for ga in src.chunks_exact(2) {
        out.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]);
    }
    out
}
