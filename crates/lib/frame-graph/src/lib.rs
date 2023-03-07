use cinder::resources::{buffer::BufferUsage, image::Format};
use math::size::Size3D;

#[derive(Debug, Copy, Clone)]
pub struct PassResourceHandle(usize);

impl PassResourceHandle {
    // TOOD: temp for testing
    pub fn new(inner: usize) -> Self {
        Self(inner)
    }
}

#[derive(Debug)]
pub enum SizeClass {
    Absolute(Size3D<u32>),
    SwapchainRelative(Size3D<f32>),
    InputRelative(Size3D<f32>),
}

#[derive(Debug)]
pub struct ImageInfo {
    pub handle: PassResourceHandle,
    pub size_class: SizeClass,
    pub format: Format,
    pub persistent: bool,
}

#[derive(Debug)]
pub struct BufferInfo {
    pub handle: PassResourceHandle,
    pub size: usize,
    pub usage: BufferUsage,
    pub persistent: bool,
}

#[derive(Debug)]
pub enum PassResource {
    Image(ImageInfo),
    Buffer(BufferInfo),
}

#[derive(Debug)]
pub struct Pass<'a> {
    name: &'a str,
    inputs: Vec<PassResourceHandle>,
    outputs: Vec<PassResource>,
}

impl<'a> Pass<'a> {
    pub fn new<I, O>(name: &'a str, inputs: I, outputs: O) -> Self
    where
        I: IntoIterator<Item = PassResourceHandle>,
        O: IntoIterator<Item = PassResource>,
    {
        Self {
            name,
            inputs: inputs.into_iter().collect(),
            outputs: outputs.into_iter().collect(),
        }
    }
}

#[derive(Debug)]
pub struct FrameGraph<'a> {
    name: &'a str,
    passes: Vec<Pass<'a>>,
}

impl<'a> FrameGraph<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            passes: Default::default(),
        }
    }

    pub fn with_passes(name: &'a str, passes: impl IntoIterator<Item = Pass<'a>>) -> Self {
        Self {
            name,
            passes: passes.into_iter().collect(),
        }
    }

    pub fn add_pass(&mut self, pass: Pass<'a>) {
        self.passes.push(pass);
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameGraph, ImageInfo, Pass, PassResource, PassResourceHandle, SizeClass};
    use cinder::resources::image::Format;
    use math::size::Size3D;

    #[test]
    fn api() {
        let mut frame_graph = FrameGraph::new("frame_graph");

        let albedo_handle = PassResourceHandle::new(0);
        let depth_handle = PassResourceHandle::new(1);
        frame_graph.add_pass(Pass::new(
            "g_buffer",
            vec![],
            vec![
                PassResource::Image(ImageInfo {
                    handle: albedo_handle,
                    size_class: SizeClass::SwapchainRelative(Size3D::new(1.0, 1.0, 1.0)),
                    format: Format::R8G8B8A8_Unorm,
                    persistent: false,
                }),
                PassResource::Image(ImageInfo {
                    handle: depth_handle,
                    size_class: SizeClass::SwapchainRelative(Size3D::new(1.0, 1.0, 1.0)),
                    format: Format::D32_SFloat,
                    persistent: false,
                }),
            ],
        ));
        frame_graph.add_pass(Pass::new(
            "lighting",
            vec![albedo_handle, depth_handle],
            vec![],
        ));
    }
}
