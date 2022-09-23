use ash::vk;

#[derive(Debug, Clone, Copy)]
pub struct RenderPassAttachmentDesc {
    format: vk::Format,
    load_op: vk::AttachmentLoadOp,
    store_op: vk::AttachmentStoreOp,
    samples: vk::SampleCountFlags,
}

impl RenderPassAttachmentDesc {
    pub fn with_format(format: impl Into<vk::Format>) -> Self {
        Self {
            format: format.into(),
            load_op: vk::AttachmentLoadOp::LOAD,
            store_op: vk::AttachmentStoreOp::STORE,
            samples: vk::SampleCountFlags::TYPE_1,
        }
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

    pub fn compile_with_layout_transition(
        self,
        initial_layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
    ) -> vk::AttachmentDescription {
        vk::AttachmentDescription {
            format: self.format,
            samples: self.samples,
            load_op: self.load_op,
            store_op: self.store_op,
            initial_layout,
            final_layout,
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
