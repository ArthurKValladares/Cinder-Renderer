use super::{
    buffer::Buffer, image::Image, pipeline::graphics::GraphicsPipeline, sampler::Sampler,
    shader::Shader,
};
use crate::device::Device;
use ash::vk;
use resource_manager::{ResourceHandle, ResourcePool};

macro_rules! replace {
    ($fn_name:ident,  $field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, handle: ResourceHandle<$t>, res: $t) {
            self.$field.replace(handle, res)
        }
    };
}

macro_rules! insert {
    ($fn_name:ident,  $field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, res: $t) -> ResourceHandle<$t> {
            self.$field.insert(res)
        }
    };
}

macro_rules! remove {
    ($fn_name:ident,  $field:ident, $t:ty) => {
        pub fn $fn_name(&mut self, handle: ResourceHandle<$t>) -> Option<$t> {
            self.$field.remove(handle)
        }
    };
}

macro_rules! getter {
    ($fn_name:ident, $fn_name_mut:ident, $field:ident, $t:ty) => {
        pub fn $fn_name(&self, handle: ResourceHandle<$t>) -> Option<&$t> {
            self.$field.get(handle)
        }

        pub fn $fn_name_mut(&mut self, handle: ResourceHandle<$t>) -> Option<&mut $t> {
            self.$field.get_mut(handle)
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

#[derive(Default)]
pub struct ResourceManager {
    graphics_pipelines: ResourcePool<GraphicsPipeline>,
    shaders: ResourcePool<Shader>,
    images: ResourcePool<Image>,
    buffers: ResourcePool<Buffer>,
    samplers: ResourcePool<Sampler>,
    // TODO: better abstraction
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

    pub fn add_to_purgatory(&mut self, pipeline: vk::Pipeline) {
        self.purgatory.push(pipeline)
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

    // Replace
    replace!(
        replace_graphics_pipeline,
        graphics_pipelines,
        GraphicsPipeline
    );
    replace!(replace_shader, shaders, Shader);
    replace!(replace_image, images, Image);
    replace!(replace_buffer, buffers, Buffer);
    replace!(replace_sampler, samplers, Sampler);

    // Remove
    remove!(
        remove_graphics_pipeline,
        graphics_pipelines,
        GraphicsPipeline
    );
    remove!(remove_shader, shaders, Shader);
    remove!(remove_image, images, Image);
    remove!(remove_buffer, buffers, Buffer);
    remove!(remove_sampler, samplers, Sampler);

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
}
