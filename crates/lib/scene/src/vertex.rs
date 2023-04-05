pub use tobj::Mesh as ObjMesh;

pub trait Vertex: Default {
    fn from_obj_mesh_index(mesh: &ObjMesh, i: usize) -> Self;

    fn pos_3d(&self) -> [f32; 3];

    // TODO: These should not need to be `f32`
    fn set_pos_3d(self, x: f32, y: f32, z: f32) -> Self;
    fn set_uv(self, _u: f32, _v: f32) -> Self {
        unimplemented!()
    }
}
