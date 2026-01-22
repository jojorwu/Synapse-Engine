# Synapse Engine: Developer Manifesto

This document outlines the technical architecture and design principles for the Synapse Engine, a next-generation game engine focused on seamless multiplayer, moddability, and high performance.

---

## Part 1: Core Architecture (The Core)

The engine's foundation is built in Rust for maximum performance, memory safety, and control over low-level execution. It is designed around a high-performance Entity Component System (ECS) based on the Archetype model.

### 1.1. Language: Rust

- **Why Rust?**: Memory safety without a garbage collector (GC), predictable performance, and access to modern, low-level APIs (Vulkan, Metal).
- **Key Crates**: `wgpu` for the rendering backend, `rayon` for parallel computation, and `quinn` for the QUIC network protocol.

### 1.2. ECS Model: Archetypes

The core of the engine is an Archetype-based ECS, inspired by designs like `Flecs` and `Bevy`.

- **Concept**: An Archetype is a unique combination of component types. All entities with the exact same set of components belong to the same Archetype.
- **Memory Layout**: Each Archetype stores its components in a **Table** using a **Structure of Arrays (SoA)** layout.
    - Each component type corresponds to a tightly packed, contiguous array (a "column").
    - This layout is extremely CPU-cache-friendly, as systems can iterate over components linearly without memory jumps.
- **Performance**: This model enables near-zero-cost abstractions and allows for extremely fast querying and iteration, which is critical for Zero-Copy FFI.

### 1.3. Rust Data Structures

The following simplified Rust structures represent the core memory layout:

```rust
// Metadata for a component type, generated at compile-time.
struct ComponentMeta {
    id: u32,
    size: usize,
    is_sync: bool, // Corresponds to the [Sync] attribute in scripting.
}

// A table storing entities of a single Archetype.
struct ArchetypeTable {
    entity_ids: Vec<u32>,
    // Columns storing the actual component data.
    columns: HashMap<u32, Box<[u8]>>,
    // Bitmask for tracking changed ("dirty") data for network sync.
    dirty_masks: HashMap<u32, BitVec>,
    capacity: usize,
    len: usize,
}

// The main world container.
pub struct World {
    tables: Vec<ArchetypeTable>,
    // A map to quickly find where an entity is stored.
    entity_map: HashMap<u32, EntityLocation>, // (table_id, row_index)
}
```

### 1.4. Foreign Function Interface (FFI) Safety

To prevent memory safety issues (e.g., C# holding a pointer to memory that Rust reallocates), a **Double-Buffered Command System** is used.

- **Rule**: Structural changes (adding/removing components, creating/deleting entities) are **forbidden** during the main logic update phase.
- **Mechanism**: All such requests are recorded in a `CommandBuffer`.
- **Execution**: The `CommandBuffer` is processed only during a specific, safe phase of the frame loop ("Structural Sync"), when no user scripts are running.

---

## Part 2: Scripting API (The Bridge)

The Scripting API is designed to provide a balance between high-level convenience and low-level performance, allowing both novice and expert modders to be productive. The primary scripting languages are C# and Java.

### 2.1. Design Philosophy: Performance & Usability

- **API-First Design**: The API is designed from the modder's perspective first, ensuring the Rust core is built to serve their needs.
- **Dual-Path Approach**:
    - **Hot Path**: For performance-critical code (e.g., updating 10,000 entities), the API provides direct, Zero-Copy memory access via `Span<T>` (C#) or efficient iterators (Java).
    - **Convenience Path**: For event-driven, single-entity logic (e.g., quests, triggers), a simpler, Unity-style `MonoBehaviour` event model is provided.

### 2.2. Components: The Data

Components are defined as simple data structures (structs in C#). A source generator/annotation processor automates the FFI binding.

- **Network Sync**: The `[Sync]` attribute marks a field for automatic network replication to clients when its value changes on the server. The engine's core handles the dirty tracking and packet serialization.

**C# Example:**
```csharp
[Component]
public struct HealthComponent
{
    [Sync] public float Current;
    public float RegenRate; // Server-only logic
}
```

**Java Example:**
```java
@Component
public class HealthComponent {
    @Sync public float current;
    public float regenRate;
}
```

### 2.3. Systems: The Logic (High Performance)

Systems contain the game logic and operate on entities with a specific set of components. The API is designed to avoid FFI overhead by operating on batches of data.

- **Mechanism**: The Rust core provides pointers to contiguous component arrays (`columns`) within the `ArchetypeTable`.
- **Zero-Copy Access (C#)**: C# code receives a `Span<T>` that directly wraps the Rust memory. Modifications are made in-place.
- **Efficient Iteration (Java)**: A custom iterator provides a highly efficient way to loop over component data, minimizing object allocation.

**C# System Example:**
```csharp
public class RegenerationSystem : ISystem
{
    private Query<HealthComponent> _query;

    public void OnUpdate(float deltaTime)
    {
        // Get a Span<T> pointing directly to Rust's memory. No copying.
        foreach (ref var health in _query.ToSpan())
        {
            health.Current += health.RegenRate * deltaTime;
        }
    }
}
```

---

## Part 3: Asset Delivery System ("Synapse Sync")

Synapse Sync is the engine's killer feature, designed to solve the "mod hell" problem by making the multiplayer connection experience seamless and automatic.

### 3.1. Core Principle: Server Authority & Thin Client

- **Server-Side Logic**: All mod logic (game rules, item behavior) executes **only on the server**.
- **Thin Client**: The client is primarily a rendering terminal. It receives state updates and requests to play effects but does not know the game rules. This architecture makes cheating via client modification significantly harder.

### 3.2. Transport Protocol: HTTP/3 (QUIC)

- **Why QUIC?**: Eliminates Head-of-Line blocking, provides multiplexing for parallel asset streaming, and has built-in TLS 1.3 encryption. This is ideal for fast and reliable delivery of many small assets over potentially unstable connections.

### 3.3. Asset Storage: Content-Addressable Storage (CAS)

- **Git for Assets**: Assets are not identified by filenames but by the hash of their content (e.g., using BLAKE3).
- **Global Deduplication**: If two mods use the same texture, the client only downloads it once. This is verified by checking the hash.

### 3.4. The Asset Handshake Process

When a client connects to a server:
1.  **Manifest Exchange**: The server sends an `Asset Manifest`, a list of all required assets with their hashes, sizes, and priorities.
    ```json
    {
      "assets": [
        { "hash": "a1b2c3...", "size": 10240, "priority": "CRITICAL" },
        { "hash": "f5e6d7...", "size": 5000000, "priority": "LAZY" }
      ]
    }
    ```
2.  **Cache Diff**: The client compares the manifest against its local asset cache (a database of hashes).
3.  **Fetch Missing Assets**: The client requests only the assets (or chunks) it doesn't have.

### 3.5. Optimization: Delta Patching & P2P

- **Delta Patching (CDC)**: For mod updates, we use Content Defined Chunking (e.g., FastCDC). Instead of re-downloading a 500MB file, the client only downloads the few kilobytes that actually changed.
- **Hybrid P2P Distribution**:
    - **Server as Tracker**: The server keeps track of which connected clients have which asset chunks.
    - **Peer Fetching**: Clients can download chunks from other players, reducing server load.
    - **LAN Discovery**: Clients on the same local network discover each other via UDP broadcast and share assets at gigabit speeds.
    - **Security**: All chunks received from peers are hash-verified against the server's authoritative manifest before being used.

### 3.6. Streaming & Prioritization

- **Instant Play**: The game starts as soon as `CRITICAL` assets are loaded (e.g., UI, local spawn area geometry).
- **Lazy Loading**: `LAZY` assets (e.g., HD textures, sounds for a distant boss) are streamed in the background while the user is already playing.

---

## Part 4: Frame Lifecycle ("The Synapse Pulse")

The engine operates on a deterministic, phased frame loop to ensure data consistency, thread safety, and predictable execution. Each phase has a clear responsibility and strict data access rules.

| Phase                | Executor      | Memory Access      | Primary Goal                                  |
| -------------------- | ------------- | ------------------ | --------------------------------------------- |
| 1. **Ingestion**     | Rust (Async)  | Write (Input)      | Collect network packets & player input        |
| 2. **User Logic**    | C# / Java     | Read / Write       | Execute mod systems & gameplay logic        |
| 3. **Structural Sync** | Rust          | Structural Write   | Apply `CommandBuffer` (create/delete entities) |
| 4. **Simulation**    | Rust          | Read / Write       | Run physics, AI, animations                 |
| 5. **Broadcast**     | Rust (Async)  | Read-Only          | Send state updates to clients               |

### Phase 1: Ingestion

- **Action**: The asynchronous networking thread (Rust) pulls all available data from network sockets (client inputs, P2P asset chunks).
- **Result**: Input components are updated for the current frame.

### Phase 2: User Logic

- **Action**: All user-defined systems (C# / Java) are executed.
- **Parallelism**: Systems that operate on disjoint sets of components can be run in parallel using a job scheduler (`rayon`).
- **Safety**: Any request to change the ECS structure (e.g., `CreateEntity`) is deferred to a `CommandBuffer` instead of being executed immediately. This keeps memory layouts stable for the duration of this phase.

### Phase 3: Structural Sync

- **Action**: This is a short, exclusive-access "stop-the-world" phase. No user code runs.
- **Purpose**: The Rust core processes the `CommandBuffer` to apply all structural changes. Entities may be moved between Archetype Tables.
- **Result**: The ECS is in a consistent state, ready for internal simulation.

### Phase 4: Simulation

- **Action**: The engine's high-performance, internal systems run on the Rust core.
- **Tasks**: Physics simulation (e.g., using Rapier), pathfinding, animation updates.
- **Event Generation**: Events detected during this phase (e.g., `OnTriggerEnter`) are queued to be dispatched to the scripting API on the *next* frame.

### Phase 5: Broadcast & Render

- **Sync Scan**: The networking system scans the `dirty_masks` in the `ArchetypeTable` for all components marked `[Sync]`.
- **Packet Generation**: A compact delta-packet is created containing only the data that has changed and is sent to clients.
- **Client-Side Prediction**: To hide latency, clients predict the results of their own inputs immediately and are smoothly corrected if the server's authoritative state differs.
