use crate::ecs::World;
use crate::ecs::components::ScriptComponent;
use crate::scripting::ScriptingBackend;

// TODO: This system should be called every frame from the main application loop.
pub fn scripting_system<T: ScriptingBackend>(world: &World, backend: &T) {
    // Get the component ID for ScriptComponent.
    let script_component_id = match world.get_component_id::<ScriptComponent>() {
        Some(id) => id,
        // If the component isn't registered, there's nothing to do.
        None => return,
    };

    // Iterate over all archetypes (stored in `tables`).
    for archetype in world.tables.iter() {
        // Check if the current archetype has the ScriptComponent by looking through its component_ids.
        if archetype.component_ids.contains(&script_component_id) {
            // If it does, iterate over all entities in this archetype.
            for &entity_id in &archetype.entity_ids {
                // And call the backend's update function for each one, casting the u32 EntityId to u64.
                backend.on_update(entity_id as u64);
            }
        }
    }
}
