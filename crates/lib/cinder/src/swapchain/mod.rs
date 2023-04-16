use crate::{command_queue::set_image_memory_barrier, device::Device};
use anyhow::Result;
use ash::vk;

fn create_swapchain_structures(
    device: &Device,
    swapchain_loader: &ash::extensions::khr::Swapchain,
    old_swapchain: Option<vk::SwapchainKHR>,
) -> Result<(
    vk::SwapchainKHR,
    Vec<vk::Image>,
    Vec<vk::ImageView>,
    Vec<vk::ImageLayout>,
)> {
    let pre_transform = if device
        .surface_data
        .surface_capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        device.surface_data.surface_capabilities.current_transform
    };

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(device.surface().surface)
        .min_image_count(device.surface_data.desired_image_count)
        .image_color_space(device.surface_data.surface_format.color_space)
        .image_format(device.surface_data.surface_format.format)
        .image_extent(device.surface_data.surface_resolution)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(pre_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(device.surface_data.present_mode)
        .clipped(true)
        .image_array_layers(1)
        .old_swapchain(if let Some(old_swapchain) = old_swapchain {
            old_swapchain
        } else {
            vk::SwapchainKHR::null()
        });
    let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;

    if let Some(old_swapchain) = old_swapchain {
        unsafe {
            swapchain_loader.destroy_swapchain(old_swapchain, None);
        }
    }

    let present_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;
    let present_image_views = present_images
        .iter()
        .map(|&image| {
            let create_view_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(device.surface_data.surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::G,
                    b: vk::ComponentSwizzle::B,
                    a: vk::ComponentSwizzle::A,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(image);
            unsafe { device.raw().create_image_view(&create_view_info, None) }
        })
        .collect::<Result<Vec<vk::ImageView>, ash::vk::Result>>()?;

    let present_image_layouts = present_images
        .iter()
        .map(|_| vk::ImageLayout::UNDEFINED)
        .collect::<Vec<vk::ImageLayout>>();

    Ok((
        swapchain,
        present_images,
        present_image_views,
        present_image_layouts,
    ))
}

#[derive(Debug, Clone, Copy)]
pub struct SwapchainImage {
    pub(crate) image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) index: u32,
    pub(crate) is_suboptimal: bool,
    rendering_complete_semaphore: vk::Semaphore,
}

pub struct Swapchain {
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    pub present_image_layouts: Vec<vk::ImageLayout>,
    pub present_complete_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,
    name: Option<&'static str>,
}

impl Swapchain {
    pub fn new(device: &Device, name: Option<&'static str>) -> Result<Self> {
        let swapchain_loader =
            ash::extensions::khr::Swapchain::new(device.instance().raw(), device.raw());

        let (swapchain, present_images, present_image_views, present_image_layouts) =
            create_swapchain_structures(device, &swapchain_loader, None)?;

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphores = (0..present_images.len())
            .map(|_| unsafe { device.raw().create_semaphore(&semaphore_create_info, None) })
            .collect::<Result<Vec<_>, vk::Result>>()?;

        let rendering_complete_semaphores = (0..present_images.len())
            .map(|_| unsafe { device.raw().create_semaphore(&semaphore_create_info, None) })
            .collect::<Result<Vec<_>, vk::Result>>()?;

        let ret = Self {
            swapchain_loader,
            swapchain,
            present_images,
            present_image_views,
            present_image_layouts,
            present_complete_semaphores,
            rendering_complete_semaphores,
            name,
        };

        Ok(ret)
    }

    pub fn acquire_image(&self, device: &Device) -> Result<SwapchainImage> {
        let semaphore_index = device.frame_index() % self.present_complete_semaphores.len();
        let present_complete_semaphore = self.present_complete_semaphores[semaphore_index];
        let rendering_complete_semaphore = self.rendering_complete_semaphores[semaphore_index];

        let (index, is_suboptimal) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                std::u64::MAX,
                present_complete_semaphore,
                vk::Fence::null(),
            )
        }?;

        Ok(SwapchainImage {
            index,
            image: self.present_images[index as usize],
            image_view: self.present_image_views[index as usize],
            is_suboptimal,
            rendering_complete_semaphore,
        })
    }

    pub fn present(&self, device: &Device, image: SwapchainImage) -> Result<bool> {
        Ok(unsafe {
            self.swapchain_loader.queue_present(
                device.present_queue(),
                &vk::PresentInfoKHR::builder()
                    .wait_semaphores(std::slice::from_ref(&image.rendering_complete_semaphore))
                    .swapchains(std::slice::from_ref(&self.swapchain))
                    .image_indices(std::slice::from_ref(&image.index))
                    .build(),
            )
        }?)
    }

    pub fn transition_image(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        swapchain_image: SwapchainImage,
    ) {
        let layout = &mut self.present_image_layouts[swapchain_image.index as usize];

        let to_present = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL == *layout;

        let new_layout = if to_present {
            vk::ImageLayout::PRESENT_SRC_KHR
        } else {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        };

        *layout = if to_present {
            *layout
        } else {
            vk::ImageLayout::UNDEFINED
        };

        set_image_memory_barrier(
            device.raw(),
            command_buffer,
            self.present_images[swapchain_image.index as usize],
            vk::ImageAspectFlags::COLOR,
            *layout,
            new_layout,
            Default::default(),
        );

        *layout = new_layout;
    }

    pub fn resize(&mut self, device: &Device) -> Result<()> {
        self.clean_images(device.raw());

        let (swapchain, present_images, present_image_views, present_image_layouts) =
            create_swapchain_structures(device, &self.swapchain_loader, Some(self.swapchain))?;

        self.swapchain = swapchain;
        self.present_images = present_images;
        self.present_image_views = present_image_views;
        self.present_image_layouts = present_image_layouts;

        Ok(())
    }

    fn clean_images(&mut self, device: &ash::Device) {
        unsafe {
            for image_view in self.present_image_views.drain(..) {
                device.destroy_image_view(image_view, None);
            }
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        self.clean_images(device.raw());
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            for semaphore in &self.rendering_complete_semaphores {
                device.raw().destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.present_complete_semaphores {
                device.raw().destroy_semaphore(*semaphore, None);
            }
        }
    }
}
