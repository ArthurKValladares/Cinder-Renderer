use super::Context;
use crate::resoruces::{buffer::Buffer, pipeline::Pipeline};

pub struct GraphicsContextDescription {}

pub struct GraphicsContext {}

impl Context for GraphicsContext {
    fn begin(&self) {}

    fn end(&self) {}

    fn resouce_barrier(&self, desc: super::BarrierDescription) {}
}

impl GraphicsContext {
    pub fn set_pipeline(&self, pipeline: &Pipeline) {}

    pub fn set_vertex_buffer(&self, buffer: Buffer) {}

    pub fn set_index_buffer(&self, buffer: Buffer) {}

    pub fn draw(&self) {}
}
