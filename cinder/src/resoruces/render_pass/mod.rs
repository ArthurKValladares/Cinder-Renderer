use ash::vk;

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
