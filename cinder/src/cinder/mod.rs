include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/default_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/egui_shader_structs.rs"
));

pub const RESERVED_DESCRIPTOR_COUNT: u32 = 32;
