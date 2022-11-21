use std::path::PathBuf;

use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderData, ShaderStage};

fn main() {
    let shader_compiler = ShaderCompiler::new(
        EnvVersion::Vulkan1_0,
        OptimizationLevel::Zero,
        Some(PathBuf::from("shaders")),
    )
    .expect("Could not create shader compiler");
    shader_compiler
        .compile_shader("shaders/default.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_shader("shaders/default.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    shader_compiler
        .compile_shader("egui-integration/shaders/egui.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_shader("egui-integration/shaders/egui.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");

    // TODO: contain this logic better later
    let vert_module =
        ShaderData::from_spv(include_bytes!("./shaders/spv/default.vert.spv")).unwrap();
    let vert_structs = vert_module.get_shader_structs();
    let rust_vert_structs = vert_structs
        .into_iter()
        .map(|stct| {
            let struct_name = rust_shader_tools::standardized_struct_name("default", &stct.name);
            let is_vertex = stct.name.contains("Vertex");
            rust_shader_tools::shader_struct_to_rust(&struct_name, &stct, is_vertex)
        })
        .collect::<Vec<_>>();

    rust_shader_tools::structs_to_file(
        PathBuf::from("gen").join("shader_structs.rs"),
        &rust_vert_structs,
    )
    .expect("Could not write structs to file");
}
