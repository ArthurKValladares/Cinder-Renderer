use anyhow::Result;
use ash::vk;

pub struct Synch {
    current_semaphore: usize,
    present_done_semaphores: Vec<vk::Semaphore>,
    render_done_semaphores: Vec<vk::Semaphore>,
    last_image_rendered_semaphore: vk::Semaphore,
    last_image_acquired_semaphore: vk::Semaphore,
    swapchain_fences: Vec<vk::Fence>,
    current_fence: vk::Fence,
    completed_fence: vk::Fence,
}

impl Synch {
    pub fn new(device: &ash::Device, num_swapchain_images: usize) -> Result<Self> {
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        unsafe {
            let present_done_semaphores = (0..num_swapchain_images)
                .map(|_| device.create_semaphore(&semaphore_create_info, None))
                .collect::<Result<Vec<_>, _>>()?;

            let render_done_semaphores = (0..num_swapchain_images)
                .map(|_| device.create_semaphore(&semaphore_create_info, None))
                .collect::<Result<Vec<_>, _>>()?;

            let last_image_rendered_semaphore =
                device.create_semaphore(&semaphore_create_info, None)?;

            let last_image_acquired_semaphore =
                device.create_semaphore(&semaphore_create_info, None)?;

            let swapchain_fences = (0..num_swapchain_images)
                .map(|_| vk::Fence::null())
                .collect::<Vec<_>>();

            Ok(Self {
                current_semaphore: 0,
                present_done_semaphores,
                render_done_semaphores,
                last_image_rendered_semaphore,
                last_image_acquired_semaphore,
                swapchain_fences,
                current_fence: vk::Fence::null(),
                completed_fence: vk::Fence::null(),
            })
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            for semaphore in self.present_done_semaphores {
                device.destroy_semaphore(semaphore, None);
            }
            for semaphore in self.render_done_semaphores {
                device.destroy_semaphore(semaphore, None);
            }
            device.destroy_semaphore(self.last_image_rendered_semaphore, None);
            device.destroy_semaphore(self.last_image_acquired_semaphore, None);
        }
    }
}
