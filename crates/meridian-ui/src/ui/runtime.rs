use std::{
    cell::RefCell,
    ffi::OsStr,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use meridian_core::{
    CoreHandle, MeridianError,
    audio::{
        AudioConfig, ChannelCount, EnvelopeCurveType, Interpolator, MeridianSoundfont, ThreadCount,
    },
    display::MIN_VIEW_RANGE_SECONDS,
    midi::{
        ChangePpqTool, ChannelMapEntry, ChannelProgram, ChannelRemapTool, ControlChangeTool,
        ControlValue, ControllerMapEntry, ControllerScaleEntry, ExtractTrackTool, HumanizeTool,
        KeyMapEntry, KeyMapTool, KeyRange, MetaTextTool, MidiFileProcessingConfig,
        MidiFilesMergeConfig, MidiFilesMergeMode, MidiModifierTool, NoteLengthTool, PitchBendTool,
        ProgramTool, QuantizeTool, RangeSelectTool, SharedMetadataTrackDestination,
        SharedMetadataTrackTool, SysexTool, TempoMapDestination, TempoMapTool, TempoPoint,
        TextKind, TimeWarpPoint, TimeWarpTool, TrackMapEntry, TrackRouteTool, VelocityMapTool,
        VelocityPoint,
    },
    protocol::{CoreEvent, ParsedMidiId, StateSnapshot, VideoRenderConfig},
    render::{
        DisplayTimeSpace, KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig,
        NoteProjectorConfig, PFA_BLUE_TOP_BAR_COLOR, PFA_GREEN_TOP_BAR_COLOR,
        PFA_RED_TOP_BAR_COLOR, PfaKeyboardProjectorConfig, PianoTrailClassicSceneConfig,
        ProjectorBackgroundConfig, ProjectorBackgroundScalingMode, ProjectorImageConfig,
        RendererKind, SceneConfig, ThreeDSceneConfig, ZenithPaletteSpec,
    },
    spawn_core,
};
use serde_json::Value;
use slint::ComponentHandle;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};

use super::{
    core_bridge::UiCoreBridge,
    state::{
        UiOptions, UiStartupOptions, app_has_active_midi_load, apply_events_to_app,
        apply_merge_sources_to_app, reduce_core_events,
    },
    view::{App, MidiLoadState},
    view_model::{MergeSourceInspection, MergeSourceViewModel, UiViewModel},
    viewport::ViewportRenderer,
};

static LAST_PALETTE_PNG: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static LAST_AURA_PNG: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static LAST_BACKGROUND_PNG: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

mod audio;
mod export;
mod midi_callbacks;
mod midi_load;
mod midi_state;
mod panels;
mod persistence;
mod render_export;
mod transport;
mod video;
mod viewport;

use audio::wire_audio_config_callbacks;
pub use export::run_ui;
use export::*;
use midi_callbacks::wire_midi_callbacks;
use midi_load::*;
use midi_state::*;
use panels::{
    append_merge_source_paths, default_modify_output_path, events_error_message,
    initialize_merge_panel, initialize_modify_panel, load_modify_pass_into_app,
    validate_modify_config, wire_merge_callbacks, wire_modify_callbacks,
};
use persistence::*;
use render_export::*;
use transport::wire_transport_callbacks;
use video::wire_video_callbacks;
use viewport::*;
