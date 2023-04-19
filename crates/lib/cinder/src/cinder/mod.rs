use crate::{
    command_queue::CommandQueue, device::Device, resources::ResourceManager, swapchain::Swapchain,
};
use anyhow::Result;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::time::Instant;

pub struct Cinder {
    pub device: Device,
    pub swapchain: Swapchain,
    pub command_queue: CommandQueue,
    pub resource_manager: ResourceManager,
    pub init_time: Instant,
}

impl Cinder {
    pub fn new<W>(window: &W, window_width: u32, window_height: u32) -> Result<Self>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let device = Device::new(window, window_width, window_height)?;
        let command_queue = CommandQueue::new(&device)?;
        let swapchain = Swapchain::new(&device)?;
        let resource_manager = ResourceManager::default();

        let init_time = Instant::now();

        Ok(Self {
            device,
            swapchain,
            command_queue,
            resource_manager,
            init_time,
        })
    }
}

impl Drop for Cinder {
    fn drop(&mut self) {
        self.device.wait_idle().ok();
        self.command_queue.destroy(&self.device);
        self.swapchain.destroy(&self.device);
        self.resource_manager.force_destroy(&self.device);
    }
}
