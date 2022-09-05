use super::AsRendererContext;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RendererContextInitError {}

pub struct RendererContext {}

impl AsRendererContext for RendererContext {
    type CreateError = RendererContextInitError;

    fn create(
        window: &winit::window::Window,
        init_dat: InitData,
    ) -> Result<Self, Self::CreateError> {
        Ok(RendererContext {})
    }
}
