use super::{
    buffer::Buffer, image::Image, pipeline::graphics::GraphicsPipeline, sampler::Sampler,
    shader::Shader,
};
use crate::device::Device;
use ash::vk;
use resource_manager::{ResourceId, ResourcePool};
use std::{fmt::Debug, sync::Arc};

pub struct ResourceHandle<T>(Arc<ResourceId<T>>);

impl<T> ResourceHandle<T> {
    pub fn id(&self) -> ResourceId<T> {
        *self.0
    }
}

macro_rules! insert {
    ($fn_name:ident,  $field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, res: $t) -> ResourceHandle<$t> {
            let id = self.$field.insert(res);
            ResourceHandle(Arc::new(id))
        }
    };
}

macro_rules! delete {
    ($fn_name:ident, $to_remove_field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, handle: ResourceHandle<$t>) {
            self.$to_remove_field.push(handle);
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

macro_rules! cleanup {
    ($self:ident.$field:ident, $device:ident) => {
        for mut res in $self.$field.drain() {
            res.destroy($device.raw());
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
    // TODO: I don't like this, but it's ok for proof of concept atm
    to_delete_graphics_pipelines: Vec<ResourceHandle<GraphicsPipeline>>,
    to_delete_shaders: Vec<ResourceHandle<Shader>>,
    to_delete_images: Vec<ResourceHandle<Image>>,
    to_delete_buffers: Vec<ResourceHandle<Buffer>>,
    to_delete_samplers: Vec<ResourceHandle<Sampler>>,
    // TODO: Purgatory implementation here pretty sloppy
    purgatory: Vec<vk::Pipeline>,
}

impl ResourceManager {
    pub fn clean(&mut self, device: &Device) {
        cleanup!(self.graphics_pipelines, device);
        cleanup!(self.shaders, device);
        cleanup!(self.images, device);
        cleanup!(self.buffers, device);
        cleanup!(self.samplers, device);
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
        to_delete_graphics_pipelines,
        GraphicsPipeline
    );
    delete!(delete_shader, to_delete_shaders, Shader);
    delete!(delete_image, to_delete_images, Image);
    delete!(delete_buffer, to_delete_buffers, Buffer);
    delete!(delete_sampler, to_delete_samplers, Sampler);

    // Get
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

    // TODO: Temp pipeline-specific functions
    pub fn remove_graphics_pipeline(
        &mut self,
        id: ResourceId<GraphicsPipeline>,
    ) -> Option<GraphicsPipeline> {
        self.graphics_pipelines.remove(id)
    }

    pub fn replace_graphics_pipeline(
        &mut self,
        id: ResourceId<GraphicsPipeline>,
        res: GraphicsPipeline,
    ) {
        self.graphics_pipelines.replace(id, res)
    }

    pub fn replace_shader(&mut self, id: ResourceId<Shader>, res: Shader) {
        self.shaders.replace(id, res)
    }

    pub fn add_pipeline_to_purgatory(&mut self, pipeline: vk::Pipeline) {
        self.purgatory.push(pipeline)
    }
}

impl<T> Debug for ResourceHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceHandle")
            .field("id", &self.0)
            .finish()
    }
}

impl<T> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
