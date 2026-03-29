use std::io::{self, Write};

use meridian_core::{
    MeridianError,
    render::{
        SceneConfig, ThreeDSceneConfig, miditrail::debug::dump_miditrail_geometry,
        shared::MiditrailSceneConfig,
    },
};

pub fn run(
    first_key: u8,
    last_key: u8,
    width: u32,
    height: u32,
    scene_json: Option<&str>,
) -> Result<(), MeridianError> {
    let config = match scene_json {
        Some(raw) => match serde_json::from_str::<SceneConfig>(raw)
            .map_err(|err| MeridianError::Platform(format!("invalid scene json: {err}")))?
        {
            SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(config)) => config,
            _ => {
                return Err(MeridianError::Platform(
                    "scene json must be a three_d miditrail scene".to_string(),
                ));
            }
        },
        None => MiditrailSceneConfig::default(),
    };

    let dump = dump_miditrail_geometry(&config, first_key, last_key, width, height);
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, &dump)
        .map_err(|err| MeridianError::Platform(format!("json encode failed: {err}")))?;
    writeln!(&mut lock)?;
    lock.flush()?;
    Ok(())
}
