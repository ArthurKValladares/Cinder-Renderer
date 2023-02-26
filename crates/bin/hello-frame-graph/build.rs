use std::path::PathBuf;

use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderData, ShaderStage};

fn write_shader_structs(bytes: &[u8], prefix: &'static str) {
    // TODO: contain this logic better later
    let vert_module = ShaderData::from_spv(bytes).unwrap();
    let vert_structs = {
        let mut descriptor_structs = vert_module.get_shader_structs();
        let mut pc_structs = vert_module.get_push_constant_structs();
        descriptor_structs.append(&mut pc_structs);
        descriptor_structs
    };

    let mut rust_vert_structs = vert_structs
        .into_iter()
        .map(|stct| {
            let struct_name = rust_shader_tools::standardized_struct_name(prefix, &stct.name);
            rust_shader_tools::shader_struct_to_rust(&struct_name, &stct)
        })
        .collect::<Vec<_>>();

    let vert_attributes = vert_module.get_vertex_attributes();
    if !vert_attributes.atts.is_empty() {
        let stct = rust_shader_tools::vertex_attributes_to_struct(
            &rust_shader_tools::standardized_struct_name(prefix, "Vertex"),
            &vert_attributes.atts,
        );
        rust_vert_structs.push(stct);
    }

    rust_shader_tools::structs_to_file(
        PathBuf::from("gen").join(format!("{prefix}_shader_structs.rs")),
        &rust_vert_structs,
    )
    .expect("Could not write structs to file");
}

fn main() {
    let shader_compiler = ShaderCompiler::new(
        EnvVersion::Vulkan1_0,
        OptimizationLevel::Zero,
        Some(PathBuf::from("shaders")),
    )
    .expect("Could not create shader compiler");

    shader_compiler
        .compile_and_write_shader("shaders/frame_graph.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_and_write_shader("shaders/frame_graph.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
    write_shader_structs(
        &std::fs::read("./shaders/spv/frame_graph.vert.spv").unwrap(),
        "frame_graph",
    );
}
