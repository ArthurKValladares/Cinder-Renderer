use crate::{
    command_queue::{set_image_memory_barrier, CommandList},
    device::Device,
};
use anyhow::Result;
use ash::vk;

type SwapchainStructures = (
    vk::SwapchainKHR,
    Vec<vk::Image>,
    Vec<vk::ImageView>,
    Vec<vk::ImageLayout>,
);

fn create_swapchain_structures(
    device: &Device,
    swapchain_loader: &ash::extensions::khr::Swapchain,
    old_swapchain: Option<vk::SwapchainKHR>,
) -> Result<SwapchainStructures> {
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
    device.set_name(vk::ObjectType::SWAPCHAIN_KHR, swapchain, "Swapchain");

    if let Some(old_swapchain) = old_swapchain {
        unsafe {
            swapchain_loader.destroy_swapchain(old_swapchain, None);
        }
    }

    let present_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;
    for (idx, image) in present_images.iter().enumerate() {
        device.set_name(
            vk::ObjectType::IMAGE,
            *image,
            &format!("Swapchain Image {idx}"),
        );
    }

    let present_image_views = present_images
        .iter()
        .enumerate()
        .map(|(idx, image)| {
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
                .image(*image);
            let image_view = unsafe { device.raw().create_image_view(&create_view_info, None) }?;
            device.set_name(
                vk::ObjectType::IMAGE_VIEW,
                image_view,
                &format!("Swapchain Image View {idx}"),
            );
            Ok(image_view)
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
    pub(crate) _image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) index: u32,
    pub(crate) _is_suboptimal: bool,
}

impl SwapchainImage {
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub struct Swapchain {
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    pub present_image_layouts: Vec<vk::ImageLayout>,
}

impl Swapchain {
    pub fn new(device: &Device) -> Result<Self> {
        let swapchain_loader =
            ash::extensions::khr::Swapchain::new(device.instance().raw(), device.raw());

        let (swapchain, present_images, present_image_views, present_image_layouts) =
            create_swapchain_structures(device, &swapchain_loader, None)?;

        let ret = Self {
            swapchain_loader,
            swapchain,
            present_images,
            present_image_views,
            present_image_layouts,
        };

        Ok(ret)
    }

    pub fn num_images(&self) -> usize {
        self.present_images.len()
    }

    pub fn acquire_image(
        &mut self,
        device: &Device,
        command_list: &CommandList,
    ) -> Result<SwapchainImage> {
        let (index, is_suboptimal) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                std::u64::MAX,
                device.image_acquired_semaphore(),
                vk::Fence::null(),
            )
        }?;

        let swapchain_image = SwapchainImage {
            index,
            _image: self.present_images[index as usize],
            image_view: self.present_image_views[index as usize],
            _is_suboptimal: is_suboptimal,
        };

        self.transition_image(device, command_list, swapchain_image);

        Ok(swapchain_image)
    }

    pub fn present(
        &mut self,
        device: &Device,
        cmd_list: CommandList,
        image: SwapchainImage,
    ) -> Result<bool> {
        self.transition_image(device, &cmd_list, image);

        cmd_list.end(device)?;

        let render_complete_fence = device.command_buffer_executed_fence();
        let render_complete_semaphore = [device.render_complete_semaphore()];

        let command_buffers = [cmd_list.buffer()];
        let wait_semaphores = [device.image_acquired_semaphore()];
        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .signal_semaphores(&render_complete_semaphore)
            .build();

        unsafe {
            device.raw().queue_submit(
                device.present_queue(),
                &[submit_info],
                render_complete_fence,
            )
        }?;

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&render_complete_semaphore)
            .swapchains(&[self.swapchain])
            .image_indices(&[image.index])
            .build();

        Ok(unsafe {
            self.swapchain_loader
                .queue_present(device.present_queue(), &present_info)
        }?)
    }

    fn transition_image(
        &mut self,
        device: &Device,
        command_list: &CommandList,
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
            command_list.buffer(),
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
        }
    }
}
