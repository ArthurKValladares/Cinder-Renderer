use super::shader::Shader;
use crate::resource_pool::{Handle, Pool};
use ash::vk;

pub struct Program {
    // TODO: Mesh shader
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    // TODO: will get these from reflection
    //descriptor_set_layout: vk::DescriptorSetLayout,
    //pipeline_layout: vk::PipelineLayout,
}

impl Program {
    pub fn create(
        shader_pool: Pool<Shader>,
        vertex_shader: Handle<Shader>,
        fragment_shader: Handle<Shader>,
    ) -> Self {
        Self {
            vertex_shader,
            fragment_shader,
        }
    }
}
