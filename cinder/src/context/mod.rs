pub mod graphics_context;
pub mod upload_context;

pub struct BarrierDescription {}

pub trait Context {
    fn begin(&self);
    fn end(&self);
    fn resouce_barrier(&self, desc: BarrierDescription);
}
