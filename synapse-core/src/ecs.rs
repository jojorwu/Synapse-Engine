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

impl ArchetypeTable {
    /// Creates a new, empty archetype table.
    fn new() -> Self {
        Self {
            entity_ids: Vec::new(),
            columns: HashMap::new(),
            dirty_masks: HashMap::new(),
            capacity: 0,
            len: 0,
        }
    }
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
    next_entity_id: EntityId,
}

impl World {
    /// Creates a new, empty World.
    pub fn new() -> Self {
        let mut tables = Vec::new();
        // Create the "empty" archetype for entities with no components.
        tables.push(ArchetypeTable::new());

        Self {
            tables,
            entity_map: HashMap::new(),
            next_entity_id: 0,
        }
    }

    /// Creates a new entity with no components.
    pub fn create_entity(&mut self) -> EntityId {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;

        // All entities start in the "empty" archetype (index 0).
        let empty_archetype = &mut self.tables[0];
        let row = empty_archetype.len;

        empty_archetype.entity_ids.push(entity_id);
        empty_archetype.len += 1;

        self.entity_map.insert(entity_id, EntityLocation {
            archetype_id: 0,
            row,
        });

        entity_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_entity_works() {
        let mut world = World::new();

        let entity1 = world.create_entity();
        let entity2 = world.create_entity();

        assert_eq!(entity1, 0);
        assert_eq!(entity2, 1);
        assert_eq!(world.entity_map.len(), 2);

        let loc1 = world.entity_map.get(&entity1).unwrap();
        assert_eq!(loc1.archetype_id, 0);
        assert_eq!(loc1.row, 0);

        let loc2 = world.entity_map.get(&entity2).unwrap();
        assert_eq!(loc2.archetype_id, 0);
        assert_eq!(loc2.row, 1);

        let empty_archetype = &world.tables[0];
        assert_eq!(empty_archetype.len, 2);
        assert_eq!(empty_archetype.entity_ids[0], entity1);
        assert_eq!(empty_archetype.entity_ids[1], entity2);
    }
}
