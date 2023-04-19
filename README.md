# Cinder-Renderer
Modern renderer written in rust, aiming to be a modern, cross-platform, GPU-driven renderer.
API and backend code are both still very temporary and going trough a major refactoring pass.

# Building And Running

To build and run the project, you need to install [Rust](https://www.rust-lang.org/tools/install) and [CMake](https://cmake.org/).

After Rust is installed, simply open a command promp at the project root directiory and run the command:

```
cargo run --bin <BIN> --release
```

The currently available binaries to run are:

```
hello-triangle
hello-cube
texture
ui
mesh
debug
depth-image
bindless
shader-hot-reload
```

### macOS
 We currently rely on MoltenVK to run on macOS. To run the project, you must install the [LunarG Vulkan SDK](https://www.lunarg.com/vulkan-sdk/). 
 
Once installed, set the environment variable `VULKAN_SDK_PATH` to the path you installed the SDK in, by default: `$HOME/VulkanSDK/<version>`. Then, run the command `source tools/setup_molten_vk.sh`, and if everything succeeds, you should be able to use the regular steps to run the project.
 
## Examples
### [Hello Triangle](./crates/bin/hello-triangle/src/main.rs)<br/>
Hello triangle example with vertex colors and a transform matrix sent per-frame to the vertex buffer.

![hello-triangle](https://user-images.githubusercontent.com/23410311/211144602-96c42b6b-355e-4d5c-a2f3-8897c80d7029.gif)

### [Hello Cube](./crates/bin/hello-cube/src/main.rs)<br/>
Hello cube example with vertex colors and a model/view/projection matrix uniform buffer object, where the model matrix is upadated per-frame.

![hello-cube](https://user-images.githubusercontent.com/23410311/211144696-135565dd-0b67-4e00-97c5-1a8b1d7562f0.gif)

### [Texture](./crates/bin/texture/src/main.rs)<br/>
Basic textured quad.

![texture](https://user-images.githubusercontent.com/23410311/211232839-00e248d9-9c73-4b71-9e00-06d532930cde.gif)

### [Ui](./crates/bin/ui/src/main.rs)<br/>
Example app using egui to render ui widgets we can use to transform a cube.

![ui](https://user-images.githubusercontent.com/23410311/211710290-65f36d24-180f-4af4-b55c-9dc2920d0306.gif)

### [Mesh](./crates/bin/mesh/src/main.rs)<br/>
3D mesh with a single albedo texture.

![mesh](https://user-images.githubusercontent.com/23410311/212804707-f4f97fb4-d63d-4449-9b20-31a01a228904.gif)

### [Debug](./crates/bin/debug/src/main.rs)<br/>
Debug names and labels.

<img width="892" alt="debug" src="https://user-images.githubusercontent.com/23410311/214242577-cbc09ca9-aedb-4465-8bc0-94162b31807b.png">
<img width="625" alt="debug_events" src="https://user-images.githubusercontent.com/23410311/214773768-d88bfb9e-a679-4dec-87d7-c2331dae89f5.png">

### [Depth Image](./crates/bin/depth-image/src/main.rs)<br/>
Render Depth image to the screen in a second pass.

![depth-image](https://user-images.githubusercontent.com/23410311/232945597-0e4ba4fe-5570-4ad1-93a6-8c7193114dd6.gif)

### [Bindless](./crates/bin/bindless/src/main.rs)<br/>
Draw a complex scene using bindless textures and a uniform buffer for vertex data.
Will be very slow to load the first time as we are processing mesh and texture data and writting an efficient zero-copy deserializable runtime format that will be used in subsequent loads.


![sponza](https://user-images.githubusercontent.com/23410311/218249268-324efc6f-941c-4787-babb-00d82991ae1d.png)

### [Hot Reload](./crates/bin/shader-hot-reload/src/main.rs)<br/>
Shader hot-reloading.



https://user-images.githubusercontent.com/23410311/220039871-11cf4ead-a305-4210-bf8e-04ae78656b48.mp4


