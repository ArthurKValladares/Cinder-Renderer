use crate::surface::{Surface, SurfaceData};
use anyhow::Result;
use ash::vk;

pub struct Swapchain {
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    // TODO: Should these and depth image be a `Texture`
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
}

fn create_swapchain_structures(
    instance: &ash::Instance,
    device: &ash::Device,
    surface: &Surface,
    surface_data: &SurfaceData,
    swapchain_loader: &ash::extensions::khr::Swapchain,
    old_swapchain: Option<vk::SwapchainKHR>,
) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>)> {
    let pre_transform = if surface_data
        .surface_capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        surface_data.surface_capabilities.current_transform
    };

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface.surface)
        .min_image_count(surface_data.desired_image_count)
        .image_color_space(surface_data.surface_format.color_space)
        .image_format(surface_data.surface_format.format)
        .image_extent(surface_data.surface_resolution)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(pre_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(surface_data.present_mode)
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
                .format(surface_data.surface_format.format)
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
            unsafe { device.create_image_view(&create_view_info, None) }
        })
        .collect::<Result<Vec<vk::ImageView>, ash::vk::Result>>()?;

    Ok((swapchain, present_images, present_image_views))
}

impl Swapchain {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        surface: &Surface,
        surface_data: &SurfaceData,
    ) -> Result<Self> {
        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);

        let (swapchain, present_images, present_image_views) = create_swapchain_structures(
            instance,
            device,
            surface,
            surface_data,
            &swapchain_loader,
            None,
        )?;

        Ok(Self {
            swapchain_loader,
            swapchain,
            present_images,
            present_image_views,
        })
    }

    pub fn resize(
        &mut self,
        instance: &ash::Instance,
        device: &ash::Device,
        surface: &Surface,
        surface_data: &SurfaceData,
    ) -> Result<()> {
        self.clean(device);

        let (swapchain, present_images, present_image_views) = create_swapchain_structures(
            instance,
            device,
            surface,
            surface_data,
            &self.swapchain_loader,
            Some(self.swapchain),
        )?;

        self.swapchain = swapchain;
        self.present_images = present_images;
        self.present_image_views = present_image_views;

        Ok(())
    }

    pub fn clean(&mut self, device: &ash::Device) {
        unsafe {
            for image_view in self.present_image_views.drain(..) {
                device.destroy_image_view(image_view, None);
            }
        }
    }
}
