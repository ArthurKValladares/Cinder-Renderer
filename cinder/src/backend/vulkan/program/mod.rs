use crate::resource_pool::Handle;
use ash::vk;

struct Shader {
    shader_module: vk::ShaderModule,
    binding: vk::DescriptorSetLayoutBinding,
}

pub struct Program {
    // TODO: Mesh shader
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
}
