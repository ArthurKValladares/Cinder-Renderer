#[cfg(feature = "metal")]
mod metal;
#[cfg(feature = "metal")]
pub use metal::*;

#[cfg(feature = "vulkan")]
mod vulkan;
#[cfg(feature = "vulkan")]
pub use vulkan::*;

#[cfg(feature = "empty")]
mod empty;
#[cfg(feature = "empty")]
pub use empty::*;

//

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Error)]
#[error("Not supported")]
pub struct ContextError;

pub trait Api: Sized {
    type Context: AsContext<Self>;
}

pub trait AsContext<A: Api>: Sized {
    fn init() -> Result<Self, ContextError>;
}
