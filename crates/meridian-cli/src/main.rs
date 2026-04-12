mod benchmark;
mod cli;
mod debug_piano_trail_classic;
mod frame_stdout;
mod json_mode;
mod render_common;
mod render_audio;
mod render_video;

fn main() -> Result<(), meridian_core::MeridianError> {
    cli::run()
}
