use anyhow::Result;
use cinder::{
    command_queue::{CommandList, RenderAttachment, RenderAttachmentDesc},
    resources::image::Image,
    Cinder,
};
use math::rect::Rect2D;
use resource_manager::ResourceId;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum AttachmentType {
    SwapchainImage,
    Reference(ResourceId<Image>),
}

pub struct RenderPass<'a> {
    color_attachments: HashMap<AttachmentType, RenderAttachmentDesc>,
    depth_attachment: Option<(AttachmentType, RenderAttachmentDesc)>,
    render_area: Option<Rect2D<i32, u32>>,
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
            render_area: None,
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
        attachment: impl Into<AttachmentType>,
        desc: RenderAttachmentDesc,
    ) -> &mut Self {
        self.depth_attachment = Some((attachment.into(), desc));
        self
    }

    pub fn with_render_area(&mut self, render_area: Rect2D<i32, u32>) -> &mut Self {
        self.render_area = Some(render_area);
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
        Self::default()
    }

    pub fn register_pass(&mut self, name: impl Into<String>) -> &mut RenderPass<'a> {
        self.passes.entry(name.into()).or_insert(Default::default())
    }

    pub fn run(&mut self, cinder: &mut Cinder) -> Result<bool> {
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

            cmd_list.begin_rendering(
                &cinder.device,
                pass.render_area.unwrap_or(surface_rect),
                &compiled_passes,
                depth_attachment,
            );
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
