
use std::sync::Arc;
use egui_wgpu::Renderer;
use egui_winit::State;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop},
    window::Window,
};

// --- Enums and Structs ---

enum ViewState {
    ProjectManager,
    MainEditor,
}

struct EditorUI {
    current_view: ViewState,
}

impl EditorUI {
    fn new() -> Self {
        Self {
            current_view: ViewState::ProjectManager,
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        match self.current_view {
            ViewState::ProjectManager => self.draw_project_manager(ctx),
            ViewState::MainEditor => self.draw_main_editor(ctx),
        }
    }

    fn draw_project_manager(&mut self, ctx: &egui::Context) {
        egui::Window::new("Project Manager")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Synapse Engine");
                ui.separator();
                ui.label("Existing Projects:");
                ui.label("  - My Awesome Game");
                ui.label("  - Another Cool Project");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Open Project").clicked() {
                        self.current_view = ViewState::MainEditor;
                    }
                    if ui.button("New Project").clicked() {
                        println!("'New Project' was clicked");
                    }
                });
            });
    }

    fn draw_main_editor(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.label("Toolbar Placeholder");
        });
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("Entity Inspector");
            ui.label("Selected Entity Details");
        });
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.heading("Asset Browser");
            ui.label("Project Assets List");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scene View");
            ui.label("3D Scene Rendering Area");
        });
    }
}

struct AppState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    egui_ctx: egui::Context,
    egui_state: State,
    egui_renderer: Renderer,
    editor_ui: EditorUI,
}

impl AppState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(surface_caps.formats[0]);
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
        surface.configure(&device, &config);

        let egui_ctx = egui::Context::default();
        let egui_state = State::new(egui_ctx.clone(), egui::ViewportId::ROOT, &window, None, None);
        let egui_renderer = Renderer::new(&device, config.format, None, 1);
        let editor_ui = EditorUI::new();

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            egui_ctx,
            egui_state,
            egui_renderer,
            editor_ui,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let raw_input = self.egui_state.take_egui_input(&self.window);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            self.editor_ui.draw(ctx);
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);
        let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
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
            self.egui_renderer.render(&mut render_pass, &clipped_primitives, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

// --- Main Function ---

pub fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(winit::window::WindowBuilder::new()
        .with_title("Synapse Engine")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .build(&event_loop)
        .unwrap());

    let mut state = pollster::block_on(AppState::new(window.clone()));

    event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                let _ = state.egui_state.on_window_event(&state.window, event);
                match event {
                    WindowEvent::CloseRequested => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                state.window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        control_flow.exit();
                    }
                    Err(e) => eprintln!("Render error: {:?}", e),
                }
            }
            _ => {}
        }
    }).unwrap();
}
