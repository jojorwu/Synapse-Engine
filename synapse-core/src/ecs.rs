use std::collections::HashMap;
use bitvec::prelude::BitVec;

// Using type aliases for clarity
pub type EntityId = u32;
pub type ComponentId = u32;
pub type ArchetypeId = usize;
pub type RowIndex = usize;

/// Metadata for a component type, generated at compile-time.
pub struct ComponentMeta {
    pub id: ComponentId,
    pub size: usize,
    pub is_sync: bool,
}

/// A table storing entities of a single Archetype.
pub struct ArchetypeTable {
    pub entity_ids: Vec<EntityId>,
    // Columns storing the actual component data as raw bytes.
    pub columns: HashMap<ComponentId, Box<[u8]>>,
    // Bitmask for tracking changed ("dirty") data for network sync.
    pub dirty_masks: HashMap<ComponentId, BitVec>,
    pub capacity: usize,
    pub len: usize,
}

/// A struct to locate an entity's data within the ECS.
pub struct EntityLocation {
    pub archetype_id: ArchetypeId,
    pub row: RowIndex,
}

/// The main world container that holds all ECS data.
pub struct World {
    pub tables: Vec<ArchetypeTable>,
    // A map to quickly find where an entity is stored.
    pub entity_map: HashMap<EntityId, EntityLocation>,
}
