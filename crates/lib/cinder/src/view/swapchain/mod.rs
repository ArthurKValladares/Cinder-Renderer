use crate::device::Device;
use anyhow::Result;
use ash::vk;

fn create_swapchain_structures(
    device: &Device,
    swapchain_loader: &ash::extensions::khr::Swapchain,
    old_swapchain: Option<vk::SwapchainKHR>,
) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>)> {
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

    Ok((swapchain, present_images, present_image_views))
}

pub struct Swapchain {
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    // TODO: Should these be `Image`s (yes)
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    name: Option<&'static str>,
}

impl Swapchain {
    pub fn new(device: &Device, name: Option<&'static str>) -> Result<Self> {
        let swapchain_loader =
            ash::extensions::khr::Swapchain::new(device.instance().raw(), device.raw());

        let (swapchain, present_images, present_image_views) =
            create_swapchain_structures(device, &swapchain_loader, None)?;

        let ret = Self {
            swapchain_loader,
            swapchain,
            present_images,
            present_image_views,
            name,
        };

        if let Some(name) = name {
            ret.set_name(device, name);
        }

        Ok(ret)
    }

    pub fn resize(&mut self, device: &Device) -> Result<()> {
        self.clean_images(device.raw());

        let (swapchain, present_images, present_image_views) =
            create_swapchain_structures(device, &self.swapchain_loader, Some(self.swapchain))?;

        self.swapchain = swapchain;
        self.present_images = present_images;
        self.present_image_views = present_image_views;

        if let Some(name) = self.name {
            self.set_name(device, name);
        }

        Ok(())
    }

    pub fn get_image(&self, index: usize) -> vk::Image {
        self.present_images[index]
    }

    pub fn get_image_view(&self, index: usize) -> vk::ImageView {
        self.present_image_views[index]
    }

    fn clean_images(&mut self, device: &ash::Device) {
        unsafe {
            for image_view in self.present_image_views.drain(..) {
                device.destroy_image_view(image_view, None);
            }
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        self.clean_images(device);
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }

    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(vk::ObjectType::SWAPCHAIN_KHR, self.swapchain, name);

        for (idx, image) in self.present_images.iter().enumerate() {
            device.set_name(
                vk::ObjectType::IMAGE,
                *image,
                &format!("{} [image {}]", name, idx),
            );
        }
        for (idx, image_view) in self.present_image_views.iter().enumerate() {
            device.set_name(
                vk::ObjectType::IMAGE_VIEW,
                *image_view,
                &format!("{} [image view {}]", name, idx),
            );
        }
    }
}
