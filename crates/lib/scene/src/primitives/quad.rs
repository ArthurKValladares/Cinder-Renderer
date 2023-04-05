use math::{point::Point3D, size::Size2D};

use crate::{Mesh, Vertex};

pub enum QuadPlane {
    X,
    Y,
    Z,
}

pub fn new_quad<V: Vertex>(center: Point3D<f32>, size: Size2D<f32>, plane: QuadPlane) -> Mesh<V> {
    let x = center.x();
    let y = center.y();
    let z = center.z();

    let h_width = size.width() / 2.0;
    let h_height = size.width() / 2.0;

    let indices = vec![0, 1, 2, 2, 3, 0];
    let vertices = match plane {
        QuadPlane::X => vec![
            V::default().set_pos_3d(x, y, z),
            V::default().set_pos_3d(x, y, z),
            V::default().set_pos_3d(x, y, z),
            V::default().set_pos_3d(x, y, z),
        ],
        QuadPlane::Y => vec![
            V::default().set_pos_3d(x, y, z),
            V::default().set_pos_3d(x, y, z),
            V::default().set_pos_3d(x, y, z),
            V::default().set_pos_3d(x, y, z),
        ],
        QuadPlane::Z => vec![
            V::default().set_pos_3d(x - h_width, y - h_height, z),
            V::default().set_pos_3d(x + h_width, y - h_height, z),
            V::default().set_pos_3d(x + h_width, y + h_height, z),
            V::default().set_pos_3d(x - h_width, y + h_height, z),
        ],
    };

    // TODO: Actually calculate this right
    let (min_pos, max_pos) = match plane {
        QuadPlane::X => ([x, y, z], [x, y, z]),
        QuadPlane::Y => ([x, y, z], [x, y, z]),
        QuadPlane::Z => ([x, y, z], [x, y, z]),
    };

    Mesh {
        indices,
        vertices,
        material_index: None,
        min_pos,
        max_pos,
    }
}
