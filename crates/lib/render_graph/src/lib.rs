use anyhow::{Ok, Result};
use bumpalo::{collections::Vec as BumpVec, Bump};
use cinder::{
    command_queue::{CommandList, RenderAttachment, RenderAttachmentDesc},
    resources::image::Image,
    swapchain::SwapchainImage,
    Cinder,
};
use math::rect::Rect2D;
use resource_manager::ResourceId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RenderPassId(usize);

#[derive(Debug)]
pub struct RenderGraphNode<'a> {
    input_nodes: BumpVec<'a, RenderPassId>,
    output_nodes: BumpVec<'a, RenderPassId>,
}

impl<'a> RenderGraphNode<'a> {
    pub fn new(bump: &'a Bump) -> Self {
        Self {
            input_nodes: BumpVec::new_in(bump),
            output_nodes: BumpVec::new_in(bump),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderPassResource {
    SwapchainImage,
    Image(ResourceId<Image>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum AttachmentType {
    SwapchainImage,
    Reference(ResourceId<Image>),
}

type RenderPassCallback<'a> = dyn Fn(&Cinder, &CommandList) -> Result<()> + 'a;

pub struct RenderPass<'a> {
    color_attachments: HashMap<AttachmentType, RenderAttachmentDesc>,
    depth_attachment: Option<(AttachmentType, RenderAttachmentDesc)>,
    inputs: BumpVec<'a, RenderPassResource>,
    outputs: BumpVec<'a, RenderPassResource>,
    render_area: Option<Rect2D<i32, u32>>,
    flipped_viewport: bool,
    callback: Box<RenderPassCallback<'a>>,
    name: Option<&'a String>,
}

impl<'a> std::fmt::Debug for RenderPass<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderPass")
            .field("color_attachments", &self.color_attachments)
            .field("depth_attachment", &self.depth_attachment)
            .field("inputs", &self.inputs)
            .field("outputs", &self.outputs)
            .field("render_area", &self.render_area)
            .field("flipped_viewport", &self.flipped_viewport)
            .field("name", &self.name)
            .finish()
    }
}

impl<'a> RenderPass<'a> {
    pub fn new(bump: &'a Bump) -> Self {
        Self {
            color_attachments: Default::default(),
            depth_attachment: Default::default(),
            inputs: BumpVec::new_in(bump),
            outputs: BumpVec::new_in(bump),
            render_area: None,
            flipped_viewport: true,
            callback: Box::new(|_, _| Ok(())),
            name: None,
        }
    }

    pub fn add_color_attachment(
        mut self,
        attachment: impl Into<AttachmentType>,
        desc: RenderAttachmentDesc,
    ) -> Self {
        self.color_attachments.insert(attachment.into(), desc);
        self
    }

    pub fn set_depth_attachment(
        mut self,
        attachment: impl Into<AttachmentType>,
        desc: RenderAttachmentDesc,
    ) -> Self {
        self.depth_attachment = Some((attachment.into(), desc));
        self
    }

    pub fn with_render_area(mut self, render_area: Rect2D<i32, u32>) -> Self {
        self.render_area = Some(render_area);
        self
    }

    pub fn with_flipped_viewport(mut self, flipped: bool) -> Self {
        self.flipped_viewport = flipped;
        self
    }

    pub fn add_input(mut self, input: RenderPassResource) -> Self {
        self.inputs.push(input);
        self
    }

    pub fn add_output(mut self, output: RenderPassResource) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn set_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Cinder, &CommandList) -> Result<()> + 'a,
    {
        self.callback = Box::new(callback);
        self
    }
}

#[derive(Debug)]
pub struct PresentContext {
    pub present_rect: Rect2D<i32, u32>,
    pub cmd_list: CommandList,
    pub swapchain_image: SwapchainImage,
}

impl PresentContext {
    pub fn present(self, cinder: &mut Cinder) -> Result<bool> {
        let ret = cinder
            .swapchain
            .present(&cinder.device, self.cmd_list, self.swapchain_image);
        cinder.device.end_queue_label();
        ret
    }
}

#[derive(Debug)]
pub struct RenderGraph<'a> {
    passes: BumpVec<'a, RenderPass<'a>>,
    // Instead of a set, could maybe be a vector of bool
    input_map: HashMap<RenderPassResource, HashSet<RenderPassId>>,
    output_map: HashMap<RenderPassResource, HashSet<RenderPassId>>,
}

impl<'a> RenderGraph<'a> {
    pub fn new(bump: &'a Bump) -> Self {
        Self {
            passes: BumpVec::new_in(bump),
            input_map: Default::default(),
            output_map: Default::default(),
        }
    }

    pub fn add_pass(&mut self, pass: RenderPass<'a>) {
        let id = RenderPassId(self.passes.len());
        for input in &pass.inputs {
            self.input_map.entry(*input).or_default().insert(id);
        }
        for output in &pass.outputs {
            self.output_map.entry(*output).or_default().insert(id);
        }
        self.passes.push(pass)
    }

    fn compile_nodes<'b>(&mut self, bump: &'b Bump) -> HashMap<RenderPassId, RenderGraphNode<'b>> {
        // TODO: since  `RenderPassId` is just an int, this can be a BumpVec too
        let mut nodes: HashMap<RenderPassId, RenderGraphNode> = Default::default();
        for (idx, pass) in self.passes.iter().enumerate() {
            let mut node = RenderGraphNode::new(bump);

            // If an input of this node is used as an output by another node, then
            // that node must have an edge pointing to this node.
            for input in &pass.inputs {
                if let Some(uses_as_output) = self.output_map.get(input) {
                    for input_pass in uses_as_output {
                        node.input_nodes.push(*input_pass);
                    }
                }
            }

            // If an output of this node is used as an input by another node, then
            // this node must have an edge pointing to that node.
            for output in &pass.outputs {
                if let Some(uses_as_input) = self.input_map.get(output) {
                    for output_pass in uses_as_input {
                        node.output_nodes.push(*output_pass);
                    }
                }
            }

            nodes.insert(RenderPassId(idx), node);
        }

        nodes
    }

    fn sorted_nodes(nodes: &HashMap<RenderPassId, RenderGraphNode>) -> Vec<RenderPassId> {
        let mut sorted_nodes: Vec<RenderPassId> = Vec::with_capacity(nodes.len());
        let mut visited: HashMap<RenderPassId, u8> = Default::default();
        let mut stack: Vec<RenderPassId> = Default::default();

        for pass_id in nodes.keys() {
            stack.push(*pass_id);
            while !stack.is_empty() {
                let to_visit = stack.last().unwrap();
                if let Some(count) = visited.get_mut(to_visit) {
                    match count {
                        1 => {
                            *count = 2;
                            sorted_nodes.push(*to_visit);
                            stack.pop();
                        }
                        2 => {
                            stack.pop();
                        }
                        _ => unreachable!(),
                    }
                } else {
                    visited.insert(*to_visit, 1);
                    let to_visit_node = nodes.get(to_visit).unwrap();
                    for child_name in &to_visit_node.output_nodes {
                        if !visited.contains_key(child_name) {
                            stack.push(*child_name);
                        }
                    }
                }
            }
        }

        sorted_nodes
    }

    pub fn run(&mut self, bump: &'a Bump, cinder: &mut Cinder) -> Result<PresentContext> {
        // TODO: Label colors, flag to disable it

        let nodes = self.compile_nodes(bump);
        let sorted_nodes = Self::sorted_nodes(&nodes);

        let surface_rect = cinder.device.surface_rect();

        cinder
            .device
            .begin_queue_label("Frame Begin", [0.0, 0.0, 1.0, 1.0]);
        let cmd_list = cinder.command_queue.get_command_list(&cinder.device)?;
        let swapchain_image = cinder.swapchain.acquire_image(&cinder.device, &cmd_list)?;

        for pass_id in sorted_nodes.iter().rev() {
            let pass = self.passes.get(pass_id.0).unwrap();

            // TODO: Maybe stop allocating every frame here
            let compiled_passes = pass
                .color_attachments
                .iter()
                .map(|(ty, desc)| match ty {
                    AttachmentType::SwapchainImage => {
                        RenderAttachment::color(swapchain_image, *desc)
                    }
                    AttachmentType::Reference(_) => todo!(),
                })
                .collect::<Vec<_>>();

            let depth_attachment = pass.depth_attachment.as_ref().map(|(ty, desc)| match ty {
                AttachmentType::SwapchainImage => {
                    panic!("Swapchain Image not yet supported for depth attachment")
                }
                AttachmentType::Reference(id) => {
                    let image = cinder
                        .resource_manager
                        .images
                        .get(*id)
                        .expect("Could not find depth attachment image");
                    RenderAttachment::depth(image, *desc)
                }
            });

            cmd_list.begin_label(
                &cinder.device,
                &format!(
                    "Begin Rendering: {:?}",
                    pass.name.unwrap_or(&format!("Pass #{}", pass_id.0))
                ),
                [1.0, 0.0, 0.0, 1.0],
            );
            cmd_list.begin_rendering(
                &cinder.device,
                pass.render_area.unwrap_or(surface_rect),
                &compiled_passes,
                depth_attachment,
            );
            // TODO: Figure out something with viewport/scissor as well
            cmd_list.bind_viewport(&cinder.device, surface_rect, pass.flipped_viewport);
            cmd_list.bind_scissor(&cinder.device, surface_rect);
            (pass.callback)(cinder, &cmd_list)?;
            cmd_list.end_rendering(&cinder.device);
            cmd_list.end_label(&cinder.device);
        }

        Ok(PresentContext {
            present_rect: surface_rect,
            cmd_list,
            swapchain_image,
        })
    }
}
