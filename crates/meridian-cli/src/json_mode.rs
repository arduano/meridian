use meridian_core::{MeridianError, protocol};

pub fn serve_json() -> Result<(), MeridianError> {
    protocol::serve_protocol_json()
}

pub fn run_one_json(raw: &str) -> Result<(), MeridianError> {
    protocol::run_one_protocol_json(raw)
}
