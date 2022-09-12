use crate::{
    context::{
        graphics_context::{GraphicsContext, GraphicsContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
        Context,
    },
    resoruces::{
        buffer::{Buffer, BufferDescription},
        pipeline::{Pipeline, PipelineDescription},
        render_pass::{RenderPass, RenderPassDescription},
        shader::{Shader, ShaderDescription},
        texture::{Texture, TextureDescription},
    },
};

pub struct Device {}

impl Device {
    pub fn create_buffer(&self, desc: BufferDescription) -> Buffer {
        Buffer {}
    }

    pub fn create_texture(&self, desc: TextureDescription) -> Texture {
        Texture {}
    }

    pub fn create_shader(&self, desc: ShaderDescription) -> Shader {
        Shader {}
    }

    pub fn create_render_pass(&self, desc: RenderPassDescription) -> RenderPass {
        RenderPass {}
    }

    pub fn create_pipeline(&self, desc: PipelineDescription) -> Pipeline {
        Pipeline {}
    }

    pub fn create_graphics_context(&self, desc: GraphicsContextDescription) -> GraphicsContext {
        GraphicsContext {}
    }

    pub fn create_upload_context(&self, desc: UploadContextDescription) -> UploadContext {
        UploadContext {}
    }

    pub fn submit_work(&self, context: &dyn Context) {}
}
