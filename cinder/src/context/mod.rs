pub mod graphics_context;
pub mod upload_context;

use crate::device::Device;
use anyhow::Result;

pub struct BarrierDescription {}

pub trait Context {
    fn begin(&self, device: &Device) -> Result<()>;
    fn end(&self, device: &Device) -> Result<()>;
    fn resouce_barrier(&self, desc: BarrierDescription);
}
