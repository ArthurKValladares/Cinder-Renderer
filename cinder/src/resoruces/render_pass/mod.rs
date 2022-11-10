use crate::{surface::SurfaceData, swapchain::Swapchain};
use anyhow::Result;
use ash::vk;

use super::image::Image;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearValue(vk::ClearValue);

impl ClearValue {
    pub fn color(vals: [f32; 4]) -> Self {
        Self(vk::ClearValue {
            color: vk::ClearColorValue { float32: vals },
        })
    }

    pub fn depth(depth: f32, stencil: u32) -> Self {
        Self(vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue { depth, stencil },
        })
    }
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
            Layout::DepthAttachment => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            Layout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AttachmentLoadOp {
    Clear,
    Load,
    DontCare,
}

impl From<AttachmentLoadOp> for vk::AttachmentLoadOp {
    fn from(op: AttachmentLoadOp) -> Self {
        match op {
            AttachmentLoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
            AttachmentLoadOp::Load => vk::AttachmentLoadOp::LOAD,
            AttachmentLoadOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

impl From<AttachmentStoreOp> for vk::AttachmentStoreOp {
    fn from(op: AttachmentStoreOp) -> Self {
        match op {
            AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
            AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassAttachmentDesc {
    desc: vk::AttachmentDescription,
}

impl RenderPassAttachmentDesc {
    pub fn new(format: impl Into<vk::Format>) -> Self {
        Self {
            desc: vk::AttachmentDescription {
                format: format.into(),
                samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            },
        }
    }

    pub fn load_op(mut self, op: AttachmentLoadOp) -> Self {
        self.desc.load_op = op.into();
        self
    }

    pub fn store_op(mut self, op: AttachmentStoreOp) -> Self {
        self.desc.store_op = op.into();
        self
    }

    pub fn initial_layout(mut self, layout: Layout) -> Self {
        self.desc.initial_layout = layout.into();
        self
    }

    pub fn final_layout(mut self, layout: Layout) -> Self {
        self.desc.final_layout = layout.into();
        self
    }

    pub fn compile(self) -> vk::AttachmentDescription {
        self.desc
    }
}

fn create_render_pass_objects(
    device: &ash::Device,
    swapchain: &Swapchain,
    surface_data: &SurfaceData,
    depth_image: &Image,
    desc: &RenderPassDescription,
) -> Result<(vk::RenderPass, Vec<vk::Framebuffer>)> {
    let renderpass_attachments = if let Some(depth_attachment) = desc.depth_attachment {
        vec![desc.color_attachment.compile(), depth_attachment.compile()]
    } else {
        vec![desc.color_attachment.compile()]
    };
    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let subpass = if desc.depth_attachment.is_some() {
        vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref))
            .depth_stencil_attachment(&depth_attachment_ref)
            .build()
    } else {
        vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref))
            .build()
    };

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&renderpass_attachments)
        .subpasses(std::slice::from_ref(&subpass));

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
                .layers(1)
                .build();

            unsafe { device.create_framebuffer(&frame_buffer_create_info, None) }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((render_pass, framebuffers))
}

#[derive(Debug)]
pub struct RenderPassDescription {
    pub color_attachment: RenderPassAttachmentDesc,
    pub depth_attachment: Option<RenderPassAttachmentDesc>,
    // TODO: subpasses
}

pub struct RenderPass {
    desc: RenderPassDescription,
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl RenderPass {
    pub(crate) fn create(
        device: &ash::Device,
        swapchain: &Swapchain,
        surface_data: &SurfaceData,
        depth_image: &Image,
        desc: RenderPassDescription,
    ) -> Result<Self> {
        let (render_pass, framebuffers) =
            create_render_pass_objects(device, swapchain, surface_data, depth_image, &desc)?;
        Ok(RenderPass {
            render_pass,
            framebuffers,
            desc,
        })
    }

    pub fn recreate(
        &mut self,
        device: &ash::Device,
        swapchain: &Swapchain,
        surface_data: &SurfaceData,
        depth_image: &Image,
    ) -> Result<()> {
        let (render_pass, framebuffers) =
            create_render_pass_objects(device, swapchain, surface_data, depth_image, &self.desc)?;
        self.render_pass = render_pass;
        self.framebuffers = framebuffers;
        Ok(())
    }

    pub(crate) fn clean(&mut self, device: &ash::Device) {
        unsafe {
            for framebuffer in self.framebuffers.drain(..) {
                device.destroy_framebuffer(framebuffer, None);
            }
            device.destroy_render_pass(self.render_pass, None);
        }
    }
}
