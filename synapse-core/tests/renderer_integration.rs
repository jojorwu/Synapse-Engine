
use glam::Quat;
use synapse_core::ecs::components::{ColorComponent, RectangleComponent, TransformComponent};
use synapse_core::ecs::World;
use synapse_core::renderer::Renderer;
use std::env;
use winit::{
    event_loop::EventLoopBuilder,
    window::WindowBuilder,
};
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

#[test]
fn renderer_initializes_and_renders_to_window_without_panic() {
    env::set_var("XDG_RUNTIME_DIR", "/tmp");
    let mut builder = EventLoopBuilder::new();
    #[cfg(target_os = "linux")]
    builder.with_any_thread(true);
    let event_loop = builder.build().unwrap();
    let window = WindowBuilder::new().with_visible(false).build(&event_loop).unwrap();
    let mut world = World::new();

    world.register_component::<TransformComponent>();
    world.register_component::<ColorComponent>();
    world.register_component::<RectangleComponent>();

    let entity1 = world.create_entity();
    world.add_component(
        entity1,
        TransformComponent {
            position: [-1.5, 0.0, 0.0],
            rotation: Quat::IDENTITY.to_array(),
            scale: [1.0, 1.0, 1.0],
        },
    );
    world.add_component(entity1, ColorComponent { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });

    let entity2 = world.create_entity();
    world.add_component(
        entity2,
        TransformComponent {
            position: [1.5, 0.0, 0.0],
            rotation: Quat::IDENTITY.to_array(),
            scale: [1.0, 1.0, 1.0],
        },
    );
    world.add_component(entity2, ColorComponent { r: 0.0, g: 1.0, b: 0.0, a: 1.0 });

    let entity3 = world.create_entity();
    world.add_component(
        entity3,
        TransformComponent {
            position: [0.0, 1.5, 0.0],
            rotation: Quat::from_rotation_z(45.0f32.to_radians()).to_array(),
            scale: [0.5, 0.5, 0.5],
        },
    );
    world.add_component(entity3, ColorComponent { r: 0.0, g: 0.0, b: 1.0, a: 1.0 });

    let entity4 = world.create_entity();
    world.add_component(
        entity4,
        RectangleComponent {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 100.0,
        },
    );
    world.add_component(entity4, ColorComponent { r: 1.0, g: 1.0, b: 0.0, a: 0.5 });

    let mut renderer = pollster::block_on(Renderer::new(&window)).unwrap();

    match renderer.render(&window, &world) {
        Ok(_) => {}
        Err(wgpu::SurfaceError::Lost) => {
            // Reconfigure the surface if it's lost.
            // In a real app, you'd handle this more gracefully.
            let size = window.inner_size();
            renderer.resize(size);
        }
        Err(wgpu::SurfaceError::OutOfMemory) => {
            // Handle out-of-memory error.
            panic!("GPU out of memory!");
        }
        Err(e) => {
            // Other errors.
            eprintln!("Error rendering frame: {:?}", e);
        }
    }
}
