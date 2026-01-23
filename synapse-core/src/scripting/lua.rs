use mlua::{Lua, Result};

pub struct LuaContext {
    lua: Lua,
}

impl LuaContext {
    pub fn new() -> Result<Self> {
        let lua = Lua::new();

        lua.globals().set(
            "log",
            lua.create_function(|_, message: String| {
                println!("[LUA] {}", message);
                Ok(())
            })?,
        )?;

        Ok(Self { lua })
    }

    pub fn execute_script(&self, script: &str) -> Result<()> {
        self.lua.load(script).exec()
    }
}
