// TODO: No `vk` deps here
use anyhow::Result;
use cinder::{
    context::{
        render_context::{RenderContext, RenderContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
    },
    device::Device,
    instance::Instance,
    profiling::Profiling,
    resoruces::{
        buffer::vk,
        image::{Format, Image, ImageDescription, ImageViewDescription, Usage},
        pipeline::PipelineCache,
    },
    surface::{Surface, SurfaceData},
    swapchain::Swapchain,
    InitData,
};
use math::{rect::Rect2D, size::Size2D};

use crate::depth_pyramid::{self, DepthPyramid};

pub struct Renderer {
    init_data: InitData,
    _instance: Instance,
    device: Device,
    surface: Surface,
    swapchain: Swapchain,
    surface_data: SurfaceData,
    pipeline_cache: PipelineCache,

    pub depth_image: Image,
    depth_pyramid: DepthPyramid,
    command_pool: vk::CommandPool,

    // TODO: Probably will have better syncronization in the future
    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,

    pub profiling: Profiling,
}

impl Renderer {
    pub fn new(window: &winit::window::Window, init_data: InitData) -> Result<Self> {
        let instance = Instance::new(window)?;

        let surface = Surface::new(window, &instance)?;

        let device = Device::new(&instance, &surface)?;

        let surface_data = surface.get_data(
            device.p_device(),
            init_data.backbuffer_resolution,
            init_data.vsync,
        )?;

        let swapchain = Swapchain::new(&instance, &device, &surface, &surface_data)?;

        let pipeline_cache = PipelineCache::new(&device)?;

        let mut depth_image = Image::create(
            &device,
            device.memopry_properties(),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: Usage::Depth,
                size: Size2D::new(
                    surface_data.surface_resolution.width,
                    surface_data.surface_resolution.height,
                ),
            },
        )?;
        depth_image.add_view(
            &device,
            ImageViewDescription {
                format: Format::D32_SFloat,
                usage: Usage::Depth,
            },
        )?;
        let depth_pyramid = DepthPyramid::create(&device, surface_data.size())?;

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore =
            unsafe { device.raw().create_semaphore(&semaphore_create_info, None) }?;
        let rendering_complete_semaphore =
            unsafe { device.raw().create_semaphore(&semaphore_create_info, None) }?;

        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let draw_commands_reuse_fence =
            unsafe { device.raw().create_fence(&fence_create_info, None) }?;
        let setup_commands_reuse_fence =
            unsafe { device.raw().create_fence(&fence_create_info, None) }?;

        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(device.queue_family_index());

        let command_pool = unsafe { device.raw().create_command_pool(&pool_create_info, None) }?;

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
            depth_pyramid,
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

    pub fn pipeline_cache(&self) -> PipelineCache {
        self.pipeline_cache
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

    pub fn surface_format(&self) -> Format {
        self.surface_data.surface_format.format.into()
    }

    pub fn create_render_context(&self, _desc: RenderContextDescription) -> Result<RenderContext> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.device
                .raw()
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
                .raw()
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
        self.surface_data.size()
    }

    pub fn surface_rect(&self) -> Rect2D<i32, u32> {
        Rect2D::from_width_height(
            self.surface_data.surface_resolution.width,
            self.surface_data.surface_resolution.height,
        )
    }

    pub fn resize(&mut self, backbuffer_resolution: Size2D<u32>) -> Result<()> {
        unsafe {
            self.device.raw().device_wait_idle()?;

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
                self.device.memopry_properties(),
                ImageDescription {
                    format: Format::D32_SFloat,
                    usage: Usage::Depth,
                    size: backbuffer_resolution,
                },
            )?;
            self.depth_image.add_view(
                &self.device,
                ImageViewDescription {
                    format: Format::D32_SFloat,
                    usage: Usage::Depth,
                },
            )?;
            self.depth_pyramid.resize(&self.device, self.surface_size());
        }

        Ok(())
    }
}
