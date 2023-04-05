pub use tobj::Mesh as ObjMesh;

pub trait Vertex {
    fn from_obj_mesh_index(mesh: &ObjMesh, i: usize) -> Self;

    fn pos_3d(&self) -> [f32; 3];
}
