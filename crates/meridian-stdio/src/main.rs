mod benchmark;
mod cli;
mod frame_stdout;
mod json_mode;

fn main() -> Result<(), meridian_core::MeridianError> {
    cli::run()
}
