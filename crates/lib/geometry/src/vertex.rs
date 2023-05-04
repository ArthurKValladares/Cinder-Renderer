// TODO: In the future this will be replaced by the Vertex trait, after I make it better
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
}
