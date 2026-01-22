# Synapse Engine: Editor Design Specification

This document outlines the design for the core features of the Synapse Engine Editor.

---

## 1. Scene View

The Scene View is the user's primary interactive window into the game world. It provides a real-time, visual representation of the entities that compose a scene.

### 1.1. Core Principle: A Running Instance

The Scene View is not a static representation; it is a direct render output from an instance of the Synapse core running in a special "editor mode". This ensures that what you see in the editor is exactly what you get in the game, including shaders, particles, and basic animations.

### 1.2. Camera and Navigation

The user can navigate the scene using a standard set of camera controls:
- **Orbit (Alt + Left Mouse Button)**: Rotates the camera around a focal point.
- **Pan (Middle Mouse Button)**: Moves the camera parallel to the view plane.
- **Zoom (Mouse Wheel)**: Moves the camera forward and backward.

### 1.3. Entity Selection

- **Direct Selection**: Users can select an entity by clicking on its visual representation in the scene.
- **Visual Feedback**: The selected entity will be highlighted with an outline or wireframe to indicate it is active. Multiple selections will be supported.

### 1.4. Manipulation Gizmos

When an entity with a `TransformComponent` is selected, a 3D gizmo appears, allowing for direct manipulation. The gizmo will have three modes, switchable via keyboard shortcuts (e.g., W, E, R):
- **Translate**: Displays arrows along the X, Y, and Z axes for moving the entity.
- **Rotate**: Displays rings for rotating the entity around each axis.
- **Scale**: Displays cubes for scaling the entity along each axis.

Any changes made via the gizmo will directly modify the data in the entity's `TransformComponent`, which will then be reflected in the "Entity Inspector".

---

## 2. Entity Inspector

The Entity Inspector is a context-sensitive panel that displays all the components attached to the currently selected entity and allows for their modification.

### 2.1. Dynamic UI Generation

The Inspector's UI is not predefined. It will be generated dynamically at runtime by reflecting on the components of the selected entity.

- **Reflection**: The editor will read the metadata of the C# and Java component scripts (`.dll`/`.jar` files) to discover their public fields and types.
- **Type-to-Widget Mapping**: The editor will map script-defined data types to appropriate UI widgets:
    - `float`, `int`: Number input field with drag-to-change functionality.
    - `boolean`: Checkbox.
    - `string`: Text input field.
    - `Vector3`, `Color`: A compound control with multiple number inputs.
    - `Enum`: Dropdown list.
    - `Entity Reference`: A special field where another entity can be dragged and dropped.

### 2.2. Core Functionality

- **View Components**: All components on the selected entity are displayed as collapsible sections, with the component's name as the header.
- **Edit Component Fields**: Users can modify the public fields of each component directly in the inspector. Changes are applied to the engine instance in real-time.
- **Add Component**: An "Add Component" button at the bottom of the panel will open a searchable dropdown list of all available component types discovered from the user's scripts. Selecting one will add it to the current entity.
- **Remove Component**: Each component section will have a context menu (e.g., a gear icon or right-click) with an option to remove the component from the entity. Critical components like `TransformComponent` may be locked from removal.

### 2.3. Data Binding

The Inspector's fields are two-way data-bound to the component data in the Rust core.
- **Editor to Engine**: Changing a value in the UI immediately calls a command to update the component data in the running engine instance.
- **Engine to Editor**: If the component data is changed by a running game system (e.g., health decreasing), the UI in the inspector will automatically update to reflect the new value.

---

## 3. Asset Browser

The Asset Browser is the interface for viewing and managing all the files within the project's `assets/` directory.

### 3.1. Filesystem Synchronization

- **Live Monitoring**: The editor will use a file system watcher to monitor the `assets/` directory for any changes (files added, removed, or renamed).
- **Automatic Import**: When a new file is detected (e.g., a user drags an `.fbx` or `.png` file into the folder), the editor will automatically trigger an import process. This process will generate the necessary engine-native format and a hash for the asset, preparing it for the Synapse Sync system.

### 3.2. User Interface

The browser will feature a standard two-pane layout:
- **Left Pane**: A collapsible tree view of the folder structure within `assets/`.
- **Right Pane**: A grid or list view displaying the contents of the selected folder. Assets will be represented by thumbnails (for textures, models) or icons (for scripts, materials).

### 3.3. Core Functionality

- **Asset Preview**: Selecting an asset in the browser will display its metadata and a preview (if applicable) in the Inspector panel.
- **Drag-and-Drop**:
    - **To Scene**: Dragging a model asset into the Scene View will create a new entity with the appropriate `RenderComponent` and `TransformComponent`.
    - **To Inspector**: Dragging a texture asset onto a material's texture field in the Inspector will assign it.
- **Basic File Operations**: A right-click context menu will provide standard file system operations like `Create Folder`, `Rename`, `Delete`, and `Show in Explorer/Finder`.

---

## 4. Scripting Integration

The editor is not an IDE. Its goal is to provide seamless integration with professional code editors (VS Code, Rider, IntelliJ IDEA) and to manage the compilation and hot-reloading of scripts.

### 4.1. Project Recognition

- **File-Based Detection**: The editor will look for project files (e.g., `.csproj` for C#, `build.gradle`/`pom.xml` for Java) in the project's root directory.
- **Solution Generation**: A button `Open in IDE` will generate the necessary solution files (`.sln`, etc.) if they don't exist and then launch the user's preferred, configured IDE.

### 4.2. Opening Scripts

- **Double-Click Action**: Double-clicking a script asset (`.cs`, `.java`) in the Asset Browser will open that specific file in the external IDE, jumping directly to the correct line and column.

### 4.3. Script Compilation and Hot-Reload

- **Background Watcher**: The editor will monitor the script project's output directories (e.g., `bin/Debug`).
- **Automatic Hot-Reload**: When a change is detected (i.e., the user compiles their code in the IDE, producing a new `.dll` or `.jar`), the editor will:
    1.  Unload the old script assembly from the running engine instance.
    2.  Load the new assembly.
    3.  Re-run the reflection process to discover any new components or changed fields.
    4.  Refresh the Entity Inspector to reflect the new component structure.
- **State Preservation**: The editor will attempt to preserve the values of serialized fields on components during a hot-reload, so the user does not lose their work in the scene.
