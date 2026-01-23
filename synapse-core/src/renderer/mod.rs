
use crate::ecs::{ComponentId, World};
use crate::ecs::components::{ColorComponent, PositionComponent};
use glam::{Mat4, Vec3};
use std::collections::{HashMap, HashSet};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct CachedBuffers {
    pos_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    len: u32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
    buffer_cache: HashMap<Box<[ComponentId]>, CachedBuffers>,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub async fn new(width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
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
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
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
                        array_stride: std::mem::size_of::<PositionComponent>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ColorComponent>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![1 => Float32x4],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            device,
            queue,
            render_pipeline,
            width,
            height,
            buffer_cache: HashMap::new(),
            camera_buffer,
            camera_bind_group,
        }
    }

    pub fn render(&mut self, world: &World) {
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: None,
            view_formats: &[],
        };
        let texture = self.device.create_texture(&texture_desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let camera_uniform = CameraUniform {
            view_proj: (Mat4::perspective_rh_gl(
                45.0f32.to_radians(),
                self.width as f32 / self.height as f32,
                0.1,
                100.0,
            ) * Mat4::look_at_rh(
                Vec3::new(0.0, 1.0, 3.0),
                Vec3::ZERO,
                Vec3::Y,
            ))
            .to_cols_array_2d(),
        };
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let pos_id = world.get_component_id::<PositionComponent>().unwrap();
        let color_id = world.get_component_id::<ColorComponent>().unwrap();

        let active_archetypes: HashSet<Box<[ComponentId]>> = world
            .tables
            .iter()
            .filter(|a| {
                a.component_ids.contains(&pos_id) && a.component_ids.contains(&color_id)
            })
            .map(|a| a.component_ids.clone().into_boxed_slice())
            .collect();

        self.buffer_cache
            .retain(|k, _| active_archetypes.contains(k));

        for archetype in &world.tables {
            if archetype.component_ids.contains(&pos_id)
                && archetype.component_ids.contains(&color_id)
            {
                let pos_data = &archetype.columns[&pos_id];
                let color_data = &archetype.columns[&color_id];
                let key = archetype.component_ids.clone().into_boxed_slice();

                let cached = self.buffer_cache.entry(key).or_insert_with(|| {
                    let pos_buffer = create_buffer(&self.device, pos_data, "Position Vertex Buffer");
                    let color_buffer =
                        create_buffer(&self.device, color_data, "Color Vertex Buffer");
                    CachedBuffers {
                        pos_buffer,
                        color_buffer,
                        len: archetype.len as u32,
                    }
                });

                if pos_data.len() as u64 > cached.pos_buffer.size() {
                    cached.pos_buffer =
                        create_buffer(&self.device, pos_data, "Position Vertex Buffer");
                } else {
                    self.queue.write_buffer(&cached.pos_buffer, 0, pos_data);
                }

                if color_data.len() as u64 > cached.color_buffer.size() {
                    cached.color_buffer =
                        create_buffer(&self.device, color_data, "Color Vertex Buffer");
                } else {
                    self.queue.write_buffer(&cached.color_buffer, 0, color_data);
                }
                cached.len = archetype.len as u32;
            }
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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            for cached in self.buffer_cache.values() {
                render_pass.set_vertex_buffer(0, cached.pos_buffer.slice(..));
                render_pass.set_vertex_buffer(1, cached.color_buffer.slice(..));
                render_pass.draw(0..cached.len, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

fn create_buffer(device: &wgpu::Device, data: &[u8], label: &str) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    })
}
