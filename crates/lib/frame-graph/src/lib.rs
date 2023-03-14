use cinder::{
    resources::{image::Image, pipeline::graphics::GraphicsPipeline},
    ResourceHandle,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Debug, Hash)]
pub struct PassInfo {
    name: String,
    pipeline_handle: ResourceHandle<GraphicsPipeline>,
    input_image: ResourceHandle<Image>,
    depth_image: Option<ResourceHandle<Image>>,
    viewport_size: [u32; 2],
}

#[derive(Debug)]
pub struct Pass;

#[derive(Debug, Default)]
pub struct FrameGraph {
    pass_infos: Vec<(ResourceHandle<Pass>, PassInfo)>,
}

impl FrameGraph {
    pub fn add_pass(&mut self, pass_info: PassInfo) -> ResourceHandle<Pass> {
        let handle = {
            let mut hasher = DefaultHasher::new();
            pass_info.hash(&mut hasher);
            ResourceHandle::from_index(hasher.finish() as usize)
        };
        self.pass_infos.push((handle, pass_info));
        handle
    }

    pub fn build_graph(&self) -> ResourceHandle<FrameGraph> {
        // TODO
        ResourceHandle::from_index(0)
    }
}

#[cfg(test)]
mod tests {
    use super::FrameGraph;

    #[test]
    fn api() {
        let mut frame_graph = FrameGraph::default();
        // TODO: Actually hook up tests, will take a bit more work
    }
}
