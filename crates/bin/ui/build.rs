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
        .compile_and_write_shader("shaders/ui.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/ui.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");

    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/ui.vert.spv").unwrap(),
        "ui",
        PathBuf::from("gen").join("ui_shader_structs.rs"),
    );

    shader_compiler
        .compile_and_write_shader(
            "../../lib/egui-integration/shaders/egui.vert",
            ShaderStage::Vertex,
        )
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader(
            "../../lib/egui-integration/shaders/egui.frag",
            ShaderStage::Fragment,
        )
        .expect("Could not compile shader");
}
