use super::ScriptingBackend;
use mlua::{Lua, LuaOptions, StdLib};
use std::fs;

/// A scripting backend for the Lua language using the mlua library.
pub struct LuaBackend {
    lua: Lua,
}

impl LuaBackend {
    /// Creates a new `LuaBackend`.
    pub fn new() -> Self {
        let libs = StdLib::ALL_SAFE;
        let lua = Lua::new_with(libs, LuaOptions::default()).expect("Failed to create Lua state with stdlib");
        Self { lua }
    }

    /// Returns a reference to the inner Lua state.
    ///
    /// This method is primarily intended for testing and debugging purposes.
    pub fn get_lua_state(&self) -> &Lua {
        &self.lua
    }
}

impl Default for LuaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptingBackend for LuaBackend {
    fn on_attach(&self, entity_id: u64, script_name: &str) {
        let script_content = match fs::read_to_string(script_name) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error: Failed to read script '{}': {}", script_name, e);
                return;
            }
        };

        let result = self.lua.scope(|_scope| {
            let globals = self.lua.globals();
            let entity_table_name = format!("entity_{}", entity_id);
            let entity_table = self.lua.create_table()?;
            globals.set(entity_table_name.as_str(), entity_table.clone())?;

            let chunk = self.lua.load(&script_content).set_environment(entity_table.clone());

            if let Err(e) = chunk.exec() {
                 eprintln!("Error executing script for entity {}: {}", entity_id, e);
                 return Err(e);
            }

            if let Ok(on_attach_fn) = entity_table.get::<mlua::Function>("on_attach") {
                if let Err(e) = on_attach_fn.call::<()>(entity_id) {
                    eprintln!("Error executing on_attach for entity {}: {}", entity_id, e);
                    return Err(e);
                }
            }

            Ok(())
        });

        if let Err(e) = result {
            eprintln!("Error attaching script for entity {}: {}", entity_id, e);
        }
    }

    fn on_update(&self, entity_id: u64) {
        let result = self.lua.scope(|_scope| {
            let globals = self.lua.globals();
            let entity_table_name = format!("entity_{}", entity_id);

            if let Ok(entity_table) = globals.get::<mlua::Table>(entity_table_name.as_str()) {
                if let Ok(on_update_fn) = entity_table.get::<mlua::Function>("on_update") {
                    if let Err(e) = on_update_fn.call::<()>(entity_id) {
                        eprintln!("Error executing on_update for entity {}: {}", entity_id, e);
                        return Err(e);
                    }
                }
            }
            Ok(())
        });

        if let Err(e) = result {
            eprintln!("Error updating script for entity {}: {}", entity_id, e);
        }
    }
}
