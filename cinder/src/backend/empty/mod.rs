use crate::Cinder;

use super::AsContext;

pub struct BackendContext {}

impl AsContext<Cinder> for BackendContext {
    fn init() -> Result<Self, super::ContextError> {
        Ok(BackendContext {})
    }
}
