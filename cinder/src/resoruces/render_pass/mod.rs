use crate::{device::Device, surface::SurfaceData, swapchain::Swapchain};
use anyhow::Result;
use ash::vk;

use super::texture::Texture;

#[derive(Debug, Clone, Copy)]
pub struct RenderPassAttachmentDesc {
    format: vk::Format,
    load_op: vk::AttachmentLoadOp,
    store_op: vk::AttachmentStoreOp,
    samples: vk::SampleCountFlags,
    initial_layout: vk::ImageLayout,
    final_layout: vk::ImageLayout,
}

pub enum Layout {
    Undefined,
    General,
    ColorAttachment,
    DepthAttachment,
    Present,
}

impl From<Layout> for vk::ImageLayout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Undefined => vk::ImageLayout::UNDEFINED,
            Layout::General => vk::ImageLayout::GENERAL,
            Layout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            Layout::DepthAttachment => vk::ImageLayout::DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL,
            Layout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
        }
    }
}

pub struct LayoutTransition {
    pub initial_layout: Layout,
    pub final_layout: Layout,
}

impl RenderPassAttachmentDesc {
    // TODO: Better abstraction for creating these later
    pub fn clear_store(format: impl Into<vk::Format>) -> Self {
        Self {
            format: format.into(),
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }
    }

    pub fn load_store(format: impl Into<vk::Format>) -> Self {
        Self {
            format: format.into(),
            load_op: vk::AttachmentLoadOp::LOAD,
            store_op: vk::AttachmentStoreOp::STORE,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }
    }

    pub fn clear_dont_care(format: impl Into<vk::Format>) -> Self {
        Self {
            format: format.into(),
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }
    }

    pub fn load_dont_care(format: impl Into<vk::Format>) -> Self {
        Self {
            format: format.into(),
            load_op: vk::AttachmentLoadOp::LOAD,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }
    }

    pub fn with_layout_transition(mut self, layout_transition: LayoutTransition) -> Self {
        self.initial_layout = layout_transition.initial_layout.into();
        self.final_layout = layout_transition.final_layout.into();

        self
    }

    pub fn discard_input(mut self) -> Self {
        self.load_op = vk::AttachmentLoadOp::DONT_CARE;
        self
    }

    pub fn clear_input(mut self) -> Self {
        self.load_op = vk::AttachmentLoadOp::CLEAR;
        self
    }

    pub fn discard_output(mut self) -> Self {
        self.store_op = vk::AttachmentStoreOp::DONT_CARE;
        self
    }

    pub fn compile(self) -> vk::AttachmentDescription {
        vk::AttachmentDescription {
            format: self.format,
            samples: self.samples,
            load_op: self.load_op,
            store_op: self.store_op,
            initial_layout: self.initial_layout,
            final_layout: self.final_layout,
            ..Default::default()
        }
    }
}

pub struct RenderPassDescription<const N: usize> {
    pub color_attachments: [RenderPassAttachmentDesc; N],
    pub depth_attachment: Option<RenderPassAttachmentDesc>,
    // TODO: subpasses
}

pub struct RenderPass {
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    // TODO: Should this be here? Might make caching worse
    pub clear_values: Vec<vk::ClearValue>,
    pub render_area: vk::Rect2D,
}

impl RenderPass {
    pub(crate) fn create<const N: usize>(
        device: &ash::Device,
        swapchain: &Swapchain,
        surface_data: &SurfaceData,
        depth_image: &Texture,
        desc: RenderPassDescription<N>,
    ) -> Result<Self> {
        // TODO: image transitions should be determined automatically.
        let renderpass_attachments = desc
            .color_attachments
            .iter()
            .map(|a| a.compile())
            .chain(desc.depth_attachment.as_ref().map(|a| a.compile()))
            .collect::<Vec<_>>();

        let color_attachment_refs = (0..desc.color_attachments.len() as u32)
            .map(|attachment| {
                vk::AttachmentReference::builder()
                    .attachment(attachment)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .build()
            })
            .collect::<Vec<_>>();
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: desc.color_attachments.len() as u32,
            layout: vk::ImageLayout::DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL,
        };

        let mut subpass_description = vk::SubpassDescription::builder()
            .color_attachments(&color_attachment_refs)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
        if desc.depth_attachment.is_some() {
            subpass_description =
                subpass_description.depth_stencil_attachment(&depth_attachment_ref);
        }
        let subpass_description = subpass_description.build();

        // TODO: Subpass dependency stuff
        let subpasses = [subpass_description];
        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(&subpasses);

        let render_pass = unsafe { device.create_render_pass(&render_pass_create_info, None)? };

        let framebuffers = swapchain
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = if desc.depth_attachment.is_some() {
                    vec![present_image_view, depth_image.view]
                } else {
                    vec![present_image_view]
                };
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(surface_data.surface_resolution.width)
                    .height(surface_data.surface_resolution.height)
                    .layers(1);

                unsafe { device.create_framebuffer(&frame_buffer_create_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RenderPass {
            render_pass,
            framebuffers,
            render_area: surface_data.surface_resolution.into(),
            clear_values: vec![
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [1.0, 0.0, 1.0, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ],
        })
    }

    // TODO: All cleamn functions should take ownership
    pub(crate) fn clean(&mut self, device: &ash::Device) {
        unsafe {
            for framebuffer in self.framebuffers.drain(..) {
                device.destroy_framebuffer(framebuffer, None);
            }
            device.destroy_render_pass(self.render_pass, None);
        }
    }
}
