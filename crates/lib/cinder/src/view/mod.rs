mod swapchain;

use self::swapchain::Swapchain;
use crate::device::{Device, SurfaceData};
use anyhow::Result;
use ash::vk;

#[derive(Debug, Clone, Copy)]
pub struct Drawable {
    pub(crate) image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) index: u32,
    pub(crate) is_suboptimal: bool,
}

pub struct View {
    swapchain: Swapchain,
}

impl View {
    pub fn new(device: &Device, surface_data: &SurfaceData) -> Result<Self> {
        let swapchain = Swapchain::new(device, surface_data)?;

        Ok(Self { swapchain })
    }

    pub fn get_current_drawable(&self, device: &Device) -> Result<Drawable> {
        let (index, is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                device.present_complete_semaphore(),
                vk::Fence::null(),
            )
        }?;

        Ok(Drawable {
            image: self.swapchain.get_image(index as usize),
            image_view: self.swapchain.get_image_view(index as usize),
            index,
            is_suboptimal,
        })
    }

    pub fn drawables_len(&self) -> usize {
        self.swapchain.present_image_views.len()
    }

    pub fn present(&self, device: &Device, drawable: Drawable) -> Result<bool> {
        Ok(unsafe {
            self.swapchain.swapchain_loader.queue_present(
                device.present_queue(),
                &vk::PresentInfoKHR::builder()
                    .wait_semaphores(std::slice::from_ref(&device.rendering_complete_semaphore))
                    .swapchains(std::slice::from_ref(&self.swapchain.swapchain))
                    .image_indices(std::slice::from_ref(&drawable.index))
                    .build(),
            )
        }?)
    }
}
