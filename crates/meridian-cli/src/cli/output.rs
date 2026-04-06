use std::io::{self, Write};

use meridian_core::MeridianError;

pub(super) fn print_json_to_stdout(
    value: &impl serde::Serialize,
    pretty: bool,
) -> Result<(), MeridianError> {
    let mut stdout = io::stdout().lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, value)
            .map_err(|error| MeridianError::Platform(error.to_string()))?;
    } else {
        serde_json::to_writer(&mut stdout, value)
            .map_err(|error| MeridianError::Platform(error.to_string()))?;
    }
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
