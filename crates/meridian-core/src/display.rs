use std::{path::PathBuf, time::Instant};

use crate::{
    error::MeridianError,
    midi::{MIDIFileBase, MIDIFileUnion},
    protocol::{CoreErrorCode, CoreEvent, FrameStats, ImageOutputFormat},
    render::{
        ProjectedScene, SceneConfig, SceneLayout, ScenePhysicsState, headless::save_scene_headless,
        project_scene, tick_scene_physics,
    },
    transport::TransportSnapshot,
};

pub struct DisplayFrame {
    pub layout: SceneLayout,
    pub stats: FrameStats,
    pub scene: ProjectedScene,
}

pub struct LiveDisplaySession {
    midi: Option<MIDIFileUnion>,
    layout: SceneLayout,
    scene_physics: ScenePhysicsState,
    last_physics_tick: Option<Instant>,
}

impl LiveDisplaySession {
    pub fn new() -> Self {
        let layout = SceneLayout::default();
        Self {
            midi: None,
            scene_physics: ScenePhysicsState::new(&layout.scene),
            layout,
            last_physics_tick: None,
        }
    }

    pub fn load_midi(&mut self, midi: MIDIFileUnion, now: Instant) {
        self.midi = Some(midi);
        self.scene_physics.reset(&self.layout.scene);
        self.last_physics_tick = Some(now);
    }

    pub fn unload_midi(&mut self, now: Instant) {
        self.midi = None;
        self.scene_physics.reset(&self.layout.scene);
        self.last_physics_tick = Some(now);
    }

    pub fn midi_loaded(&self) -> bool {
        self.midi.is_some()
    }

    pub fn scene(&self) -> &SceneConfig {
        &self.layout.scene
    }

    pub fn layout(&self) -> &SceneLayout {
        &self.layout
    }

    pub fn set_scene_config(&mut self, scene: SceneConfig, now: Instant) {
        self.layout.scene = scene;
        self.scene_physics.reset(&self.layout.scene);
        self.last_physics_tick = Some(now);
    }

    pub fn set_view_range(
        &mut self,
        seconds: f64,
        time_space: Option<crate::render::DisplayTimeSpace>,
    ) {
        self.layout.view_range = seconds.clamp(1.0, 30.0);
        if let Some(time_space) = time_space {
            self.layout.time_space = time_space;
        }
    }

    pub fn set_key_range(&mut self, first_key: u8, last_key: u8) {
        self.layout.first_key = first_key;
        self.layout.last_key = last_key;
    }

    pub fn reset_physics(&mut self, now: Instant) {
        self.scene_physics.reset(&self.layout.scene);
        self.last_physics_tick = Some(now);
    }

    pub fn mark_physics_tick(&mut self, now: Instant) {
        self.last_physics_tick = Some(now);
    }

    pub fn refresh_note_colors(&mut self) -> Result<(), MeridianError> {
        let Some(midi) = self.midi.as_mut() else {
            return Ok(());
        };
        let colors = match &self.layout.scene {
            SceneConfig::TwoD(scene) => scene
                .notes
                .palette()
                .build_color_table(midi.track_count())?,
            SceneConfig::ThreeD(scene) => scene.palette().build_color_table(midi.track_count())?,
        };
        midi.apply_default_track_colors(colors);
        Ok(())
    }

    pub fn midi_length(&self) -> f64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.midi_length())
            .unwrap_or(0.0)
    }

    pub fn total_notes(&self) -> u64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.stats().total_notes)
            .unwrap_or(0)
    }

    pub fn apply_viewport_overrides(
        &mut self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<(), MeridianError> {
        if let Some(viewport_width) = viewport_width {
            if viewport_width == 0 {
                return Err(MeridianError::InvalidMidi(
                    "viewport_width must be > 0".into(),
                ));
            }
            self.layout.viewport_width = viewport_width;
        }
        if let Some(viewport_height) = viewport_height {
            if viewport_height == 0 {
                return Err(MeridianError::InvalidMidi(
                    "viewport_height must be > 0".into(),
                ));
            }
            self.layout.viewport_height = viewport_height;
        }
        Ok(())
    }

    pub fn validate_layout(&self) -> Result<(), CoreEvent> {
        if self.layout.viewport_width == 0 || self.layout.viewport_height == 0 {
            return Err(CoreEvent::Error {
                code: CoreErrorCode::InvalidViewport,
                message: "viewport dimensions must be greater than zero".into(),
            });
        }
        if self.layout.first_key > self.layout.last_key {
            return Err(CoreEvent::Error {
                code: CoreErrorCode::InvalidLayout,
                message: "first_key must be <= last_key".into(),
            });
        }
        Ok(())
    }

    pub fn tick_projector_physics(
        &mut self,
        current_time: f64,
        delta_seconds: f64,
    ) -> Result<(), MeridianError> {
        let Some(midi) = self.midi.as_mut() else {
            return Ok(());
        };
        tick_scene_physics(
            midi,
            current_time,
            &self.layout,
            &mut self.scene_physics,
            delta_seconds,
        );
        Ok(())
    }

    pub fn sync_projector_physics(&mut self, current_time: f64) -> Result<(), MeridianError> {
        let now = Instant::now();
        if let Some(last_tick) = self.last_physics_tick {
            self.tick_projector_physics(current_time, now.duration_since(last_tick).as_secs_f64())?;
        }
        self.last_physics_tick = Some(now);
        Ok(())
    }

    pub fn render_frame(
        &mut self,
        transport: TransportSnapshot,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<DisplayFrame, MeridianError> {
        self.sync_projector_physics(transport.current_time)?;
        self.apply_viewport_overrides(viewport_width, viewport_height)?;
        self.validate_layout()
            .map_err(|event| MeridianError::InvalidMidi(format!("{event:?}")))?;

        let scene = self.project_current_scene(transport.current_time)?;
        let stats = FrameStats::from_scene(&scene);

        Ok(DisplayFrame {
            layout: self.layout.clone(),
            stats,
            scene,
        })
    }

    pub fn save_frame_headless(
        &mut self,
        frame: DisplayFrame,
        output: PathBuf,
        format: ImageOutputFormat,
        state: crate::protocol::StateSnapshot,
    ) -> Result<CoreEvent, MeridianError> {
        let bytes_written = save_scene_headless(
            frame.layout.viewport_width,
            frame.layout.viewport_height,
            &frame.layout,
            &frame.scene,
            format,
            &output,
        )?;
        Ok(CoreEvent::FrameSaved {
            output,
            format,
            state,
            stats: frame.stats,
            bytes_written,
        })
    }

    fn project_current_scene(
        &mut self,
        current_time: f64,
    ) -> Result<ProjectedScene, MeridianError> {
        let midi = self
            .midi
            .as_mut()
            .ok_or_else(|| MeridianError::InvalidMidi("no midi loaded".into()))?;
        Ok(project_scene(
            midi,
            current_time,
            Some(&self.scene_physics),
            &self.layout,
        ))
    }
}
