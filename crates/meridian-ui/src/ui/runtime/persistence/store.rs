use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;

use super::schema::UiConfigFile;

pub(super) const CONFIG_FILE_NAME: &str = "config.json";
pub(super) const CONFIG_BAK_FILE_NAME: &str = "config.json.bak";
const CONFIG_TMP_FILE_NAME: &str = "config.json.tmp";
const CONFIG_DIR_ENV_VAR: &str = "MERIDIAN_UI_CONFIG_DIR";

pub(in super::super) fn load_ui_config() -> Option<UiConfigFile> {
    match load_ui_config_from_dir(None) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("meridian-ui: {error}");
            None
        }
    }
}

pub(super) fn load_ui_config_from_dir(
    dir_override: Option<&Path>,
) -> Result<Option<UiConfigFile>, String> {
    let Some(config_dir) = resolve_config_dir(dir_override)? else {
        return Ok(None);
    };

    let primary_path = config_dir.join(CONFIG_FILE_NAME);
    let backup_path = config_dir.join(CONFIG_BAK_FILE_NAME);

    match load_ui_config_file(&primary_path) {
        Ok(Some(config)) => Ok(Some(config)),
        Ok(None) => load_ui_config_file(&backup_path),
        Err(primary_error) => match load_ui_config_file(&backup_path) {
            Ok(Some(config)) => {
                eprintln!(
                    "meridian-ui: failed to parse {}; restored from {}",
                    primary_path.display(),
                    backup_path.display()
                );
                Ok(Some(config))
            }
            Ok(None) => Err(primary_error),
            Err(backup_error) => Err(format!(
                "{primary_error}; backup also failed: {backup_error}"
            )),
        },
    }
}

pub(super) fn save_ui_config_to_dir(
    dir_override: Option<&Path>,
    config: &UiConfigFile,
) -> Result<(), String> {
    let Some(config_dir) = resolve_config_dir(dir_override)? else {
        return Err("unable to resolve config directory".into());
    };
    fs::create_dir_all(&config_dir)
        .map_err(|error| format!("failed to create {}: {error}", config_dir.display()))?;

    let primary_path = config_dir.join(CONFIG_FILE_NAME);
    let backup_path = config_dir.join(CONFIG_BAK_FILE_NAME);
    let temp_path = config_dir.join(CONFIG_TMP_FILE_NAME);

    let serialized = serde_json::to_vec_pretty(config)
        .map_err(|error| format!("failed to serialize UI config: {error}"))?;

    {
        let mut file = File::create(&temp_path)
            .map_err(|error| format!("failed to create {}: {error}", temp_path.display()))?;
        file.write_all(&serialized)
            .map_err(|error| format!("failed to write {}: {error}", temp_path.display()))?;
        file.write_all(b"\n")
            .map_err(|error| format!("failed to finalize {}: {error}", temp_path.display()))?;
        file.sync_all()
            .map_err(|error| format!("failed to flush {}: {error}", temp_path.display()))?;
    }

    replace_config_file(&primary_path, &backup_path, &temp_path)?;

    Ok(())
}

fn load_ui_config_file(path: &Path) -> Result<Option<UiConfigFile>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let config = serde_json::from_slice::<UiConfigFile>(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(Some(config))
}

fn replace_config_file(primary: &Path, backup: &Path, temp: &Path) -> Result<(), String> {
    let had_primary = primary.exists();

    if had_primary {
        if backup.exists() {
            fs::remove_file(backup)
                .map_err(|error| format!("failed to clear {}: {error}", backup.display()))?;
        }
        fs::rename(primary, backup).map_err(|error| {
            format!(
                "failed to move {} to {}: {error}",
                primary.display(),
                backup.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(temp, primary) {
        if had_primary && backup.exists() {
            let _ = fs::rename(backup, primary);
        }
        return Err(format!(
            "failed to move {} into place as {}: {error}",
            temp.display(),
            primary.display()
        ));
    }

    Ok(())
}

fn resolve_config_dir(dir_override: Option<&Path>) -> Result<Option<PathBuf>, String> {
    if let Some(dir) = dir_override {
        return Ok(Some(dir.to_path_buf()));
    }

    if let Ok(dir) = std::env::var(CONFIG_DIR_ENV_VAR) {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Ok(Some(PathBuf::from(trimmed)));
        }
    }

    Ok(ProjectDirs::from("io", "github.arduano", "Meridian")
        .map(|dirs| dirs.config_dir().to_path_buf()))
}
