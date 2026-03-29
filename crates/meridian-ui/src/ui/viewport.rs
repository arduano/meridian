use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Instant,
};

use meridian_core::{
    CoreHandle,
    render::pfa::wgpu::{PrimitiveSceneRenderer, VIEWPORT_FORMAT},
};
use slint::wgpu_28::wgpu;

use super::{
    state::{UiFrameUpdate, apply_frame_update_to_app},
    view::App,
    view_model::UiViewModel,
};

struct ViewportTexture {
    texture: wgpu::Texture,
    size: (u32, u32),
}

pub(super) struct ViewportRenderer {
    app: slint::Weak<App>,
    core: CoreHandle,
    shared_state: Arc<Mutex<UiViewModel>>,
    pending_viewport_image: Rc<RefCell<Option<slint::Image>>>,
    viewport_size: Rc<RefCell<(u32, u32)>>,
    renderer: Option<PrimitiveSceneRenderer>,
    viewport: Option<ViewportTexture>,
    enabled: bool,
    fps_smoothed: f32,
    last_frame_at: Option<Instant>,
}

impl ViewportRenderer {
    pub(super) fn new(
        app: slint::Weak<App>,
        core: CoreHandle,
        shared_state: Arc<Mutex<UiViewModel>>,
        pending_viewport_image: Rc<RefCell<Option<slint::Image>>>,
        viewport_size: Rc<RefCell<(u32, u32)>>,
        enabled: bool,
    ) -> Self {
        Self {
            app,
            core,
            shared_state,
            pending_viewport_image,
            viewport_size,
            renderer: None,
            viewport: None,
            enabled,
            fps_smoothed: 0.0,
            last_frame_at: None,
        }
    }

    pub(super) fn handle(
        &mut self,
        state: slint::RenderingState,
        graphics_api: &slint::GraphicsAPI<'_>,
    ) {
        if !self.enabled {
            return;
        }

        match (state, graphics_api) {
            (slint::RenderingState::RenderingSetup, slint::GraphicsAPI::WGPU28 { device, .. }) => {
                if self.renderer.is_none() {
                    self.renderer = Some(PrimitiveSceneRenderer::new(device));
                }
            }
            (
                slint::RenderingState::BeforeRendering,
                slint::GraphicsAPI::WGPU28 { device, queue, .. },
            ) => self.render(device, queue),
            (slint::RenderingState::RenderingTeardown, _) => {
                self.renderer = None;
                self.viewport = None;
            }
            _ => {}
        }
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let (width, height) = *self.viewport_size.borrow();

        if self
            .viewport
            .as_ref()
            .is_none_or(|viewport| viewport.size != (width, height))
        {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MeridianViewportTexture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: VIEWPORT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let imported_image = slint::Image::try_from(texture.clone())
                .expect("Slint should accept the off-screen viewport texture");
            self.viewport = Some(ViewportTexture {
                texture,
                size: (width, height),
            });
            *self.pending_viewport_image.borrow_mut() = Some(imported_image);
        }

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(viewport) = self.viewport.as_ref() else {
            return;
        };
        let Ok(frame) = self.core.render_frame(Some(width), Some(height)) else {
            return;
        };

        let mut fps_text = String::new();
        let now = Instant::now();
        if let Some(previous) = self.last_frame_at {
            let delta = now.duration_since(previous).as_secs_f32();
            if delta > 0.0 {
                let fps = 1.0 / delta;
                self.fps_smoothed = if self.fps_smoothed == 0.0 {
                    fps
                } else {
                    self.fps_smoothed * 0.88 + fps * 0.12
                };
                fps_text = format!("{:.1}", self.fps_smoothed);
            }
        }
        self.last_frame_at = Some(now);
        renderer.render(device, queue, &viewport.texture, &frame.scene);

        let shared_state = Arc::clone(&self.shared_state);
        let update = UiFrameUpdate {
            state: frame.state.clone(),
            visible_notes: frame.stats.visible_notes,
            active_keys: frame.stats.active_keys,
            fps_text,
        };
        let app_weak = self.app.clone();
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            apply_frame_update_to_app(&app, &shared_state, &update);
        });
    }
}
