use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::time::Instant;

use slint::wgpu_28::wgpu;
use wgpu::util::DeviceExt;

const VIEWPORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

const SHADER: &str = r#"
struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _padding: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    let clip = positions[vertex_index];
    var out: VsOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = clip * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let centered = in.uv - vec2<f32>(0.5, 0.5);
    let dist = length(centered * vec2<f32>(uniforms.resolution.x / max(uniforms.resolution.y, 1.0), 1.0));
    let wave = 0.5 + 0.5 * sin(dist * 18.0 - uniforms.time * 2.1);
    let sweep = 0.5 + 0.5 * sin((in.uv.x * 10.0) + uniforms.time * 1.7);
    let glow = max(0.0, 1.0 - dist * 1.5);

    let base = vec3<f32>(0.04, 0.08, 0.12);
    let color = base
        + vec3<f32>(0.05, 0.32, 0.70) * sweep
        + vec3<f32>(0.85, 0.45, 0.16) * wave * glow;

    return vec4<f32>(color, 1.0);
}
"#;

slint::slint! {
    export component App inherits Window {
        in-out property <image> viewport-image;
        in-out property <string> status-text: "Waiting for the wgpu renderer";
        out property <int> viewport-px-width: viewport-box.width / 1px;
        out property <int> viewport-px-height: viewport-box.height / 1px;

        title: "Slint + wgpu embedded viewport";
        preferred-width: 1100px;
        preferred-height: 720px;
        background: rgb(16, 20, 25);

        VerticalLayout {
            padding: 18px;
            spacing: 14px;

            Rectangle {
                border-radius: 10px;
                background: rgb(23, 32, 43);
                border-width: 1px;
                border-color: rgb(47, 70, 93);
                height: 98px;

                VerticalLayout {
                    padding: 14px;
                    spacing: 6px;

                    Text {
                        text: "Embedded accelerated viewport";
                        font-size: 24px;
                        font-weight: 700;
                        color: rgb(242, 246, 251);
                    }

                    Text {
                        text: "The right-hand rectangle is a native Rust wgpu texture composed inside the Slint scene.";
                        color: rgb(185, 198, 214);
                        wrap: word-wrap;
                    }
                }
            }

            HorizontalLayout {
                spacing: 14px;

                Rectangle {
                    width: 280px;
                    border-radius: 10px;
                    background: rgb(24, 33, 43);
                    border-width: 1px;
                    border-color: rgb(45, 64, 85);

                    VerticalLayout {
                        padding: 14px;
                        spacing: 10px;

                        Text {
                            text: "Surrounding UI";
                            font-size: 20px;
                            font-weight: 600;
                            color: rgb(242, 246, 251);
                        }

                        Text {
                            text: "This panel exists to prove the viewport is embedded in normal layout, not a separate native child window.";
                            color: rgb(185, 198, 214);
                            wrap: word-wrap;
                        }

                        Rectangle {
                            height: 74px;
                            border-radius: 8px;
                            background: rgb(14, 22, 32);
                            border-width: 1px;
                            border-color: rgb(41, 66, 93);

                            VerticalLayout {
                                padding: 10px;
                                spacing: 4px;

                                Text {
                                    text: "Render path";
                                    color: rgb(142, 184, 223);
                                    font-weight: 600;
                                }

                                Text {
                                    text: "Slint layout -> imported wgpu texture -> composited in-window";
                                    color: rgb(216, 228, 241);
                                    wrap: word-wrap;
                                }
                            }
                        }

                        Rectangle {
                            height: 74px;
                            border-radius: 8px;
                            background: rgb(14, 22, 32);
                            border-width: 1px;
                            border-color: rgb(83, 57, 36);

                            VerticalLayout {
                                padding: 10px;
                                spacing: 4px;

                                Text {
                                    text: "What to expect";
                                    color: rgb(255, 191, 135);
                                    font-weight: 600;
                                }

                                Text {
                                    text: "A moving procedural shader once rendering starts and the host can open a desktop window.";
                                    color: rgb(244, 226, 213);
                                    wrap: word-wrap;
                                }
                            }
                        }
                    }
                }

                viewport-box := Rectangle {
                    horizontal-stretch: 1;
                    vertical-stretch: 1;
                    min-width: 320px;
                    min-height: 240px;
                    border-radius: 12px;
                    border-width: 2px;
                    border-color: rgb(78, 116, 150);
                    background: rgb(9, 18, 27);
                    clip: true;

                    Image {
                        x: 0;
                        y: 0;
                        width: parent.width;
                        height: parent.height;
                        source: root.viewport-image;
                        image-fit: fill;
                    }

                    Rectangle {
                        x: 14px;
                        y: 14px;
                        border-radius: 8px;
                        background: rgb(11, 22, 33);
                        border-width: 1px;
                        border-color: rgb(54, 80, 106);
                        width: status.preferred-width + 20px;
                        height: status.preferred-height + 12px;

                        status := Text {
                            x: 10px;
                            y: 6px;
                            text: root.status-text;
                            color: rgb(242, 246, 251);
                        }
                    }
                }
            }
        }
    }
}

struct RendererResources {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct ViewportTexture {
    texture: wgpu::Texture,
    allocated_size: (u32, u32),
}

struct ViewportRenderer {
    app: slint::Weak<App>,
    resources: Option<RendererResources>,
    viewport: Option<ViewportTexture>,
    start: Instant,
}

impl ViewportRenderer {
    fn new(app: slint::Weak<App>) -> Self {
        Self {
            app,
            resources: None,
            viewport: None,
            start: Instant::now(),
        }
    }

    fn handle(&mut self, state: slint::RenderingState, graphics_api: &slint::GraphicsAPI<'_>) {
        match (state, graphics_api) {
            (slint::RenderingState::RenderingSetup, slint::GraphicsAPI::WGPU28 { device, .. }) => {
                if self.resources.is_none() {
                    self.resources = Some(Self::create_resources(device));
                }
            }
            (
                slint::RenderingState::BeforeRendering,
                slint::GraphicsAPI::WGPU28 { device, queue, .. },
            ) => {
                self.render(device, queue);
            }
            (slint::RenderingState::AfterRendering, _) => {
                if let Some(app) = self.app.upgrade() {
                    app.window().request_redraw();
                }
            }
            (slint::RenderingState::RenderingTeardown, _) => {
                self.viewport = None;
                self.resources = None;
            }
            _ => {}
        }
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(app) = self.app.upgrade() else {
            return;
        };

        let width = app.get_viewport_px_width().max(1) as u32;
        let height = app.get_viewport_px_height().max(1) as u32;

        if self.resources.is_none() {
            self.resources = Some(Self::create_resources(device));
        }

        let needs_resize = self.viewport.as_ref().is_none_or(|viewport| {
            let (allocated_width, allocated_height) = viewport.allocated_size;

            width > allocated_width
                || height > allocated_height
                || width.saturating_mul(2) < allocated_width
                || height.saturating_mul(2) < allocated_height
        });

        if needs_resize {
            let allocated_width = bucketed_extent(width);
            let allocated_height = bucketed_extent(height);

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("EmbeddedViewportTexture"),
                size: wgpu::Extent3d {
                    width: allocated_width,
                    height: allocated_height,
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
                .expect("Slint should accept an RGBA8 texture for Image import");

            self.viewport = Some(ViewportTexture {
                texture,
                allocated_size: (allocated_width, allocated_height),
            });

            app.set_viewport_image(imported_image);
            app.set_status_text(slint::format!(
                "Viewport {}x{} (allocated {}x{})",
                width,
                height,
                allocated_width,
                allocated_height
            ));
        }

        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let Some(viewport) = self.viewport.as_ref() else {
            return;
        };

        let uniform_bytes = uniforms_bytes(width, height, self.start.elapsed().as_secs_f32());
        queue.write_buffer(&resources.uniform_buffer, 0, &uniform_bytes);

        let view = viewport
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("EmbeddedViewportEncoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("EmbeddedViewportPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.03,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&resources.pipeline);
            pass.set_bind_group(0, &resources.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }

    fn create_resources(device: &wgpu::Device) -> RendererResources {
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("EmbeddedViewportUniformLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(16).expect("16 is non-zero")),
                },
                count: None,
            }],
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("EmbeddedViewportUniformBuffer"),
            contents: &uniforms_bytes(1, 1, 0.0),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("EmbeddedViewportBindGroup"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("EmbeddedViewportPipelineLayout"),
            bind_group_layouts: &[&uniform_layout],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("EmbeddedViewportShader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("EmbeddedViewportPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: VIEWPORT_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        RendererResources {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }
}

fn uniforms_bytes(width: u32, height: u32, time: f32) -> [u8; 16] {
    let values = [width as f32, height as f32, time, 0.0];
    let mut bytes = [0_u8; 16];

    for (index, value) in values.into_iter().enumerate() {
        let start = index * 4;
        bytes[start..start + 4].copy_from_slice(&value.to_ne_bytes());
    }

    bytes
}

fn bucketed_extent(requested: u32) -> u32 {
    const BUCKET: u32 = 128;
    requested.max(1).div_ceil(BUCKET) * BUCKET
}

fn main() -> Result<(), Box<dyn Error>> {
    slint::BackendSelector::new()
        .require_wgpu_28(slint::wgpu_28::WGPUConfiguration::default())
        .select()?;

    let app = App::new()?;
    app.set_status_text("Initializing shared wgpu device and viewport texture".into());

    let renderer = Rc::new(RefCell::new(ViewportRenderer::new(app.as_weak())));
    let renderer_for_notifier = Rc::clone(&renderer);

    app.window()
        .set_rendering_notifier(move |state, graphics_api| {
            renderer_for_notifier
                .borrow_mut()
                .handle(state, graphics_api);
        })?;

    app.run()?;
    Ok(())
}
