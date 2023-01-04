// TODO: This is very bad, should be app-side and not engine side
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/mesh_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/egui_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/triangle_shader_structs.rs"
));

pub const RESERVED_DESCRIPTOR_COUNT: u32 = 32;
