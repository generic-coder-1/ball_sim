use bytemuck::{bytes_of, cast_slice};
use egui_wgpu_backend::wgpu::{
    self, util::DeviceExt, BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType, BufferUsages, PipelineCompilationOptions, RenderPipeline, ShaderStages
};

use crate::texture::Texture;

pub struct ChunkRenderingData {
    pipeline: RenderPipeline,

    //group 0
    instance_array_buffer: wgpu::Buffer,
    instance_array_bind_group: wgpu::BindGroup,

    //group 1
    atlas_texture: wgpu::Texture,
    atlas_info_buffer: wgpu::Buffer,
    atlas_bind_group: wgpu::BindGroup,
    //group 2 will be provided for us
}

const CHUNK_SIZE: usize = 32;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Chunk {
    position: [i32; 2],
    data: [u32; CHUNK_SIZE * CHUNK_SIZE],
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
    tiles_per_row: u32,
    tiles_size: [f32; 2],
}

impl ChunkRenderingData {
    pub fn new(
        device: wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        atlas_texture: Texture,
        atlas_info: &AtlasInfo,
    ) -> Self {
        let instance_array: Vec<Chunk> = vec![];
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
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/chunk.wgsl").into())
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk_pipeline_layout"),
            bind_group_layouts: &[
                &instance_array_bind_group_layout,
                &atlas_bind_group_layout,
                &camera_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &chunk_shader, entry_point: Some("fs_main"), compilation_options: PipelineCompilationOptions::default(), buffers: () },
            primitive: (),
            depth_stencil: (),
            multisample: (),
            fragment: (),
            multiview: (),
            cache: (),
        });

        Self {
            instance_array_buffer,
            instance_array_bind_group,

            atlas_texture: atlas_texture.texture,
            atlas_info_buffer,
            atlas_bind_group,
        }
    }
}
