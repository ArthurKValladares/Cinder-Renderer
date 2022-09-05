#[cfg(feature = "metal")]
pub mod metal;
#[cfg(feature = "metal")]
pub use metal as back;

#[cfg(feature = "vulkan")]
pub mod vulkan;
#[cfg(feature = "vulkan")]
pub use vulkan as back;

#[cfg(not(any(feature = "metal", feature = "vulkan")))]
pub mod empty;
#[cfg(not(any(feature = "metal", feature = "vulkan")))]
pub use empty as back;

use crate::{context::FrameNumber, init::InitData};

pub trait AsRendererContext: Sized {
    type CreateError;
    fn create(
        window: &winit::window::Window,
        init_dat: InitData,
    ) -> Result<Self, Self::CreateError>;

    fn submit_frame(&mut self, frame_number: FrameNumber);
}
