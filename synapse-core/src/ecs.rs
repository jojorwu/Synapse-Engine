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
    // Using Vec<u8> instead of Box<[u8]> allows for dynamic resizing.
    pub columns: HashMap<ComponentId, Vec<u8>>,
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

    /// Adds a component of type `T` to the given entity, moving it to a new archetype.
    pub fn add_component<T: Component>(&mut self, entity_id: EntityId, component: T) {
        let component_type_id = TypeId::of::<T>();
        let component_id = self.component_meta.get(&component_type_id)
            .expect("Component type not registered!").id;

        let src_location = *self.entity_map.get(&entity_id).expect("Entity not found!");

        let dest_archetype_id = self.find_or_create_archetype(src_location.archetype_id, component_id);

        if dest_archetype_id == src_location.archetype_id {
            // The entity already has this component. We could update the value, but for now we do nothing.
            return;
        }

        // --- Data Move Operation ---
        let (src_archetype, dest_archetype) = if src_location.archetype_id < dest_archetype_id {
            let (left, right) = self.tables.split_at_mut(dest_archetype_id);
            (&mut left[src_location.archetype_id], &mut right[0])
        } else {
            let (left, right) = self.tables.split_at_mut(src_location.archetype_id);
            (&mut right[0], &mut left[dest_archetype_id])
        };

        let dest_row = dest_archetype.len;

        // 1. Copy old component data from source to destination.
        for &comp_id in &src_archetype.component_ids.clone() {
            let src_col = src_archetype.columns.get(&comp_id).unwrap();
            let dest_col = dest_archetype.columns.entry(comp_id).or_insert_with(Vec::new);
            let size = self.component_meta.values().find(|meta| meta.id == comp_id).unwrap().size;

            let start = src_location.row * size;
            let end = start + size;
            dest_col.extend_from_slice(&src_col[start..end]);
        }

        // 2. Add the new component's data to the destination.
        let new_comp_size = mem::size_of::<T>();
        let dest_col = dest_archetype.columns.entry(component_id).or_insert_with(Vec::new);
        unsafe {
            let bytes = std::slice::from_raw_parts(&component as *const T as *const u8, new_comp_size);
            dest_col.extend_from_slice(bytes);
        }

        // 3. Remove entity from the source archetype using swap_remove.
        let last_row_in_src = src_archetype.len - 1;
        for &comp_id in &src_archetype.component_ids {
            let col = src_archetype.columns.get_mut(&comp_id).unwrap();
            let size = self.component_meta.values().find(|meta| meta.id == comp_id).unwrap().size;

            let src_start = src_location.row * size;

            if src_location.row < last_row_in_src {
                let last_start = last_row_in_src * size;
                let last_end = last_start + size;
                col.copy_within(last_start..last_end, src_start);
            }
            col.truncate(last_row_in_src * size);
        }

        let moved_entity_id = src_archetype.entity_ids.swap_remove(src_location.row);
        src_archetype.len -= 1;

        if src_location.row < src_archetype.len {
            // If an entity was moved to fill the gap, update its location in the map.
            self.entity_map.get_mut(&moved_entity_id).unwrap().row = src_location.row;
        }

        // 4. Finalize the move.
        dest_archetype.entity_ids.push(entity_id);
        dest_archetype.len += 1;

        let new_location = self.entity_map.get_mut(&entity_id).unwrap();
        new_location.archetype_id = dest_archetype_id;
        new_location.row = dest_row;
    }

    /// Gets a reference to a component of type `T` for the given entity.
    pub fn get_component<T: Component>(&self, entity_id: EntityId) -> Option<&T> {
        let location = self.entity_map.get(&entity_id)?;
        let archetype = &self.tables[location.archetype_id];

        let component_type_id = TypeId::of::<T>();
        let component_id = self.component_meta.get(&component_type_id)?.id;

        let column = archetype.columns.get(&component_id)?;

        let size = mem::size_of::<T>();
        let start = location.row * size;
        let end = start + size;
        let bytes = &column[start..end];

        // This is unsafe because we are reinterpreting raw bytes as a specific type.
        // It's safe here because we've verified the component type and size.
        unsafe {
            Some(&*(bytes.as_ptr() as *const T))
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
        assert_eq!(empty_archetype.entity_ids.len(), 2);
        assert_eq!(empty_archetype.entity_ids[0], entity1);
        assert_eq!(empty_archetype.entity_ids[1], entity2);
    }

    // Define some test components
    #[derive(PartialEq, Debug, Clone, Copy)]
    struct Position { x: f32, y: f32 }
    impl Component for Position {}

    #[derive(PartialEq, Debug, Clone, Copy)]
    struct Velocity { dx: f32, dy: f32 }
    impl Component for Velocity {}

    #[test]
    fn add_component_moves_entity_and_data_correctly() {
        let mut world = World::new();

        world.register_component::<Position>();
        world.register_component::<Velocity>();

        let entity = world.create_entity();

        // 1. Check initial state (in empty archetype)
        let loc0_archetype_id = world.entity_map.get(&entity).unwrap().archetype_id;
        assert_eq!(loc0_archetype_id, 0);
        assert_eq!(world.tables[0].entity_ids.len(), 1);

        // 2. Add Position component
        world.add_component(entity, Position { x: 1.0, y: 2.0 });

        let loc1_archetype_id = world.entity_map.get(&entity).unwrap().archetype_id;
        assert_ne!(loc1_archetype_id, 0); // Should have moved
        assert_eq!(world.tables[0].entity_ids.len(), 0); // Should be gone from old archetype
        assert_eq!(world.tables.len(), 2); // A new archetype should have been created

        let archetype1 = &world.tables[loc1_archetype_id];
        assert_eq!(archetype1.entity_ids.len(), 1);
        assert_eq!(archetype1.entity_ids[0], entity);

        // Verify data integrity
        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(*pos, Position { x: 1.0, y: 2.0 });

        // 3. Add Velocity component
        world.add_component(entity, Velocity { dx: 1.0, dy: 0.0 });

        let loc2_archetype_id = world.entity_map.get(&entity).unwrap().archetype_id;
        assert_ne!(loc2_archetype_id, loc1_archetype_id); // Should have moved again
        assert_eq!(world.tables[loc1_archetype_id].entity_ids.len(), 0);
        assert_eq!(world.tables.len(), 3); // Another new archetype

        let archetype2 = &world.tables[loc2_archetype_id];
        assert_eq!(archetype2.entity_ids.len(), 1);
        assert_eq!(archetype2.entity_ids[0], entity);

        // Verify data integrity again
        let pos_after_move = world.get_component::<Position>(entity).unwrap();
        let vel_after_move = world.get_component::<Velocity>(entity).unwrap();
        assert_eq!(*pos_after_move, Position { x: 1.0, y: 2.0 });
        assert_eq!(*vel_after_move, Velocity { dx: 1.0, dy: 0.0 });
    }
}
