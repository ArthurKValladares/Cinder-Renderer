use super::AsRendererContext;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RendererContextInitError {}

pub struct RendererContext {}
pub struct FrameSubmitError {}

impl AsRendererContext for RendererContext {
    type CreateError = RendererContextInitError;
    type SubmitFrameError = FrameSubmitError;

    fn create(
        window: &winit::window::Window,
        init_dat: InitData,
    ) -> Result<Self, Self::CreateError> {
        Ok(RendererContext {})
    }

    fn submit_frame(&mut self, frame_number: FrameNumber) -> Result<(), Self::SubmitFrameError> {
        Ok(())
    }
}
