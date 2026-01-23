use synapse_core::scripting::lua::LuaContext;
use std::fs;

#[test]
fn lua_bridge_log_works() {
    let context = LuaContext::new().unwrap();
    let script = fs::read_to_string("tests/test.lua").unwrap();
    context.execute_script(&script).unwrap();
}
