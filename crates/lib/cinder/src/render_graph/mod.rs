use crate::{
    command_queue::{RenderAttachment, RenderAttachmentDesc},
    Cinder,
};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum AttachmentType {
    SwapchainImage,
    Reference(String),
}

impl From<&str> for AttachmentType {
    fn from(value: &str) -> Self {
        Self::Reference(value.to_string())
    }
}

impl From<String> for AttachmentType {
    fn from(value: String) -> Self {
        Self::Reference(value)
    }
}

#[derive(Debug, Default)]
pub struct RenderPass {
    color_attachments: HashMap<AttachmentType, RenderAttachmentDesc>,
    depth_attachment: Option<(String, RenderAttachmentDesc)>,
}

impl RenderPass {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_color_attachment(
        &mut self,
        attachment: impl Into<AttachmentType>,
        desc: RenderAttachmentDesc,
    ) -> &mut Self {
        self.color_attachments.insert(attachment.into(), desc);
        self
    }

    pub fn set_depth_attachment(
        &mut self,
        name: impl Into<String>,
        desc: RenderAttachmentDesc,
    ) -> &mut Self {
        self.depth_attachment = Some((name.into(), desc));
        self
    }
}

#[derive(Debug, Default)]
pub struct RenderGraph {
    passes: HashMap<String, RenderPass>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_pass(&mut self, name: impl Into<String>) -> &mut RenderPass {
        self.passes.entry(name.into()).or_insert(Default::default())
    }

    pub fn run(&self, cinder: &mut Cinder) -> Result<()> {
        // TODO: This is nowhere close to right
        let surface_rect = cinder.device.surface_rect();

        let cmd_list = cinder.command_queue.get_command_list(&cinder.device)?;
        let swapchain_image = cinder.swapchain.acquire_image(&cinder.device, &cmd_list)?;

        for (_, pass) in &self.passes {
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

            // TODO: Will need to figure something out for the surface rect
            // TODO: Hook up depth
            cmd_list.begin_rendering(&cinder.device, surface_rect, &compiled_passes, None);
        }

        Ok(())
    }
}
