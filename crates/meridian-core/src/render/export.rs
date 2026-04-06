use std::path::{Path, PathBuf};

use crate::{
    MeridianError,
    protocol::{FrameColorMode, ImageExportArtifacts, ImageOutputFormat},
    render::pfa::wgpu::{encode_rgba_to_png, encode_rgba_to_ppm},
};

pub struct ExportFrame {
    width: u32,
    height: u32,
    premultiplied_rgba: Vec<u8>,
}

impl ExportFrame {
    pub fn new(width: u32, height: u32, premultiplied_rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            premultiplied_rgba,
        }
    }

    pub fn write_color_output(
        &self,
        output: &Path,
        format: ImageOutputFormat,
        mode: FrameColorMode,
    ) -> Result<u64, MeridianError> {
        let mut rgba = Vec::new();
        self.fill_color_rgba(mode, &mut rgba);
        write_rgba_output(output, format, self.width, self.height, &rgba)
    }

    pub fn write_requested_sidecars(
        &self,
        output: &Path,
        export_premultiplied_rgb: bool,
        export_straight_rgb: bool,
        export_alpha_mask: bool,
    ) -> Result<ImageExportArtifacts, MeridianError> {
        let mut artifacts = ImageExportArtifacts::default();

        if export_premultiplied_rgb {
            let path = sidecar_path(output, "premultiplied");
            let mut rgba = Vec::new();
            self.fill_color_rgba(FrameColorMode::Premultiplied, &mut rgba);
            write_rgba_output(
                &path,
                ImageOutputFormat::infer_from_path(&path),
                self.width,
                self.height,
                &rgba,
            )?;
            artifacts.premultiplied_rgb = Some(path);
        }

        if export_straight_rgb {
            let path = sidecar_path(output, "straight");
            let mut rgba = Vec::new();
            self.fill_color_rgba(FrameColorMode::Straight, &mut rgba);
            write_rgba_output(
                &path,
                ImageOutputFormat::infer_from_path(&path),
                self.width,
                self.height,
                &rgba,
            )?;
            artifacts.straight_rgb = Some(path);
        }

        if export_alpha_mask {
            let path = sidecar_path(output, "alpha");
            let mut rgba = Vec::new();
            self.fill_alpha_mask_rgba(&mut rgba);
            write_rgba_output(
                &path,
                ImageOutputFormat::infer_from_path(&path),
                self.width,
                self.height,
                &rgba,
            )?;
            artifacts.alpha_mask = Some(path);
        }

        Ok(artifacts)
    }

    pub fn fill_color_rgba(&self, mode: FrameColorMode, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.premultiplied_rgba.len());
        match mode {
            FrameColorMode::Premultiplied => {
                for pixel in self.premultiplied_rgba.chunks_exact(4) {
                    out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
            FrameColorMode::Straight => {
                for pixel in self.premultiplied_rgba.chunks_exact(4) {
                    let alpha = pixel[3];
                    let [r, g, b] = if alpha == 0 {
                        [0, 0, 0]
                    } else {
                        [
                            unpremultiply_channel(pixel[0], alpha),
                            unpremultiply_channel(pixel[1], alpha),
                            unpremultiply_channel(pixel[2], alpha),
                        ]
                    };
                    out.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }
    }

    pub fn fill_alpha_mask_rgba(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.premultiplied_rgba.len());
        for pixel in self.premultiplied_rgba.chunks_exact(4) {
            let alpha = pixel[3];
            out.extend_from_slice(&[alpha, alpha, alpha, 255]);
        }
    }

    pub fn fill_alpha_luma(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.premultiplied_rgba.len() / 4);
        for pixel in self.premultiplied_rgba.chunks_exact(4) {
            out.push(pixel[3]);
        }
    }
}

pub fn sidecar_path(output: &Path, suffix: &str) -> PathBuf {
    let parent = output.parent().map(Path::to_path_buf).unwrap_or_default();
    let extension = output.extension().and_then(|ext| ext.to_str());
    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("output");
    let file_name = match extension {
        Some(extension) if !extension.is_empty() => format!("{stem}.{suffix}.{extension}"),
        _ => format!("{stem}.{suffix}"),
    };
    parent.join(file_name)
}

fn write_rgba_output(
    output: &Path,
    format: ImageOutputFormat,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<u64, MeridianError> {
    let bytes = match format {
        ImageOutputFormat::Ppm => encode_rgba_to_ppm(width, height, rgba),
        ImageOutputFormat::Png => encode_rgba_to_png(width, height, rgba)?,
        ImageOutputFormat::Rgba => rgba.to_vec(),
    };
    std::fs::write(output, &bytes)?;
    Ok(bytes.len() as u64)
}

fn unpremultiply_channel(color: u8, alpha: u8) -> u8 {
    (((color as u16) * 255 + (alpha as u16 / 2)) / alpha as u16).min(255) as u8
}
