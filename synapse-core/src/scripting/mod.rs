pub mod lua;

/// A trait for scripting language backends.
pub trait ScriptingBackend {
    /// Called when a script is attached to an entity.
    fn on_attach(&self, entity_id: u64, script_name: &str);

    /// Called every frame for an entity with a script.
    fn on_update(&self, entity_id: u64);
}

/// A placeholder for the C# scripting backend.
pub struct PlaceholderCSharpBackend;

impl ScriptingBackend for PlaceholderCSharpBackend {
    fn on_attach(&self, entity_id: u64, script_name: &str) {
        println!(
            "[PlaceholderCSharpBackend] Attaching script '{}' to entity {}",
            script_name, entity_id
        );
    }

    fn on_update(&self, entity_id: u64) {
        // In a real implementation, this would call the update method in the C# script.
        println!("[PlaceholderCSharpBackend] Updating entity {}", entity_id);
    }
}
