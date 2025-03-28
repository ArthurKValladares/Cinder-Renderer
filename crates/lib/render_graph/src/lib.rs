#![feature(allocator_api)]

use anyhow::{Ok, Result};
use bumpalo::{collections::Vec as BumpVec, Bump};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap, HashSet};
use math::rect::Rect2D;
use renderer::{
    command_queue::{CommandList, RenderAttachment, RenderAttachmentDesc},
    resources::image::Image,
    swapchain::SwapchainImage,
    Renderer,
};
use resource_manager::ResourceId;

type BumpHashSet<'a, T> = HashSet<T, DefaultHashBuilder, &'a Bump>;
type BumpHashMap<'a, K, V> = HashMap<K, V, DefaultHashBuilder, &'a Bump>;
type BumpBox<'a, T> = Box<T, &'a Bump>;

static DEBUG_LABELS: bool = false;

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

type RenderPassCallback<'a> = dyn Fn(&Renderer, &CommandList) -> Result<()> + 'a;

pub struct RenderPass<'a> {
    color_attachments: BumpHashMap<'a, AttachmentType, RenderAttachmentDesc>,
    depth_attachment: Option<(AttachmentType, RenderAttachmentDesc)>,
    inputs: BumpVec<'a, RenderPassResource>,
    outputs: BumpVec<'a, RenderPassResource>,
    render_area: Option<Rect2D<i32, u32>>,
    flipped_viewport: bool,
    callback: BumpBox<'a, RenderPassCallback<'a>>,
    name: Option<&'a str>,
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
            color_attachments: BumpHashMap::new_in(bump),
            depth_attachment: Default::default(),
            inputs: BumpVec::new_in(bump),
            outputs: BumpVec::new_in(bump),
            render_area: None,
            flipped_viewport: true,
            callback: Box::new_in(|_, _| Ok(()), bump),
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

    pub fn set_callback<F>(mut self, bump: &'a Bump, callback: F) -> Self
    where
        F: Fn(&Renderer, &CommandList) -> Result<()> + 'a,
    {
        self.callback = Box::new_in(callback, bump);
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
    pub fn present(self, cinder: &mut Renderer) -> Result<bool> {
        let ret = cinder
            .swapchain
            .present(&cinder.device, self.cmd_list, self.swapchain_image);
        if DEBUG_LABELS {
            cinder.device.end_queue_label();
        }
        ret
    }
}

#[derive(Debug)]
pub struct RenderGraph<'a> {
    passes: BumpVec<'a, RenderPass<'a>>,
    // Instead of a set, could maybe be a vector of bool
    input_map: BumpHashMap<'a, RenderPassResource, BumpHashSet<'a, RenderPassId>>,
    output_map: BumpHashMap<'a, RenderPassResource, BumpHashSet<'a, RenderPassId>>,
}

impl<'a> RenderGraph<'a> {
    pub fn new(bump: &'a Bump) -> Self {
        Self {
            passes: BumpVec::new_in(bump),
            input_map: BumpHashMap::new_in(bump),
            output_map: BumpHashMap::new_in(bump),
        }
    }

    pub fn add_pass(&mut self, bump: &'a Bump, pass: RenderPass<'a>) {
        let id = RenderPassId(self.passes.len());
        for input in &pass.inputs {
            self.input_map
                .entry(*input)
                .or_insert_with(|| BumpHashSet::new_in(bump))
                .insert(id);
        }
        for output in &pass.outputs {
            self.output_map
                .entry(*output)
                .or_insert_with(|| BumpHashSet::new_in(bump))
                .insert(id);
        }
        self.passes.push(pass)
    }

    fn compile_nodes<'b>(&self, bump: &'b Bump) -> BumpVec<RenderGraphNode<'b>> {
        let mut nodes = BumpVec::with_capacity_in(self.passes.len(), bump);
        for (_idx, pass) in self.passes.iter().enumerate() {
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

            nodes.push(node);
        }

        nodes
    }

    fn sorted_nodes<'b>(
        bump: &'a Bump,
        nodes: &BumpVec<'b, RenderGraphNode>,
    ) -> BumpVec<'a, RenderPassId> {
        let mut sorted_nodes: BumpVec<'a, RenderPassId> =
            BumpVec::with_capacity_in(nodes.len(), bump);
        let mut stack: BumpVec<RenderPassId> = BumpVec::new_in(&bump);
        let mut visited: BumpVec<u8> = bumpalo::vec![in &bump; 0; nodes.len()];

        for pass_idx in 0..nodes.len() {
            let pass_id = RenderPassId(pass_idx);
            stack.push(pass_id);
            while !stack.is_empty() {
                let to_visit = stack.last().unwrap();
                let visit_count = &mut visited[to_visit.0];
                match visit_count {
                    0 => {
                        *visit_count = 1;
                        let to_visit_node = &nodes[to_visit.0];
                        for child_id in &to_visit_node.output_nodes {
                            if visited[child_id.0] == 0 {
                                stack.push(*child_id);
                            }
                        }
                    }
                    1 => {
                        *visit_count = 2;
                        sorted_nodes.push(*to_visit);
                        stack.pop();
                    }
                    2 => {
                        stack.pop();
                    }
                    _ => unreachable!(),
                }
            }
        }

        sorted_nodes
    }

    pub fn run(self, bump: &'a Bump, cinder: &mut Renderer) -> Result<PresentContext> {
        // TODO: Label colors, flag to disable it

        let nodes = self.compile_nodes(bump);
        let sorted_nodes = Self::sorted_nodes(bump, &nodes);

        let surface_rect = cinder.device.surface_rect();

        if DEBUG_LABELS {
            cinder
                .device
                .begin_queue_label("Frame Begin", [0.0, 0.0, 1.0, 1.0]);
        }
        let cmd_list = cinder.command_queue.get_command_list(&cinder.device)?;
        let swapchain_image = cinder.swapchain.acquire_image(&cinder.device, &cmd_list)?;

        for pass_id in sorted_nodes.iter().rev() {
            let pass = self.passes.get(pass_id.0).unwrap();

            let mut compiled_passes = BumpVec::new_in(bump);
            for (ty, desc) in pass.color_attachments.iter() {
                match ty {
                    AttachmentType::SwapchainImage => {
                        compiled_passes.push(RenderAttachment::color(swapchain_image, *desc));
                    }
                    AttachmentType::Reference(_) => todo!(),
                }
            }

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

            if DEBUG_LABELS {
                cmd_list.begin_label(
                    &cinder.device,
                    &format!(
                        "Begin Rendering: {:?}",
                        pass.name.unwrap_or(&format!("Pass #{}", pass_id.0))
                    ),
                    [1.0, 0.0, 0.0, 1.0],
                );
            }
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
            if DEBUG_LABELS {
                cmd_list.end_label(&cinder.device);
            }
        }

        Ok(PresentContext {
            present_rect: surface_rect,
            cmd_list,
            swapchain_image,
        })
    }
}
