use super::AsRendererContext;

pub struct RendererContext {}

impl AsRendererContext for RendererContext {
    fn create() -> Self {
        RendererContext {}
    }
}
