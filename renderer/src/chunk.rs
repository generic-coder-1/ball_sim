use core::panic;

use bytemuck::{bytes_of, cast_slice};
use egui_wgpu_backend::wgpu::{
    self, util::DeviceExt, BindGroup, BindGroupEntry, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferUsages, ColorWrites, PipelineCompilationOptions, PrimitiveState, RenderPass,
    RenderPipeline, ShaderStages, SurfaceConfiguration, TextureDescriptor, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

use crate::{texture::Texture, vertex::Vertex};

pub struct ChunkRenderingData {
    pipeline: RenderPipeline,

    //group 0
    instance_array_buffer: wgpu::Buffer,
    instance_data: wgpu::Texture,
    instance_array_size: u32,
    instance_array_bind_group: wgpu::BindGroup,

    //group 1
    atlas_bind_group: wgpu::BindGroup,
    //group 2 will be provided for us

    //quad
    vertex_buffer: wgpu::Buffer,
}

pub const CHUNK_SIZE: usize = 32;
const MAX_CHUNKS: usize = 256;

#[repr(C, align(4))]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq, Eq, Hash, Default)]
pub struct ChunkPosition {
    pub position: [i32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Chunk {
    pub data: [u8; CHUNK_SIZE * CHUNK_SIZE],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            data: [0; CHUNK_SIZE * CHUNK_SIZE],
        }
    }
}

impl Chunk {
    pub fn set_tile(&mut self, pos: [u32; 2], tile: u8) {
        self.data[(pos[0] + (CHUNK_SIZE as u32 - pos[1] - 1) * CHUNK_SIZE as u32) as usize] = tile;
    }

    pub fn get_tile(&self, pos: [u32; 2]) -> u8 {
        self.data[(pos[0] + (CHUNK_SIZE as u32 - pos[1] - 1) * CHUNK_SIZE as u32) as usize]
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct AtlasInfo {
    pub tiles_per_row: u32,
    pub _pad: u32,
    pub tiles_size: [u32; 2],
}

impl ChunkRenderingData {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        atlas_texture: Texture,
        atlas_info: &AtlasInfo,
    ) -> Self {
        let instance_array: Vec<ChunkPosition> =
            vec![ChunkPosition { position: [0; 2] }; MAX_CHUNKS];
        let chunks = vec![
            Chunk {
                data: [0; CHUNK_SIZE * CHUNK_SIZE],
            };
            MAX_CHUNKS
        ];
        let instance_data = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: Some("Chunk data"),
                size: wgpu::Extent3d {
                    width: CHUNK_SIZE as u32,
                    height: CHUNK_SIZE as u32,
                    depth_or_array_layers: MAX_CHUNKS as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Uint,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[TextureFormat::R8Uint],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &bytemuck::cast_vec(chunks),
        );

        let instance_array_size = 0;
        let instance_array_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance_array_buffer"),
            contents: cast_slice(&instance_array),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let instance_array_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("instance_array_data_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let instance_array_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instance_array_bind_group"),
            layout: &instance_array_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instance_array_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&instance_data.create_view(
                        &TextureViewDescriptor {
                            label: Some("chunk data view"),
                            format: Some(TextureFormat::R8Uint),
                            dimension: Some(wgpu::TextureViewDimension::D2Array),
                            aspect: wgpu::TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: 0,
                            array_layer_count: None,
                            usage: None,
                        },
                    )),
                },
            ],
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
            instance_array_buffer,
            instance_data,
            instance_array_size,
            instance_array_bind_group,

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

    pub fn update_chunks(
        &mut self,
        queue: &wgpu::Queue,
        pos: Vec<ChunkPosition>,
        data: Vec<Chunk>,
    ) {
        if pos.len() != data.len() {
            panic!("sizes of data is incorrect");
        }
        if data.len() > MAX_CHUNKS {
            panic!("drawing too many chunks");
        }
        queue.write_buffer(
            &self.instance_array_buffer,
            0,
            bytemuck::cast_slice(pos.as_slice()),
        );
        let ext = wgpu::Extent3d {
            width: CHUNK_SIZE as u32,
            height: CHUNK_SIZE as u32,
            depth_or_array_layers: data.len() as u32,
        };
        self.instance_array_size = data.len() as u32;
        queue.write_texture(
            self.instance_data.as_image_copy(),
            bytemuck::cast_slice(data.as_slice()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(CHUNK_SIZE as u32),
                rows_per_image: Some(CHUNK_SIZE as u32),
            },
            ext,
        );
    }
}
