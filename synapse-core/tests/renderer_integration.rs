
use synapse_core::ecs::World;
use synapse_core::ecs::components::{ColorComponent, PositionComponent};
use synapse_core::renderer::Renderer;
use std::env;

#[test]
fn renderer_initializes_and_renders_without_panic_offscreen() {
    env::set_var("XDG_RUNTIME_DIR", "/tmp");
    let mut world = World::new();

    world.register_component::<PositionComponent>();
    world.register_component::<ColorComponent>();

    let entity1 = world.create_entity();
    world.add_component(entity1, PositionComponent { x: -0.5, y: -0.5, z: 0.0 });
    world.add_component(entity1, ColorComponent { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });

    let entity2 = world.create_entity();
    world.add_component(entity2, PositionComponent { x: 0.5, y: -0.5, z: 0.0 });
    world.add_component(entity2, ColorComponent { r: 0.0, g: 1.0, b: 0.0, a: 1.0 });

    let entity3 = world.create_entity();
    world.add_component(entity3, PositionComponent { x: 0.0, y: 0.5, z: 0.0 });
    world.add_component(entity3, ColorComponent { r: 0.0, g: 0.0, b: 1.0, a: 1.0 });

    let mut renderer = pollster::block_on(Renderer::new(1024, 768));

    renderer.render(&world);
}
