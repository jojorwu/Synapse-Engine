
pub mod culling;

use crate::ecs::World;
use crate::ecs::components::{ColorComponent, RectangleComponent, TransformComponent};
use crate::renderer::culling::Frustum;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use egui;
use egui_wgpu;
use egui_winit;
use std::sync::Arc;
use winit::window::Window;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("Failed to create wgpu surface")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("No suitable adapter found")]
    NoAdapter,
    #[error("Failed to get device")]
    GetDevice(#[from] wgpu::RequestDeviceError),
}

pub struct RendererSettings {
    pub light_position: [f32; 3],
    pub light_color: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    _padding: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    _padding: u32,
    color: [f32; 3],
    _padding2: u32,
    light_view_proj: [[f32; 4]; 4],
}

struct DepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

pub struct Renderer<'a> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    depth_texture: DepthTexture,
    shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    shadow_sampler: wgpu::Sampler,
    mesh: Mesh,
    instance_buffer: wgpu::Buffer,
    instances: Vec<InstanceRaw>,
    render_pipeline_2d: wgpu::RenderPipeline,
    camera_buffer_2d: wgpu::Buffer,
    camera_bind_group_2d: wgpu::BindGroup,
    vertex_buffer_2d: wgpu::Buffer,
    color_buffer_2d: wgpu::Buffer,
    num_vertices_2d: u32,
    egui_context: egui::Context,
    egui_winit_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    pub settings: RendererSettings,
}

struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

impl<'a> Renderer<'a> {
    fn create_mesh(device: &wgpu::Device) -> Mesh {
        let vertices = &[
            // Front
            Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0] },
            Vertex { position: [0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0] },
            Vertex { position: [0.5, 0.5, 0.5], normal: [0.0, 0.0, 1.0] },
            Vertex { position: [-0.5, 0.5, 0.5], normal: [0.0, 0.0, 1.0] },
            // Back
            Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0] },
            Vertex { position: [0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0] },
            Vertex { position: [0.5, 0.5, -0.5], normal: [0.0, 0.0, -1.0] },
            Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 0.0, -1.0] },
            // Right
            Vertex { position: [0.5, -0.5, 0.5], normal: [1.0, 0.0, 0.0] },
            Vertex { position: [0.5, -0.5, -0.5], normal: [1.0, 0.0, 0.0] },
            Vertex { position: [0.5, 0.5, -0.5], normal: [1.0, 0.0, 0.0] },
            Vertex { position: [0.5, 0.5, 0.5], normal: [1.0, 0.0, 0.0] },
            // Left
            Vertex { position: [-0.5, -0.5, 0.5], normal: [-1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, -0.5], normal: [-1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, 0.5, -0.5], normal: [-1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, 0.5, 0.5], normal: [-1.0, 0.0, 0.0] },
            // Top
            Vertex { position: [-0.5, 0.5, 0.5], normal: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, 0.5, 0.5], normal: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0] },
            Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0] },
            // Bottom
            Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, -1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.5], normal: [0.0, -1.0, 0.0] },
            Vertex { position: [0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0] },
            Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0] },
        ];
        let indices: &[u16] = &[
            0, 1, 2, 2, 3, 0,
            4, 5, 6, 6, 7, 4,
            8, 9, 10, 10, 11, 8,
            12, 13, 14, 14, 15, 12,
            16, 17, 18, 18, 19, 16,
            20, 21, 22, 22, 23, 20,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = indices.len() as u32;

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices,
        }
    }

    fn init_buffers_and_bind_groups(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout_2d: &wgpu::BindGroupLayout,
        shadow_view: &wgpu::TextureView,
        shadow_sampler: &wgpu::Sampler,
    ) -> (
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
    ) {
        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            position: [0.0; 3],
            _padding: 0,
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
            light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
            label: Some("light_bind_group"),
        });

        let camera_uniform_2d = CameraUniform {
            view_proj: Mat4::orthographic_rh_gl(0.0, config.width as f32, config.height as f32, 0.0, -1.0, 1.0)
                .to_cols_array_2d(),
            position: [0.0; 3],
            _padding: 0,
        };
        let camera_buffer_2d = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer 2D"),
            contents: bytemuck::cast_slice(&[camera_uniform_2d]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_2d = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: camera_bind_group_layout_2d,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer_2d.as_entire_binding(),
            }],
            label: Some("camera_bind_group_2d"),
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vertex_buffer_2d = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer 2D"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_buffer_2d = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Color Buffer 2D"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        (
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
            camera_buffer_2d,
            camera_bind_group_2d,
            instance_buffer,
            vertex_buffer_2d,
            color_buffer_2d,
        )
    }

    fn init_textures(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> (DepthTexture, wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let depth_texture = DepthTexture::new(device, config.width, config.height);

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Texture"),
            size: wgpu::Extent3d {
                width: 2048,
                height: 2048,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        (depth_texture, shadow_texture, shadow_view, shadow_sampler)
    }

    fn init_pipelines(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
        shadow_pipeline_layout: &wgpu::PipelineLayout,
        camera_bind_group_layout_2d: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[camera_bind_group_layout, light_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let depth_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Depth Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("depth_only.wgsl").into()),
        });

        let depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Depth Pipeline"),
            layout: Some(&shadow_pipeline_layout), // Reuse the same layout as shadow
            vertex: wgpu::VertexState {
                module: &depth_shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4],
                    },
                ],
            },
            fragment: None, // No fragment shader
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &depth_shader, // Reuse the depth shader
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4],
                    },
                ],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // Cull front faces to prevent peter-panning
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let shader_2d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader 2D"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_2d.wgsl").into()),
        });

        let render_pipeline_layout_2d =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout 2D"),
                bind_group_layouts: &[camera_bind_group_layout_2d],
                push_constant_ranges: &[],
            });

        let render_pipeline_2d =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline 2D"),
                layout: Some(&render_pipeline_layout_2d),
                vertex: wgpu::VertexState {
                    module: &shader_2d,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![1 => Float32x4],
                    },
                ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_2d,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        (
            render_pipeline,
            depth_pipeline,
            shadow_pipeline,
            render_pipeline_2d,
        )
    }

    pub async fn new(window: &'a Window) -> Result<Self, RendererError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RendererError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await?;
        let device = Arc::new(device);

        let caps = surface.get_capabilities(&adapter);
        let texture_format = caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
                label: Some("light_bind_group_layout"),
            });

        let (depth_texture, shadow_texture, shadow_view, shadow_sampler) = Self::init_textures(&device, &config);

        let camera_bind_group_layout_2d =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout_2d"),
            });
        let (
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
            camera_buffer_2d,
            camera_bind_group_2d,
            instance_buffer,
            vertex_buffer_2d,
            color_buffer_2d,
        ) = Self::init_buffers_and_bind_groups(
            &device,
            &config,
            &camera_bind_group_layout,
            &light_bind_group_layout,
            &camera_bind_group_layout_2d,
            &shadow_view,
            &shadow_sampler,
        );

        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let (
            render_pipeline,
            depth_pipeline,
            shadow_pipeline,
            render_pipeline_2d,
        ) = Self::init_pipelines(
            &device,
            &config,
            &camera_bind_group_layout,
            &light_bind_group_layout,
            &shadow_pipeline_layout,
            &camera_bind_group_layout_2d,
        );

        let mesh = Self::create_mesh(&device);

        let egui_context = egui::Context::default();
        let egui_winit_state = egui_winit::State::new(egui_context.clone(), egui::ViewportId::ROOT, &window, None, None);
        let egui_renderer = egui_wgpu::Renderer::new(&*device, config.format, None, 1);

        let settings = RendererSettings {
            light_position: [2.0, 2.0, 2.0],
            light_color: [1.0, 1.0, 1.0],
        };

        Ok(Self {
            instance,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            depth_pipeline,
            shadow_pipeline,
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
            depth_texture,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            mesh,
            instance_buffer,
            instances: Vec::new(),
            render_pipeline_2d,
            camera_buffer_2d,
            camera_bind_group_2d,
            vertex_buffer_2d,
            color_buffer_2d,
            num_vertices_2d: 0,
            egui_context,
            egui_winit_state,
            egui_renderer,
            settings,
        })
    }

    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        self.egui_winit_state.on_window_event(window, event).consumed
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = DepthTexture::new(&self.device, self.config.width, self.config.height);
        }
    }

    pub fn render(&mut self, window: &Window, world: &World) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let camera_position = Vec3::new(0.0, 1.0, 3.0);
        let view_proj_matrix = Mat4::perspective_rh_gl(
            45.0f32.to_radians(),
            self.config.width as f32 / self.config.height as f32,
            0.1,
            100.0,
        ) * Mat4::look_at_rh(
            camera_position,
            Vec3::ZERO,
            Vec3::Y,
        );
        let camera_uniform = CameraUniform {
            view_proj: view_proj_matrix.to_cols_array_2d(),
            position: camera_position.into(),
            _padding: 0,
        };
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let light_proj = Mat4::orthographic_rh_gl(-10.0, 10.0, -10.0, 10.0, 1.0, 20.0);
        let light_view = Mat4::look_at_rh(
            self.settings.light_position.into(),
            Vec3::ZERO,
            Vec3::Y,
        );
        let light_view_proj = light_proj * light_view;

        let light_uniform = LightUniform {
            position: self.settings.light_position,
            _padding: 0,
            color: self.settings.light_color,
            _padding2: 0,
            light_view_proj: light_view_proj.to_cols_array_2d(),
        };
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );

        let frustum = Frustum::from_matrix(&view_proj_matrix);
        let transform_id = world.get_component_id::<TransformComponent>().unwrap();
        let color_id = world.get_component_id::<ColorComponent>().unwrap();

        self.instances.clear();
        for archetype in &world.tables {
            if archetype.component_ids.contains(&transform_id)
                && archetype.component_ids.contains(&color_id)
            {
                // UNSAFE: This is a direct memory access to the component data.
                // It is safe under the following conditions, enforced by the ECS:
                // 1. The archetype contains the `TransformComponent` and `ColorComponent`.
                // 2. The `archetype.len` accurately reflects the number of entities.
                // 3. The underlying `Vec<u8>` in the column is tightly packed and
                //    correctly aligned for the component type.
                let transforms = unsafe {
                    std::slice::from_raw_parts(
                        archetype.columns[&transform_id].as_ptr() as *const TransformComponent,
                        archetype.len,
                    )
                };
                let colors = unsafe {
                    std::slice::from_raw_parts(
                        archetype.columns[&color_id].as_ptr() as *const ColorComponent,
                        archetype.len,
                    )
                };

                for (i, transform) in transforms.iter().enumerate() {
                    let position = Vec3::from(transform.position);
                    let scale = Vec3::from(transform.scale);
                    let radius = scale.x.max(scale.y).max(scale.z);

                    if frustum.is_sphere_visible(position, radius) {
                        self.instances.push(InstanceRaw {
                            model: transform.to_matrix().to_cols_array_2d(),
                            color: [colors[i].r, colors[i].g, colors[i].b, colors[i].a],
                        });
                    }
                }
            }
        }

        let instance_data = bytemuck::cast_slice(&self.instances);
        if instance_data.len() as u64 > self.instance_buffer.size() {
            self.instance_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: instance_data,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
        } else {
            self.queue
                .write_buffer(&self.instance_buffer, 0, instance_data);
        }

        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let light_view_uniform = CameraUniform {
                view_proj: light_view_proj.to_cols_array_2d(),
                position: self.settings.light_position,
                _padding: 0,
            };
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[light_view_uniform]),
            );

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            shadow_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            shadow_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            shadow_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            shadow_pass.draw_indexed(0..self.mesh.num_indices, 0, 0..self.instances.len() as u32);
        }

        {
            let mut depth_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth Pre-Pass"),
                color_attachments: &[], // No color attachments
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            depth_pass.set_pipeline(&self.depth_pipeline);
            depth_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            depth_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            depth_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            depth_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            depth_pass.draw_indexed(0..self.mesh.num_indices, 0, 0..self.instances.len() as u32);
        }
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.mesh.num_indices, 0, 0..self.instances.len() as u32);
        }

        let rect_id = world.get_component_id::<RectangleComponent>().unwrap();
        let color_id = world.get_component_id::<ColorComponent>().unwrap();

        let mut vertices_2d = Vec::new();
        let mut colors_2d = Vec::new();
        for archetype in &world.tables {
            if archetype.component_ids.contains(&rect_id)
                && archetype.component_ids.contains(&color_id)
            {
                // UNSAFE: This is a direct memory access to the component data.
                // It is safe under the following conditions, enforced by the ECS:
                // 1. The archetype contains the `RectangleComponent` and `ColorComponent`.
                // 2. The `archetype.len` accurately reflects the number of entities.
                // 3. The underlying `Vec<u8>` in the column is tightly packed and
                //    correctly aligned for the component type.
                let rects = unsafe {
                    std::slice::from_raw_parts(
                        archetype.columns[&rect_id].as_ptr() as *const RectangleComponent,
                        archetype.len,
                    )
                };
                let colors = unsafe {
                    std::slice::from_raw_parts(
                        archetype.columns[&color_id].as_ptr() as *const ColorComponent,
                        archetype.len,
                    )
                };

                for (i, rect) in rects.iter().enumerate() {
                    let x = rect.x;
                    let y = rect.y;
                    let w = rect.width;
                    let h = rect.height;
                    let color = [colors[i].r, colors[i].g, colors[i].b, colors[i].a];
                    vertices_2d.extend_from_slice(&[
                        [x, y],
                        [x + w, y],
                        [x, y + h],
                        [x, y + h],
                        [x + w, y],
                        [x + w, y + h],
                    ]);
                    colors_2d.extend_from_slice(&[color, color, color, color, color, color]);
                }
            }
        }

        self.num_vertices_2d = vertices_2d.len() as u32;
        let vertex_data = bytemuck::cast_slice(&vertices_2d);
        if vertex_data.len() as u64 > self.vertex_buffer_2d.size() {
            self.vertex_buffer_2d =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer 2D"),
                        contents: vertex_data,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
        } else {
            self.queue
                .write_buffer(&self.vertex_buffer_2d, 0, vertex_data);
        }

        let color_data = bytemuck::cast_slice(&colors_2d);
        if color_data.len() as u64 > self.color_buffer_2d.size() {
            self.color_buffer_2d =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Color Buffer 2D"),
                        contents: color_data,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
        } else {
            self.queue
                .write_buffer(&self.color_buffer_2d, 0, color_data);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass 2D"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline_2d);
            render_pass.set_bind_group(0, &self.camera_bind_group_2d, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer_2d.slice(..));
            render_pass.set_vertex_buffer(1, self.color_buffer_2d.slice(..));
            render_pass.draw(0..self.num_vertices_2d, 0..1);
        }

        let raw_input = self.egui_winit_state.take_egui_input(window);
        let full_output = self.egui_context.run(raw_input, |ctx| {
            egui::Window::new("Dev Panel").show(ctx, |ui| {
                ui.label(format!("Entity count: {}", world.entity_map.len()));
            });

            egui::Window::new("Render Settings").show(ctx, |ui| {
                ui.label("Light Position");
                ui.add(egui::Slider::new(&mut self.settings.light_position[0], -10.0..=10.0).text("X"));
                ui.add(egui::Slider::new(&mut self.settings.light_position[1], -10.0..=10.0).text("Y"));
                ui.add(egui::Slider::new(&mut self.settings.light_position[2], -10.0..=10.0).text("Z"));

                ui.label("Light Color");
                ui.color_edit_button_rgb(&mut self.settings.light_color);
            });
        });

        self.egui_winit_state.handle_platform_output(window, full_output.platform_output);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let clipped_primitives = self.egui_context.tessellate(full_output.shapes, screen_descriptor.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &clipped_primitives, &screen_descriptor);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.egui_renderer.render(&mut render_pass, &clipped_primitives, &screen_descriptor);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();

        Ok(())
    }
}

impl DepthTexture {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}
