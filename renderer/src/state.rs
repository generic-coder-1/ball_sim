use std::sync::Arc;

use bytemuck::bytes_of;
use egui_wgpu_backend::{
    wgpu::{self, core::binding_model::BindGroupEntry, BindingType, BufferUsages},
    ScreenDescriptor,
};
use shared::{
    anyhow,
    egui::{self, Context},
    egui_winit_platform::{self, Platform},
    winit::{self, platform},
};
pub use wgpu::SurfaceError;
use wgpu::{
    util::{DeviceExt, RenderEncoder},
    BindGroup, BindGroupLayoutEntry, ShaderStages,
};
use winit::window::Window;

use crate::{chunk::ChunkRenderingData, texture};

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct CameraUniform {
    pub pos: [f32; 2],
    pub scale: f32,
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    egui_renderer: egui_wgpu_backend::RenderPass,
    pub egui_platform: Platform,
    pub window: Arc<Window>,

    chunk_rendering_data: ChunkRenderingData,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("lets hope this never hapens");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        // surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        //shaders
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("background_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/chunk.wgsl").into()),
        });

        //camera
        let camera_uniform = CameraUniform {
            pos: [0.0; 2],
            scale: 1.0,
        };
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let platform = Platform::new(shared::egui_winit_platform::PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: window.scale_factor(),
            ..Default::default()
        });
        let egui_renderer = egui_wgpu_backend::RenderPass::new(&device, surface_format, 1);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            egui_renderer,
            egui_platform: platform,
            camera_buffer: camera_uniform_buffer,
            camera_bind_group,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn update_camera(&mut self, camera: CameraUniform) {
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytes_of(&camera));
    }

    pub fn render(&mut self, ui_code: impl FnOnce(&Context)) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        //egui stuff
        self.egui_platform.begin_pass();
        ui_code(&self.egui_platform.context());
        let full_output = self.egui_platform.end_pass(Some(&self.window));
        let paint_jobs = self
            .egui_platform
            .context()
            .tessellate(full_output.shapes, self.window.scale_factor() as f32);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.window.inner_size().width,
            physical_height: self.window.inner_size().height,
            scale_factor: self.window.scale_factor() as f32,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.background_pipeline);
            render_pass.set_bind_group(0, &self.screen_size_bind_group, &[]);
            render_pass.set_bind_group(1, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(2, &self.camera_bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            render_pass.forget_lifetime();
        }
        let tdelta: egui::TexturesDelta = full_output.textures_delta;
        self.egui_renderer
            .add_textures(&self.device, &self.queue, &tdelta)
            .expect("add texture ok");
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            paint_jobs.as_slice(),
            &screen_descriptor,
        );
        self.egui_renderer
            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
            .expect("ui couldn't render properly");

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
