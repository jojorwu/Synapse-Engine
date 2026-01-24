-- A simple Lua script for testing the LuaBackend.

_TEST_VALUE = 0

function on_attach(entity_id)
    _TEST_VALUE = 1
end

function on_update(entity_id)
    _TEST_VALUE = _TEST_VALUE + 1
end
