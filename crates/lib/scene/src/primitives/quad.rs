use crate::{Mesh, Vertex};

// Quad of size [1,1], centered at Z-coordinate 0
pub fn new_quad<V: Vertex>() -> Mesh<V> {
    let indices = vec![0, 1, 2, 2, 3, 0];
    let vertices = vec![
        V::default().set_pos_3d(-0.5, -0.5, 0.0).set_uv(0.0, 1.0),
        V::default().set_pos_3d(0.5, -0.5, 0.0).set_uv(1.0, 1.0),
        V::default().set_pos_3d(0.5, 0.5, 0.0).set_uv(1.0, 0.0),
        V::default().set_pos_3d(-0.5, 0.5, 0.0).set_uv(0.0, 0.0),
    ];

    Mesh {
        indices,
        vertices,
        material_index: None,
        min_pos: [-0.5, -0.5, 0.0],
        max_pos: [0.5, 0.5, 0.0],
    }
}
