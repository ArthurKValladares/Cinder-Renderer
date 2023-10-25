use crate::{
    command_queue::CommandQueue, device::Device, resources::ResourceManager,
    shader_hot_reloader::HotReloaderState, swapchain::Swapchain,
};
use anyhow::Result;
use bumpalo::Bump;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::time::Instant;

#[derive(Debug, PartialEq, Eq)]
enum FrameState {
    Running(Instant),
    NotRunning,
}

impl FrameState {
    pub fn is_running(&self) -> bool {
        match self {
            FrameState::Running(_) => true,
            FrameState::NotRunning => false,
        }
    }
}

pub struct Cinder {
    pub device: Device,
    pub swapchain: Swapchain,
    pub command_queue: CommandQueue,
    pub resource_manager: ResourceManager,
    pub shader_hot_reloader: HotReloaderState,
    init_time: Instant,
    frame_state: FrameState,
    last_dt: Option<u128>,
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
        let shader_hot_reloader = HotReloaderState::new()?;

        let init_time = Instant::now();

        Ok(Self {
            device,
            swapchain,
            command_queue,
            resource_manager,
            shader_hot_reloader,
            init_time,
            frame_state: FrameState::NotRunning,
            last_dt: None,
        })
    }

    pub fn init(&mut self) {
        take_mut::take(&mut self.shader_hot_reloader, |hot_reloader| {
            hot_reloader.run()
        });
    }

    pub fn init_time(&self) -> Instant {
        self.init_time
    }

    pub fn last_dt(&self) -> Option<u128> {
        self.last_dt
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.swapchain.resize(&self.device)?;
        Ok(())
    }

    pub fn start_frame(&mut self) -> Result<()> {
        debug_assert!(
            self.frame_state == FrameState::NotRunning,
            "Called `start_frame` twice before calling `end_frame`"
        );
        self.frame_state = FrameState::Running(Instant::now());

        self.device.new_frame()?;
        Ok(())
    }

    pub fn end_frame(&mut self) {
        assert!(
            self.frame_state.is_running(),
            "Called `end_frame` without calling `start_frame`"
        );
        match self.frame_state {
            FrameState::Running(frame_start) => {
                self.last_dt = Some(frame_start.elapsed().as_millis())
            }
            FrameState::NotRunning => unreachable!(),
        }
        self.frame_state = FrameState::NotRunning;

        self.resource_manager.consume(&self.device);
        self.device.bump_frame();
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
