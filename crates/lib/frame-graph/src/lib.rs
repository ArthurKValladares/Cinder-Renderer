use cinder::resources::{buffer::BufferUsage, image::Format};
use math::size::Size3D;
use thiserror::Error;

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

#[derive(Debug, Error)]
pub enum PassError {
    #[error("size of color inputs must match color outputs")]
    ColorAttachmentsMismatch,
}

#[derive(Debug)]
pub struct Pass<'a> {
    name: &'a str,
    color_inputs: Vec<PassResourceHandle>,
    color_outputs: Vec<PassResource>,
}

impl<'a> Pass<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            color_inputs: Default::default(),
            color_outputs: Default::default(),
        }
    }

    pub fn extend_color_inputs(&mut self, inputs: impl IntoIterator<Item = PassResourceHandle>) {
        self.color_inputs.extend(inputs)
    }

    pub fn add_color_input(&mut self, res: PassResourceHandle) {
        self.color_inputs.push(res)
    }

    pub fn extend_color_outputs(&mut self, outputs: impl IntoIterator<Item = PassResource>) {
        self.color_outputs.extend(outputs)
    }

    pub fn add_color_output(&mut self, handle: PassResource) {
        self.color_outputs.push(handle)
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

    fn validate_passes(&self) -> Result<(), PassError> {
        for pass in &self.passes {
            if pass.color_inputs.len() != pass.color_outputs.len() {
                return Err(PassError::ColorAttachmentsMismatch);
            }
        }
        Ok(())
    }

    pub fn bake(self) -> Result<(), PassError> {
        self.validate_passes()?;

        Ok(())
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

        let mut g_buffer_pass = Pass::new("g_buffer");
        g_buffer_pass.extend_color_outputs(vec![
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
        ]);
        frame_graph.add_pass(g_buffer_pass);

        let mut lighting_pass = Pass::new("lighting");
        lighting_pass.extend_color_inputs(vec![albedo_handle, depth_handle]);
        frame_graph.add_pass(lighting_pass);

        let bake_result = frame_graph.bake();
        assert!(bake_result.is_ok(), "Error: {:?}", bake_result);
    }
}
