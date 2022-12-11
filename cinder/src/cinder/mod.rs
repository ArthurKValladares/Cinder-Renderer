use crate::{
    context::{
        render_context::{RenderContext, RenderContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
    },
    device::Device,
    instance::Instance,
    profiling::Profiling,
    resoruces::{
        buffer::{Buffer, BufferDescription},
        image::{self, Image, ImageDescription},
        pipeline::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        shader::{Shader, ShaderDescription},
    },
    surface::{Surface, SurfaceData},
    swapchain::Swapchain,
    InitData,
};
use anyhow::Result;
use ash::vk;
use math::{rect::Rect2D, size::Size2D};
use tracing::{span, Level};

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/default_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/egui_shader_structs.rs"
));

pub const RESERVED_DESCRIPTOR_COUNT: u32 = 32;

pub struct Cinder {
    init_data: InitData,
    _instance: Instance,
    device: Device,
    surface: Surface,
    swapchain: Swapchain,
    surface_data: SurfaceData,
    pipeline_cache: vk::PipelineCache,

    pub depth_image: Image,
    command_pool: vk::CommandPool,

    // TODO: Probably will have better syncronization in the future
    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,

    pub profiling: Profiling,
}

impl Cinder {
    pub fn new(window: &winit::window::Window, init_data: InitData) -> Result<Self> {
        let span = span!(Level::DEBUG, "Cinder::new");
        let _enter = span.enter();

        let instance = Instance::new(window)?;

        let surface = Surface::new(window, &instance)?;

        let device = Device::new(&instance, &surface)?;

        let surface_data = surface.get_data(
            device.p_device(),
            init_data.backbuffer_resolution,
            init_data.vsync,
        )?;

        let swapchain = Swapchain::new(&instance, &device, &surface, &surface_data)?;

        let pipeline_cache = {
            let ci = vk::PipelineCacheCreateInfo::builder().build();
            unsafe { device.create_pipeline_cache(&ci, None)? }
        };

        let depth_image = Image::create(
            &device,
            &device.memopry_properties(),
            ImageDescription {
                format: image::Format::D32_SFloat,
                usage: image::Usage::Depth,
                size: Size2D::new(
                    surface_data.surface_resolution.width,
                    surface_data.surface_resolution.height,
                ),
            },
        )?;

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        let rendering_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;

        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let draw_commands_reuse_fence = unsafe { device.create_fence(&fence_create_info, None) }?;
        let setup_commands_reuse_fence = unsafe { device.create_fence(&fence_create_info, None) }?;

        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(device.queue_family_index());

        let command_pool = unsafe { device.create_command_pool(&pool_create_info, None) }?;

        let profiling = Profiling::new(&device)?;

        Ok(Self {
            init_data,
            _instance: instance,
            device,
            surface,
            swapchain,
            surface_data,
            pipeline_cache,
            depth_image,
            present_complete_semaphore,
            rendering_complete_semaphore,
            draw_commands_reuse_fence,
            setup_commands_reuse_fence,
            command_pool,
            profiling,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn swapchain(&self) -> &Swapchain {
        &self.swapchain
    }

    pub fn depth_image(&self) -> &Image {
        &self.depth_image
    }

    pub fn setup_fence(&self) -> vk::Fence {
        self.setup_commands_reuse_fence
    }

    pub fn draw_fence(&self) -> vk::Fence {
        self.draw_commands_reuse_fence
    }

    pub fn present_semaphore(&self) -> vk::Semaphore {
        self.present_complete_semaphore
    }

    pub fn render_semaphore(&self) -> vk::Semaphore {
        self.rendering_complete_semaphore
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.device.present_queue()
    }

    pub fn surface_format(&self) -> vk::Format {
        self.surface_data.surface_format.format
    }

    pub fn create_buffer(&self, desc: BufferDescription) -> Result<Buffer> {
        Buffer::create(&self.device, desc)
    }

    pub fn create_image(&self, desc: ImageDescription) -> Result<Image> {
        Image::create(&self.device, self.device.memopry_properties(), desc)
    }

    pub fn create_sampler(&self) -> Result<Sampler> {
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            ..Default::default()
        };

        let sampler = unsafe { self.device.create_sampler(&sampler_info, None) }?;

        Ok(Sampler { raw: sampler })
    }

    pub fn create_shader(&self, desc: ShaderDescription) -> Result<Shader> {
        Shader::create(&self.device, desc)
    }

    pub fn create_graphics_pipeline(
        &self,
        desc: GraphicsPipelineDescription,
    ) -> Result<GraphicsPipeline> {
        GraphicsPipeline::create(
            &self.device,
            self.surface_format(),
            self.depth_image.desc.format.into(),
            self.pipeline_cache,
            desc,
        )
    }

    pub fn create_render_context(&self, _desc: RenderContextDescription) -> Result<RenderContext> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        }[0];

        Ok(RenderContext::from_command_buffer(command_buffer))
    }

    pub fn create_upload_context(&self, _desc: UploadContextDescription) -> Result<UploadContext> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        }[0];

        Ok(UploadContext::from_command_buffer(command_buffer))
    }

    pub fn present(&self, present_index: u32) -> Result<bool> {
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(&self.rendering_complete_semaphore))
            .swapchains(std::slice::from_ref(&self.swapchain.swapchain))
            .image_indices(std::slice::from_ref(&present_index))
            .build();

        let is_suboptimal = unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(self.device.present_queue(), &present_info)
        }?;
        Ok(is_suboptimal)
    }

    // TODO: probably should totally abstract this from user code
    pub fn acquire_next_image(&self) -> Result<(u32, bool)> {
        let (present_index, is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.present_complete_semaphore,
                vk::Fence::null(),
            )
        }?;
        Ok((present_index, is_suboptimal))
    }

    pub fn surface_size(&self) -> Size2D<u32> {
        Size2D::new(
            self.surface_data.surface_resolution.width,
            self.surface_data.surface_resolution.height,
        )
    }

    pub fn surface_rect(&self) -> Rect2D<i32, u32> {
        Rect2D::from_width_height(
            self.surface_data.surface_resolution.width,
            self.surface_data.surface_resolution.height,
        )
    }

    pub fn resize(&mut self, backbuffer_resolution: Size2D<u32>) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()?;

            self.surface_data = self.surface.get_data(
                self.device.p_device(),
                backbuffer_resolution,
                self.init_data.vsync,
            )?;
            self.swapchain
                .resize(&self.device, &self.surface, &self.surface_data)?;
            self.depth_image.clean(&self.device);
            self.depth_image = Image::create(
                &self.device,
                &self.device.memopry_properties(),
                ImageDescription {
                    format: image::Format::D32_SFloat,
                    usage: image::Usage::Depth,
                    size: backbuffer_resolution,
                },
            )?;
        }

        Ok(())
    }
}
