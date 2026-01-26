use synapse_core::ecs::{systems, World};
use synapse_core::ecs::components::ScriptComponent;
use synapse_core::scripting::lua::LuaBackend;

#[test]
fn lua_backend_executes_script() {
    // 1. Setup
    let mut world = World::new();
    world.register_component::<ScriptComponent>();

    let backend = LuaBackend::new();
    let entity = world.create_entity();
    let script_path = "tests/test.lua"; // Path relative to the crate root

    // 2. Attach the script
    world.add_script(
        entity,
        ScriptComponent {
            script_name: script_path.to_string(),
        },
        &backend,
    );

    // 3. Run the scripting system
    systems::scripting_system(&world, &backend);

    // 4. Verification
    let lua = backend.get_lua_state();
    let result = lua.scope(|_scope| {
        let globals = lua.globals();
        let entity_table_name = format!("entity_{}", entity);
        let entity_table = globals.get::<mlua::Table>(entity_table_name.as_str())?;

        // The script's on_attach function should have been called when we called `add_script`,
        // setting _TEST_VALUE to 1.
        // The script's on_update function should have been called by the `scripting_system`,
        // incrementing _TEST_VALUE to 2.
        let test_value: i32 = entity_table.get("_TEST_VALUE")?;
        assert_eq!(test_value, 2, "Lua script did not execute or modify the test value correctly.");
        Ok(())
    });

    assert!(result.is_ok());
}
