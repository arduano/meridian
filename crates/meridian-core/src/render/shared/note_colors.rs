use std::{fs::File, io::BufReader, path::PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    MeridianError,
    midi::{MIDIColor, MIDIColorPair},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ZenithPaletteSpec {
    Random,
    RandomGradients,
    PngFile { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum NotePaletteConfig {
    DefaultTrackColors,
    ZenithPalette {
        palette: ZenithPaletteSpec,
        #[serde(default = "default_false")]
        randomize: bool,
    },
}

impl Default for ZenithPaletteSpec {
    fn default() -> Self {
        Self::Random
    }
}

impl Default for NotePaletteConfig {
    fn default() -> Self {
        Self::DefaultTrackColors
    }
}

impl NotePaletteConfig {
    pub fn build_color_table(
        &self,
        track_count: usize,
    ) -> Result<Vec<MIDIColorPair>, MeridianError> {
        match self {
            Self::DefaultTrackColors => Ok(MIDIColorPair::new_vec(track_count)),
            Self::ZenithPalette { palette, randomize } => {
                palette.build_color_table(track_count, *randomize)
            }
        }
    }
}

impl ZenithPaletteSpec {
    fn build_color_table(
        &self,
        track_count: usize,
        randomize: bool,
    ) -> Result<Vec<MIDIColorPair>, MeridianError> {
        let count = track_count.max(1) * 16;
        let coords = permuted_coords(count, randomize);
        match self {
            Self::Random => Ok(coords
                .into_iter()
                .map(|coord| {
                    let hue = fract(coord as f32 * 0.12345);
                    let color = hsv_to_color(hue, 1.0, 1.0);
                    MIDIColorPair::solid(color)
                })
                .collect()),
            Self::RandomGradients => Ok(coords
                .into_iter()
                .map(|coord| {
                    let hue = fract(coord as f32 * 0.12345);
                    let left = hsv_to_color(hue, 1.0, 1.0);
                    let right = hsv_to_color(fract(hue + 0.166), 1.0, 1.0);
                    MIDIColorPair::new(left, right)
                })
                .collect()),
            Self::PngFile { path } => load_palette_png(path, &coords),
        }
    }
}

fn load_palette_png(path: &PathBuf, coords: &[usize]) -> Result<Vec<MIDIColorPair>, MeridianError> {
    let file = File::open(path)
        .map_err(|e| MeridianError::InvalidMidi(format!("failed to open palette PNG: {e}")))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| MeridianError::InvalidMidi(format!("failed to read palette PNG: {e}")))?;
    match reader.info().color_type {
        png::ColorType::Rgba | png::ColorType::GrayscaleAlpha => {
            return Err(MeridianError::InvalidMidi(
                "palette PNGs must be RGB-only; alpha is not supported".into(),
            ));
        }
        _ => {}
    }
    let mut buffer = vec![
        0;
        reader.output_buffer_size().ok_or_else(|| {
            MeridianError::InvalidMidi("failed to size palette PNG decode buffer".into())
        })?
    ];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| MeridianError::InvalidMidi(format!("failed to decode palette PNG: {e}")))?;
    let bytes = &buffer[..info.buffer_size()];
    let width = info.width as usize;
    let height = info.height as usize;

    if height == 0 || !(width == 16 || width == 32) {
        return Err(MeridianError::InvalidMidi(
            "palette PNG must be 16px or 32px wide and at least 1px tall".into(),
        ));
    }

    let pixels = decode_rgba_pixels(bytes, &info)?;
    Ok(coords
        .iter()
        .map(|coord| {
            let x = coord % 16;
            let y = (coord / 16) % height;
            if width == 16 {
                let color = pixels[y * width + x];
                MIDIColorPair::solid(color)
            } else {
                let left = pixels[y * width + x * 2];
                let right = pixels[y * width + x * 2 + 1];
                MIDIColorPair::new(left, right)
            }
        })
        .collect())
}

fn decode_rgba_pixels(
    bytes: &[u8],
    info: &png::OutputInfo,
) -> Result<Vec<MIDIColor>, MeridianError> {
    let components = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Indexed => 4,
    };

    let mut colors = Vec::with_capacity((info.width * info.height) as usize);
    for chunk in bytes.chunks_exact(components) {
        if matches!(components, 2 | 4) {
            let alpha = chunk[components - 1];
            if alpha != 255 {
                return Err(MeridianError::InvalidMidi(
                    "palette PNGs must be fully opaque RGB-only images".into(),
                ));
            }
        }
        let color = match components {
            1 => MIDIColor::new(chunk[0], chunk[0], chunk[0]),
            2 => MIDIColor::new(chunk[0], chunk[0], chunk[0]),
            3 => MIDIColor::new(chunk[0], chunk[1], chunk[2]),
            4 => MIDIColor::new(chunk[0], chunk[1], chunk[2]),
            _ => unreachable!(),
        };
        colors.push(color);
    }
    Ok(colors)
}

fn permuted_coords(count: usize, randomize: bool) -> Vec<usize> {
    let mut coords: Vec<usize> = (0..count).collect();
    if randomize {
        coords.sort_by_key(|&index| splitmix64(index as u64 ^ 0x9E37_79B9_7F4A_7C15));
    }
    coords
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fract(value: f32) -> f32 {
    value - value.floor()
}

fn hsv_to_color(h: f32, s: f32, v: f32) -> MIDIColor {
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    MIDIColor::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

const fn default_false() -> bool {
    false
}
