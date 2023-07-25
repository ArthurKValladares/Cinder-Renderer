use crate::{
    command_queue::{CommandList, RenderAttachment, RenderAttachmentDesc},
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

pub struct RenderPass<'a> {
    color_attachments: HashMap<AttachmentType, RenderAttachmentDesc>,
    depth_attachment: Option<(String, RenderAttachmentDesc)>,
    callback: Box<dyn Fn(&Cinder, &CommandList) -> Result<()> + 'a>,
}

impl<'a> std::fmt::Debug for RenderPass<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderPass")
            .field("color_attachments", &self.color_attachments)
            .field("depth_attachment", &self.depth_attachment)
            .finish()
    }
}

impl<'a> Default for RenderPass<'a> {
    fn default() -> Self {
        Self {
            color_attachments: Default::default(),
            depth_attachment: Default::default(),
            callback: Box::new(|_, _| Ok(())),
        }
    }
}

impl<'a> RenderPass<'a> {
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

    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(&Cinder, &CommandList) -> Result<()> + 'a,
    {
        self.callback = Box::new(callback);
    }
}

#[derive(Debug, Default)]
pub struct RenderGraph<'a> {
    passes: HashMap<String, RenderPass<'a>>,
}

impl<'a> RenderGraph<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_pass(&mut self, name: impl Into<String>) -> &mut RenderPass<'a> {
        self.passes.entry(name.into()).or_insert(Default::default())
    }

    pub fn run(&self, cinder: &mut Cinder) -> Result<bool> {
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
            // TODO: Figure out something with viewport/scissor as well
            cmd_list.bind_viewport(&cinder.device, surface_rect, true);
            cmd_list.bind_scissor(&cinder.device, surface_rect);
            (pass.callback)(cinder, &cmd_list)?;
            cmd_list.end_rendering(&cinder.device);
        }

        cinder
            .swapchain
            .present(&cinder.device, cmd_list, swapchain_image)
    }
}
