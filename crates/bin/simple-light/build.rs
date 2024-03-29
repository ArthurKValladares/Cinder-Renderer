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
        .compile_and_write_shader("shaders/light.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/light.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/light.vert.spv").unwrap(),
        "light",
        PathBuf::from("gen").join("light_shader_structs.rs"),
        false,
    );

    shader_compiler
        .compile_and_write_shader("shaders/lit_mesh.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/lit_mesh.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/lit_mesh.vert.spv").unwrap(),
        "LitMesh",
        PathBuf::from("gen").join("lit_mesh_shader_structs.rs"),
        false,
    );

    shader_compiler
        .compile_and_write_shader("shaders/shadow_map.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/shadow_map.vert.spv").unwrap(),
        "ShadowMap",
        PathBuf::from("gen").join("shadow_map_shader_structs.rs"),
        false,
    );

    shader_compiler
        .compile_and_write_shader("shaders/shadow_map_quad.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/shadow_map_quad.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    rust_shader_tools::write_shader_structs(
        &std::fs::read("./shaders/spv/shadow_map_quad.vert.spv").unwrap(),
        "ShadowMapQuad",
        PathBuf::from("gen").join("shadow_map_quad_shader_structs.rs"),
        false,
    );
}
