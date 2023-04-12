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
        .compile_and_write_shader("shaders/hot_reload.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/hot_reload.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/hot_reload.vert.spv").unwrap(),
        "hot_reload",
        PathBuf::from("gen").join("hot_reload_shader_structs.rs"),
        false,
    );
}
