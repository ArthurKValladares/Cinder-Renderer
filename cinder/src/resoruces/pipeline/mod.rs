use super::{render_pass::RenderPass, shader::Shader};
use ash::vk;

// TODO: This lifetime is annoying
pub struct GraphicsPipelineDescription<'a> {
    pub vertex_shader: Shader,
    pub fragment_shader: Shader,
    pub render_pass: &'a RenderPass,
}

pub struct PipelineCommon {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

pub struct GraphicsPipeline {
    pub common: PipelineCommon,
}
