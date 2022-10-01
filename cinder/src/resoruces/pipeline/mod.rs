use super::{render_pass::RenderPass, shader::Shader};
use anyhow::Result;
use ash::vk;
use std::collections::HashMap;

// TODO: This lifetime is annoying
pub struct GraphicsPipelineDescription<'a> {
    pub vertex_shader: Shader,
    pub fragment_shader: Shader,
    pub render_pass: &'a RenderPass,
}

pub struct PipelineCommon {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    //pub set_layout_info: Vec<HashMap<u32, vk::DescriptorType>>,
    //pub descriptor_pool_sizes: Vec<vk::DescriptorPoolSize>,
    //pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    //pub pipeline_bind_point: vk::PipelineBindPoint,
}

pub struct GraphicsPipeline {
    pub common: PipelineCommon,
}

impl GraphicsPipeline {
    pub(crate) fn create(device: &ash::Device) -> Result<Self> {
        todo!()
    }
}
