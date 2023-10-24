use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderStage};
use std::path::PathBuf;

fn main() {
    let shader_compiler = ShaderCompiler::new(
        EnvVersion::Vulkan1_0,
        OptimizationLevel::Zero,
        Some(PathBuf::from("shaders")),
    )
    .expect("Could not create shader compiler");

    shader_compiler
        .compile_and_write_shader("shaders/triangle.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/triangle.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");

    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/triangle.vert.spv").unwrap(),
        "triangle",
        PathBuf::from("gen").join("triangle_shader_structs.rs"),
        false,
    );
}
