use super::shader::Shader;
use crate::resource_pool::Handle;
use ash::vk;

pub struct Program {
    // TODO: Mesh shader
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
}
