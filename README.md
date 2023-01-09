# Cinder-Renderer
Modern renderer written in rust, aiming to be a modern, cross-platform, GPU-driven renderer.
API and backend code are both still very temporary and going trough a major refactoring pass.

## Examples
### [Hello Triangle](./crates/bin/hello-triangle/src/main.rs)<br/>
Hello triangle example with vertex colors and a transform matrix sent per-frame to the vertex buffer.

![hello-triangle](https://user-images.githubusercontent.com/23410311/211144602-96c42b6b-355e-4d5c-a2f3-8897c80d7029.gif)

### [Hello Cube](./crates/bin/hello-cube/src/main.rs)<br/>
Hello cube example with vertex colors and a model/view/projection matrix uniform buffer object, where the model matrix is upadated per-frame.

![hello-cube](https://user-images.githubusercontent.com/23410311/211144696-135565dd-0b67-4e00-97c5-1a8b1d7562f0.gif)

### [Texture](./crates/bin/texture/src/main.rs)<br/>
Basic textured quad.

![texture](https://user-images.githubusercontent.com/23410311/211232696-af123319-85a8-45dc-9be8-ff79af567cee.png)
