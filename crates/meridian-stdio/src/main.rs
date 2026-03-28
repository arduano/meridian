mod benchmark;
mod cli;
mod frame_stdout;
mod json_mode;
mod render_video;

fn main() -> Result<(), meridian_core::MeridianError> {
    cli::run()
}
