use rust_shader_tools::{ShaderCompiler, ShaderStage};

fn main() {
    let shader_compiler = ShaderCompiler::new().expect("Could not create shader compiler");
    shader_compiler
        .compile_shader("shaders/default.vert", ShaderStage::Vertex)
        .expect("Could not compile shader");
    shader_compiler
        .compile_shader("shaders/default.frag", ShaderStage::Fragment)
        .expect("Could not compile shader");
}
