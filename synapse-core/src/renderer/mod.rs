
use crate::ecs::World;
use crate::ecs::components::{ColorComponent, PositionComponent};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

// --- Vertex ---

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// --- Renderer ---

pub struct Renderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_capacity: u64,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer_capacity = 256;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: vertex_buffer_capacity * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            vertex_buffer_capacity,
        }
    }

    pub fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>, world: &World, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut vertices = Vec::new();
        for archetype in &world.tables {
            if archetype.component_ids.contains(&world.get_component_id::<PositionComponent>().unwrap())
                && archetype.component_ids.contains(&world.get_component_id::<ColorComponent>().unwrap())
            {
                let pos_col = archetype.columns.get(&world.get_component_id::<PositionComponent>().unwrap()).unwrap();
                let color_col = archetype.columns.get(&world.get_component_id::<ColorComponent>().unwrap()).unwrap();

                for i in 0..archetype.len {
                    let pos: &PositionComponent = bytemuck::from_bytes(&pos_col[i * std::mem::size_of::<PositionComponent>()..]);
                    let color: &ColorComponent = bytemuck::from_bytes(&color_col[i * std::mem::size_of::<ColorComponent>()..]);

                    vertices.push(Vertex {
                        position: [pos.x, pos.y, pos.z],
                        color: [color.r, color.g, color.b],
                    });
                }
            }
        }

        let num_vertices = vertices.len() as u64;
        if num_vertices > self.vertex_buffer_capacity {
            self.vertex_buffer_capacity = num_vertices * 2;
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: self.vertex_buffer_capacity * std::mem::size_of::<Vertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..num_vertices as u32, 0..1);
    }
}

// --- RendererContext ---

pub struct RendererContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    renderer: Renderer,
    scene_texture: wgpu::Texture,
    pub scene_texture_id: egui::TextureId,
}

impl RendererContext {
    pub fn new(
        _window: Arc<Window>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        size: winit::dpi::PhysicalSize<u32>,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) -> Self {
        let renderer = Renderer::new(&device, &config);

        let scene_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Texture"),
            size: wgpu::Extent3d {
                width: 800,
                height: 600,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let scene_texture_id = egui_renderer.register_native_texture(
            &device,
            &scene_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::FilterMode::Linear,
        );

        Self {
            surface,
            device,
            queue,
            config,
            size,
            renderer,
            scene_texture,
            scene_texture_id,
        }
    }

    pub fn render(&mut self, world: &World) {
        let view = self
            .scene_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer.render(&mut render_pass, world, &self.device, &self.queue);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn resize_scene_texture(&mut self, new_size: egui::Vec2, egui_renderer: &mut egui_wgpu::Renderer) {
        self.scene_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Texture"),
            size: wgpu::Extent3d {
                width: new_size.x as u32,
                height: new_size.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        egui_renderer.update_egui_texture_from_wgpu_texture(
            &self.device,
            &self.scene_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::FilterMode::Linear,
            self.scene_texture_id,
        );
    }
}
