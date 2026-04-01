use std::{fs, path::PathBuf};

use meridian_core::protocol::{ProtocolRequest, ProtocolResponse, ProtocolSchemaDemo};
use ts_rs::{Config, TS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let out_dir = repo_root.join("sdk/typescript/generated");

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)?;
    }
    fs::create_dir_all(&out_dir)?;

    let config = Config::new()
        .with_large_int("number")
        .with_import_extension(Some("ts"))
        .with_out_dir(&out_dir);

    ProtocolRequest::export_all(&config)?;
    ProtocolResponse::export_all(&config)?;
    ProtocolSchemaDemo::export_all(&config)?;

    println!("{}", out_dir.display());
    Ok(())
}
