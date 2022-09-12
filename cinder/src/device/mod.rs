use crate::resoruces::{
    buffer::{Buffer, BufferDescription},
    pipeline::{Pipeline, PipelineDescription},
    render_pass::{RenderPass, RenderPassDescription},
    shader::{Shader, ShaderDescription},
    texture::{Texture, TextureDescription},
};

pub struct Device {}

impl Device {
    pub fn create_buffer(&self, desc: BufferDescription) -> Buffer {
        todo!()
    }

    pub fn create_texture(&self, desc: TextureDescription) -> Texture {
        todo!()
    }

    pub fn create_shader(&self, desc: ShaderDescription) -> Shader {
        todo!()
    }

    pub fn create_render_pass(&self, desc: RenderPassDescription) -> RenderPass {
        todo!()
    }

    pub fn create_pipeline(&self, desc: PipelineDescription) -> Pipeline {
        todo!()
    }
}
