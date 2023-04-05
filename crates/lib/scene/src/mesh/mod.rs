use crate::Vertex;
use tobj::Model;

pub struct Mesh<V: Vertex> {
    pub indices: Vec<u32>,
    pub vertices: Vec<V>,
    pub material_index: Option<usize>,
    pub min_pos: [f32; 3],
    pub max_pos: [f32; 3],
}

impl<V> Mesh<V>
where
    V: Vertex,
{
    pub fn from_obj_model(model: Model) -> Self {
        let obj_mesh = model.mesh;

        let mut mesh_min_pos = [f32::INFINITY; 3];
        let mut mesh_max_pos = [f32::NEG_INFINITY; 3];
        let mut vertices = Vec::with_capacity(obj_mesh.positions.len() / 3);

        for i in 0..obj_mesh.positions.len() / 3 {
            let vertex = V::from_obj_mesh_index(&obj_mesh, i);
            let pos = vertex.pos_3d();

            vertices.push(vertex);

            for i in 0..3 {
                mesh_min_pos[i] = f32::min(mesh_min_pos[i], pos[i]);
                mesh_max_pos[i] = f32::max(mesh_max_pos[i], pos[i]);
            }
        }

        Self {
            indices: obj_mesh.indices,
            vertices,
            material_index: obj_mesh.material_id,
            min_pos: mesh_min_pos,
            max_pos: mesh_max_pos,
        }
    }
}
