use super::{
    buffer::Buffer, image::Image, pipeline::graphics::GraphicsPipeline, sampler::Sampler,
    shader::Shader,
};
use crate::device::{Device, MAX_FRAMES_IN_FLIGHT};
use ash::vk;
use resource_manager::{ResourceId, ResourcePool};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResourceManagerError {
    #[error("Invalid pipeline handle")]
    InvalidPipelineHandle,
    #[error("Resource not in cache")]
    ResourceNotInCache,
    #[error(transparent)]
    FallbackError(#[from] anyhow::Error),
}

pub enum Resource {
    GraphicsPipeline(GraphicsPipeline),
    RawPipeline(vk::Pipeline),
    Shader(Shader),
    Image(Image),
    Buffer(Buffer),
    Sampler(Sampler),
}

type DeleteQueue = Vec<Resource>;

macro_rules! insert {
    ($fn_name:ident,  $field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, res: $t) -> ResourceId<$t> {
            let id = self.$field.insert(res);
            id
        }
    };
}

macro_rules! delete {
    ($fn_name:ident, $fn_name_raw:ident, $field:ident, $t:ty, $k:ident) => {
        pub fn $fn_name(&mut self, handle: ResourceId<$t>, current_frame_index: usize) {
            if let Some(old) = self.$field.remove(handle) {
                self.to_consume[current_frame_index].push(Resource::$k(old));
            }
        }

        pub fn $fn_name_raw(&mut self, res: $t, current_frame_index: usize) {
            self.to_consume[current_frame_index].push(Resource::$k(res));
        }
    };
}

macro_rules! getter {
    ($fn_name:ident, $fn_name_mut:ident, $field:ident, $t:ty) => {
        pub fn $fn_name(&self, id: ResourceId<$t>) -> Option<&$t> {
            self.$field.get(id)
        }

        pub fn $fn_name_mut(&mut self, id: ResourceId<$t>) -> Option<&mut $t> {
            self.$field.get_mut(id)
        }
    };
}

// TODO: Do I want to queue delete on Drop?
// TODO: Auto-generate struct with proc-macro?
#[derive(Default)]
pub struct ResourceManager {
    graphics_pipelines: ResourcePool<GraphicsPipeline>,
    shaders: ResourcePool<Shader>,
    images: ResourcePool<Image>,
    buffers: ResourcePool<Buffer>,
    samplers: ResourcePool<Sampler>,
    to_consume: [DeleteQueue; MAX_FRAMES_IN_FLIGHT],
    consume_index: usize,
}

impl ResourceManager {
    pub fn force_destroy(&mut self, device: &Device) {
        for mut res in self.graphics_pipelines.drain() {
            res.destroy(device.raw());
        }
        for mut res in self.shaders.drain() {
            res.destroy(device.raw());
        }
        for mut res in self.images.drain() {
            res.destroy(device.raw());
        }
        for mut res in self.buffers.drain() {
            res.destroy(device.raw());
        }
        for mut res in self.samplers.drain() {
            res.destroy(device.raw());
        }
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            self.consume(device);
        }
    }

    pub fn consume(&mut self, device: &Device) {
        self.consume_index = (self.consume_index + 1) % MAX_FRAMES_IN_FLIGHT;
        for res in &mut self.to_consume[self.consume_index] {
            match res {
                Resource::GraphicsPipeline(pipeline) => pipeline.destroy(device.raw()),
                Resource::RawPipeline(pipeline) => unsafe {
                    device.raw().destroy_pipeline(*pipeline, None)
                },
                Resource::Shader(shader) => shader.destroy(device.raw()),
                Resource::Image(image) => image.destroy(device.raw()),
                Resource::Buffer(buffer) => buffer.destroy(device.raw()),
                Resource::Sampler(sampler) => sampler.destroy(device.raw()),
            }
        }
        self.to_consume[self.consume_index].clear();
    }

    pub fn replace_shader(
        &mut self,
        handle: ResourceId<Shader>,
        new: Shader,
        current_frame_index: usize,
    ) {
        if let Some(old) = self.shaders.replace(handle, new) {
            self.to_consume[current_frame_index].push(Resource::Shader(old));
        }
    }

    pub fn recreate_graphics_pipeline(
        &mut self,
        device: &Device,
        pipeline_handle: ResourceId<GraphicsPipeline>,
        vertex_handle: ResourceId<Shader>,
        fragment_handle: ResourceId<Shader>,
    ) -> Result<(), ResourceManagerError> {
        if let Some(old) = self.graphics_pipelines.get_mut(pipeline_handle) {
            let vertex_shader = self
                .shaders
                .get(vertex_handle)
                .ok_or(ResourceManagerError::ResourceNotInCache)?;
            let fragment_shader = self
                .shaders
                .get(fragment_handle)
                .ok_or(ResourceManagerError::ResourceNotInCache)?;
            let old_raw_pipeline = old
                .recreate(vertex_shader, fragment_shader, device)
                .map_err(ResourceManagerError::FallbackError)?;
            self.to_consume[device.current_frame_in_flight]
                .push(Resource::RawPipeline(old_raw_pipeline));
            Ok(())
        } else {
            Err(ResourceManagerError::InvalidPipelineHandle)
        }
    }

    // Insert
    insert!(
        insert_graphics_pipeline,
        graphics_pipelines,
        GraphicsPipeline
    );
    insert!(insert_shader, shaders, Shader);
    insert!(insert_image, images, Image);
    insert!(insert_buffer, buffers, Buffer);
    insert!(insert_sampler, samplers, Sampler);

    // Delete
    delete!(
        delete_graphics_pipeline,
        delete_graphics_pipeline_raw,
        graphics_pipelines,
        GraphicsPipeline,
        GraphicsPipeline
    );
    delete!(delete_shader, delete_shader_raw, shaders, Shader, Shader);
    delete!(delete_image, delete_image_raw, images, Image, Image);
    delete!(delete_buffer, delete_buffer_raw, buffers, Buffer, Buffer);
    delete!(
        delete_sampler,
        delete_sampler_raw,
        samplers,
        Sampler,
        Sampler
    );

    // Getters
    getter!(
        get_graphics_pipeline,
        get_graphics_pipeline_mut,
        graphics_pipelines,
        GraphicsPipeline
    );
    getter!(get_shader, get_shader_mut, shaders, Shader);
    getter!(get_image, get_image_mut, images, Image);
    getter!(get_buffer, get_buffer_mut, buffers, Buffer);
    getter!(get_sampler, get_sampler_mut, samplers, Sampler);
}
