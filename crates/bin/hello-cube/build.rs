use std::path::PathBuf;

use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderStage};

fn main() {
    let shader_compiler = ShaderCompiler::new(
        EnvVersion::Vulkan1_0,
        OptimizationLevel::Zero,
        Some(PathBuf::from("shaders")),
    )
    .expect("Could not create shader compiler");

    shader_compiler
        .compile_and_write_shader("shaders/cube.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/cube.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/cube.vert.spv").unwrap(),
        "cube",
        PathBuf::from("gen").join("cube_shader_structs.rs"),
        false,
    );
}
