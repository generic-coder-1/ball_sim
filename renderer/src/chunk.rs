use std::{iter::repeat, num::NonZero};

use bytemuck::{bytes_of, cast_slice};
use egui_wgpu_backend::wgpu::{
    self, util::DeviceExt, BindGroup, BindGroupEntry, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferUsages, ColorWrites, PipelineCompilationOptions, PrimitiveState, RenderPass,
    RenderPipeline, ShaderStages, SurfaceConfiguration,
};

use crate::{texture::Texture, vertex::Vertex};

pub struct ChunkRenderingData {
    pipeline: RenderPipeline,

    //group 0
    instance_array_buffer: wgpu::Buffer,
    instance_array_size: u32,
    instance_array_size_max: u32,
    instance_array_bind_group_layout: wgpu::BindGroupLayout,
    instance_array_bind_group: wgpu::BindGroup,

    //group 1
    atlas_texture: wgpu::Texture,
    atlas_info_buffer: wgpu::Buffer,
    atlas_bind_group: wgpu::BindGroup,
    //group 2 will be provided for us

    //quad
    vertex_buffer: wgpu::Buffer,
}

const CHUNK_SIZE: usize = 32;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Chunk {
    pub position: [i32; 2],
    pub data: [u8; CHUNK_SIZE * CHUNK_SIZE],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            position: Default::default(),
            data: [0; CHUNK_SIZE * CHUNK_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct AtlasInfo {
    pub tiles_per_row: u32,
    pub _pad: u32,
    pub tiles_size: [f32; 2],
}

impl ChunkRenderingData {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        atlas_texture: Texture,
        atlas_info: &AtlasInfo,
    ) -> Self {
        //dummy data because we can't create a zero sized buffer :(
        let instance_array: Vec<Chunk> = vec![
            Chunk {
                position: [0; 2],
                data: [0; CHUNK_SIZE * CHUNK_SIZE],
            };
            256
        ];
        let instance_array_size = 0;
        let instance_array_size_max = instance_array.len() as u32;
        let instance_array_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance_array_buffer"),
            contents: cast_slice(&instance_array),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let instance_array_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("instance_array_data_bind_group_layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let instance_array_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instance_array_bind_group"),
            layout: &instance_array_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: instance_array_buffer.as_entire_binding(),
            }],
        });

        let atlas_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas_info_buffer"),
            contents: bytes_of(atlas_info),
            usage: BufferUsages::UNIFORM,
        });
        let atlas_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("atlas_bind_group_layout"),
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
                        ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas_bind_group"),
            layout: &atlas_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&atlas_texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: atlas_info_buffer.as_entire_binding(),
                },
            ],
        });

        let chunk_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/chunk.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk_pipeline_layout"),
            bind_group_layouts: &[
                &instance_array_bind_group_layout,
                &atlas_bind_group_layout,
                camera_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &chunk_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &chunk_shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
                [1.0, 1.0].into(),
                [0.0, 1.0].into(),
            ]),
            usage: BufferUsages::VERTEX,
        });

        Self {
            instance_array_buffer,
            instance_array_size,
            instance_array_size_max,
            instance_array_bind_group_layout,
            instance_array_bind_group,

            atlas_texture: atlas_texture.texture,
            atlas_info_buffer,
            atlas_bind_group,
            pipeline,

            vertex_buffer,
        }
    }

    pub fn render(&self, render_pass: &mut RenderPass, camera_bind_group: &BindGroup) {
        if self.instance_array_size > 0 {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.instance_array_bind_group, &[]);
            render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);
            render_pass.set_bind_group(2, camera_bind_group, &[]);
            render_pass.set_pipeline(&self.pipeline);

            render_pass.draw(0..4, 0..self.instance_array_size);
        }
    }

    pub fn update_chunk_buffer(
        &mut self,
        chunks: Vec<&Chunk>,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        if !chunks.is_empty() {
            let mut data = chunks.iter().map(|c| bytes_of(*c)).fold(
                Vec::with_capacity(size_of::<Chunk>() * chunks.len()),
                |mut acc, val| {
                    val.iter().for_each(|&v| {
                        acc.push(v);
                    });
                    acc
                },
            );

            if chunks.len() as u32 > self.instance_array_size_max {
                while chunks.len() as u32 > self.instance_array_size_max {
                    self.instance_array_size_max *= 2;
                }
                data.extend(std::iter::repeat_n(
                    0,
                    (self.instance_array_size_max as usize - chunks.len()) * size_of::<Chunk>(),
                ));
                self.instance_array_buffer.destroy();
                self.instance_array_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("instance_array_buffer"),
                        contents: data.as_slice(),
                        usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    });
                self.instance_array_bind_group =
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("instance_array_bind_group"),
                        layout: &self.instance_array_bind_group_layout,
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: self.instance_array_buffer.as_entire_binding(),
                        }],
                    });
            } else {
                let mut view = queue
                    .write_buffer_with(
                        &self.instance_array_buffer,
                        0,
                        NonZero::new(data.len() as u64)
                            .expect("we literally checked that chunks was not empty earlier"),
                    )
                    .expect("buffer can be written to");

                view.copy_from_slice(data.as_slice());
            }
            queue.submit([]);
        }
        self.instance_array_size = chunks.len() as u32;
    }
}
