use bytemuck::cast_slice;
use egui_wgpu_backend::wgpu::{
    self, util::DeviceExt, BindGroupEntry, BindGroupLayoutEntry,
    BindingType, BufferUsages, PipelineCompilationOptions, PrimitiveState, RenderPass,
    ShaderStages, SurfaceConfiguration,
};
use shared::egui::Direction as EguiDirection;

use crate::{texture::Texture, vertex::Vertex};

pub struct BallRenderingData {
    pipeline: wgpu::RenderPipeline,

    instance_position_buffer: wgpu::Buffer,
    instance_on_buffer: wgpu::Buffer,
    instance_array_size: u32,
    instance_bind_group: wgpu::BindGroup,

    texture_bind_group: wgpu::BindGroup,

    //quad
    vertex_buffer: wgpu::Buffer,
}

#[repr(C, align(4))]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq, Eq, Hash, Default)]
pub struct BallPosition {
    pub position: [i32; 2],
}

pub struct BallsOn {
    data: Vec<u32>,
}

impl From<Vec<(bool, Direction)>> for BallsOn {
    fn from(value: Vec<(bool, Direction)>) -> Self {
        Self {
            data: value
                .iter()
                .map(|(on, dir)| (if *on { 1 } else { 0 }) | u32::from(*dir) << 1)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl From<Direction> for u32 {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Right => 0,
            Direction::Up => 1,
            Direction::Down => 2,
            Direction::Left => 3,
        }
    }
}

const MAX_BALLS: u32 = 2 << 14;

impl BallRenderingData {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        ball_texture: Texture,
        dir_texture: Texture,
        surface_config: &SurfaceConfiguration,
    ) -> Self {
        let positions_array = vec![BallPosition { position: [0; 2] }; MAX_BALLS as usize];
        let data_array: BallsOn = vec![(true, Direction::Right); MAX_BALLS as usize].into();
        let instance_array_size = 0;
        let instance_position_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_position_buffer"),
                contents: bytemuck::cast_slice(&positions_array),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
        let instance_on_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance_on_buffer"),
            contents: bytemuck::cast_slice(&data_array.data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let instance_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("instance_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let instance_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instance_bind_group"),
            layout: &instance_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instance_position_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: instance_on_buffer.as_entire_binding(),
                },
            ],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&ball_texture.view),
            },BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&dir_texture.view),
            }
            ],
        });

        let ball_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ball_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/ball.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ball_pipline_layout"),
            bind_group_layouts: &[
                &instance_bind_group_layout,
                &texture_bind_group_layout,
                camera_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ball_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ball_shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk_vertex_buffer"),
            contents: cast_slice::<Vertex, u8>(&[
                [0.0, 0.0].into(),
                [1.0, 0.0].into(),
                [0.0, 1.0].into(),
                [1.0, 1.0].into(),
            ]),
            usage: BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            instance_position_buffer,
            instance_on_buffer,
            instance_array_size,
            instance_bind_group,
            texture_bind_group,
            vertex_buffer,
        }
    }

    pub fn render(&self, render_pass: &mut RenderPass, camera_bind_group: &wgpu::BindGroup) {
        if self.instance_array_size > 0 {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.instance_bind_group, &[]);
            render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(2, camera_bind_group, &[]);
            render_pass.set_pipeline(&self.pipeline);

            render_pass.draw(0..4, 0..self.instance_array_size);
        }
    }

    pub fn update_balls(&mut self, queue: &wgpu::Queue, pos: Vec<BallPosition>, data: Vec<(bool, Direction)>) {
        if pos.len() != data.len() {
            panic!("sizes of data is incorrect");
        }
        if data.len() > MAX_BALLS as usize {
            panic!("drawing too many balls");
        }
        self.instance_array_size = data.len() as u32;
        queue.write_buffer(
            &self.instance_position_buffer,
            0,
            bytemuck::cast_slice(pos.as_slice()),
        );
        queue.write_buffer(
            &self.instance_on_buffer,
            0,
            bytemuck::cast_slice(
                data.iter()
                    .map(|(on, dir)| if *on { 1 } else { 0 } | u32::from(*dir)<<1)
                    .collect::<Vec<u32>>()
                    .as_slice(),
            ),
        );
    }
}
