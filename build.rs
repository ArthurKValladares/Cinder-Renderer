use std::path::PathBuf;

use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderStage};

fn main() {
    let shader_compiler = ShaderCompiler::new(
        EnvVersion::Vulkan1_2,
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
}
