use std::any::TypeId;
use std::collections::HashMap;
use std::mem;
use bitvec::prelude::BitVec;

// Using type aliases for clarity
pub type EntityId = u32;
pub type ComponentId = u32;
pub type ArchetypeId = usize;
pub type RowIndex = usize;

/// A marker trait for all component types.
/// Must be 'static to be stored safely in the World.
pub trait Component: 'static {}

/// Metadata for a component type, generated at compile-time.
pub struct ComponentMeta {
    pub id: ComponentId,
    pub size: usize,
    pub is_sync: bool,
}

/// A table storing entities of a single Archetype.
pub struct ArchetypeTable {
    pub entity_ids: Vec<EntityId>,
    // The set of component IDs that define this archetype.
    pub component_ids: Vec<ComponentId>,
    // Columns storing the actual component data as raw bytes.
    pub columns: HashMap<ComponentId, Box<[u8]>>,
    // Bitmask for tracking changed ("dirty") data for network sync.
    pub dirty_masks: HashMap<ComponentId, BitVec>,
    pub capacity: usize,
    pub len: usize,
}

impl ArchetypeTable {
    /// Creates a new, empty archetype table.
    fn new(component_ids: Vec<ComponentId>) -> Self {
        Self {
            entity_ids: Vec::new(),
            component_ids,
            columns: HashMap::new(),
            dirty_masks: HashMap::new(),
            capacity: 0,
            len: 0,
        }
    }
}

/// A struct to locate an entity's data within the ECS.
#[derive(Clone, Copy)]
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
    component_meta: HashMap<TypeId, ComponentMeta>,
    next_component_id: ComponentId,
    // A cache to speed up finding the destination archetype when adding a component.
    // Key: current archetype_id, Value: map of component_id -> destination archetype_id
    archetype_graph: HashMap<ArchetypeId, HashMap<ComponentId, ArchetypeId>>,
}

impl World {
    /// Creates a new, empty World.
    pub fn new() -> Self {
        let mut tables = Vec::new();
        // Create the "empty" archetype for entities with no components.
        tables.push(ArchetypeTable::new(Vec::new()));

        Self {
            tables,
            archetype_graph: HashMap::new(),
            entity_map: HashMap::new(),
            next_entity_id: 0,
            component_meta: HashMap::new(),
            next_component_id: 0,
        }
    }

    /// Registers a new component type with the World.
    pub fn register_component<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        if self.component_meta.contains_key(&type_id) {
            // This component type is already registered.
            return;
        }

        let meta = ComponentMeta {
            id: self.next_component_id,
            size: mem::size_of::<T>(),
            is_sync: false, // For now, we assume false. This will be updated later.
        };

        self.component_meta.insert(type_id, meta);
        self.next_component_id += 1;
    }

    /// Finds an archetype that matches the given component set, or creates it if it doesn't exist.
    fn find_or_create_archetype(&mut self, current_archetype_id: ArchetypeId, component_to_add: ComponentId) -> ArchetypeId {
        if let Some(dest_archetype_id) = self.archetype_graph
            .get(&current_archetype_id)
            .and_then(|transitions| transitions.get(&component_to_add))
        {
            // We have a cached transition, so we can return the destination archetype ID immediately.
            return *dest_archetype_id;
        }

        // The transition is not cached, so we need to figure out the destination archetype.
        let current_archetype = &self.tables[current_archetype_id];
        let mut new_component_ids = current_archetype.component_ids.clone();
        new_component_ids.push(component_to_add);
        new_component_ids.sort_unstable();

        // Check if an archetype with this exact component set already exists.
        if let Some(_existing_archetype) = self.tables.iter().find(|a| a.component_ids == new_component_ids) {
            // Found an existing archetype. Cache the transition.
            let dest_archetype_id = self.tables.iter().position(|a| a.component_ids == new_component_ids).unwrap();
            self.archetype_graph
                .entry(current_archetype_id)
                .or_default()
                .insert(component_to_add, dest_archetype_id);
            return dest_archetype_id;
        }

        // No archetype with this component set exists, so we create a new one.
        let new_archetype_id = self.tables.len();
        let new_archetype = ArchetypeTable::new(new_component_ids);
        self.tables.push(new_archetype);

        // Cache the transition for future use.
        self.archetype_graph
            .entry(current_archetype_id)
            .or_default()
            .insert(component_to_add, new_archetype_id);

        new_archetype_id
    }

    // A simplified placeholder for moving component data.
    // A real implementation would be more optimized and handle memory allocation better.
    fn move_entity_data(
        &mut self,
        entity_id: EntityId,
        src_archetype_id: ArchetypeId,
        src_row: usize,
        dest_archetype_id: ArchetypeId,
    ) {
        // This logic is complex and will be implemented properly later.
        // For now, we simulate the move by just updating the map.

        let (source_archetype, dest_archetype) = if src_archetype_id < dest_archetype_id {
            let (left, right) = self.tables.split_at_mut(dest_archetype_id);
            (&mut left[src_archetype_id], &mut right[0])
        } else {
            let (left, right) = self.tables.split_at_mut(src_archetype_id);
            (&mut right[0], &mut left[dest_archetype_id])
        };

        // Simplified logic: remove from source, add to destination
        source_archetype.entity_ids.swap_remove(src_row);

        if src_row < source_archetype.entity_ids.len() {
            let moved_entity_id = source_archetype.entity_ids[src_row];
            self.entity_map.get_mut(&moved_entity_id).unwrap().row = src_row;
        }

        let dest_row = dest_archetype.entity_ids.len();
        dest_archetype.entity_ids.push(entity_id);

        self.entity_map.insert(entity_id, EntityLocation {
            archetype_id: dest_archetype_id,
            row: dest_row,
        });
    }

    /// Adds a component of type `T` to the given entity.
    /// Note: This is a simplified version. A full implementation requires careful memory management.
    pub fn add_component<T: Component>(&mut self, entity_id: EntityId, _component: T) {
        let component_type_id = TypeId::of::<T>();
        let component_id = self.component_meta.get(&component_type_id)
            .expect("Component type not registered!").id;

        let current_location = self.entity_map.get(&entity_id).expect("Entity not found!").clone();

        // --- Borrow checker fix ---
        // First, perform the mutable operation to get the destination archetype ID.
        let dest_archetype_id = self.find_or_create_archetype(current_location.archetype_id, component_id);

        // Now, we can proceed with the rest of the logic without holding a mutable borrow.
        let current_archetype_id = self.entity_map.get(&entity_id).unwrap().archetype_id;

        if dest_archetype_id == current_archetype_id {
            // The entity already has this component (or one with the same archetype transition).
            return;
        }

        // The actual data move is complex. We'll simulate it for now.
        let src_row = self.entity_map.get(&entity_id).unwrap().row;
        self.move_entity_data(entity_id, current_archetype_id, src_row, dest_archetype_id);
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
        assert_eq!(empty_archetype.entity_ids.len(), 2);
        assert_eq!(empty_archetype.entity_ids[0], entity1);
        assert_eq!(empty_archetype.entity_ids[1], entity2);
    }

    // Define some test components
    struct Position { _x: f32, _y: f32 }
    impl Component for Position {}

    struct Velocity { _dx: f32, _dy: f32 }
    impl Component for Velocity {}

    #[test]
    fn add_component_moves_entity_to_new_archetype() {
        let mut world = World::new();

        world.register_component::<Position>();
        world.register_component::<Velocity>();

        let entity = world.create_entity();

        // 1. Check initial state (in empty archetype)
        let loc0_archetype_id = world.entity_map.get(&entity).unwrap().archetype_id;
        assert_eq!(loc0_archetype_id, 0);
        assert_eq!(world.tables[0].entity_ids.len(), 1);

        // 2. Add Position component
        world.add_component(entity, Position { _x: 0.0, _y: 0.0 });

        let loc1_archetype_id = world.entity_map.get(&entity).unwrap().archetype_id;
        assert_ne!(loc1_archetype_id, 0); // Should have moved
        assert_eq!(world.tables[0].entity_ids.len(), 0); // Should be gone from old archetype
        assert_eq!(world.tables.len(), 2); // A new archetype should have been created

        let archetype1 = &world.tables[loc1_archetype_id];
        assert_eq!(archetype1.entity_ids.len(), 1);
        assert_eq!(archetype1.entity_ids[0], entity);

        // 3. Add Velocity component
        world.add_component(entity, Velocity { _dx: 1.0, _dy: 0.0 });

        let loc2_archetype_id = world.entity_map.get(&entity).unwrap().archetype_id;
        assert_ne!(loc2_archetype_id, loc1_archetype_id); // Should have moved again
        assert_eq!(world.tables[loc1_archetype_id].entity_ids.len(), 0);
        assert_eq!(world.tables.len(), 3); // Another new archetype

        let archetype2 = &world.tables[loc2_archetype_id];
        assert_eq!(archetype2.entity_ids.len(), 1);
        assert_eq!(archetype2.entity_ids[0], entity);
    }
}
