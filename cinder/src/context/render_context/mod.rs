use super::ContextShared;
use crate::{
    cinder::Cinder,
    device::Device,
    profiling::QueryPool,
    resoruces::{buffer::Buffer, image::Image, pipeline::GraphicsPipeline, shader::ShaderStage},
    swapchain::Swapchain,
    util::rect_to_vk,
};
use anyhow::Result;
use ash::vk;
use math::rect::Rect2D;
use thiserror::Error;

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
pub struct RenderAttachment(vk::RenderingAttachmentInfo);

impl RenderAttachment {
    pub fn color(swapchain: &Swapchain, present_index: u32) -> Self {
        RenderAttachment(
            vk::RenderingAttachmentInfo::builder()
                .image_view(swapchain.present_image_views[present_index as usize])
                .build(),
        )
    }

    pub fn depth(depth_image: &Image) -> Self {
        RenderAttachment(
            vk::RenderingAttachmentInfo::builder()
                .image_view(depth_image.view)
                .build(),
        )
    }

    pub fn load_op(mut self, op: AttachmentLoadOp) -> Self {
        self.0.load_op = op.into();
        self
    }

    pub fn store_op(mut self, op: AttachmentStoreOp) -> Self {
        self.0.store_op = op.into();
        self
    }

    pub fn clear_value(mut self, clear_value: ClearValue) -> Self {
        self.0.clear_value = clear_value.0;
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.0.image_layout = layout.into();
        self
    }
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("invalid push constant")]
    InvalidPushConstant,
}

pub struct RenderContextDescription {}

pub struct RenderContext {
    pub shared: ContextShared,
}

impl RenderContext {
    pub fn from_command_buffer(command_buffer: vk::CommandBuffer) -> Self {
        Self {
            shared: ContextShared::from_command_buffer(command_buffer),
        }
    }

    pub fn begin(&self, cinder: &Cinder) -> Result<()> {
        self.shared
            .begin(cinder.device(), cinder.draw_commands_reuse_fence)
    }

    pub fn end(
        &self,
        cinder: &Cinder,
        command_buffer_reuse_fence: vk::Fence,
        submit_queue: vk::Queue,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
    ) -> Result<()> {
        self.shared.end(
            cinder.device(),
            command_buffer_reuse_fence,
            submit_queue,
            wait_mask,
            wait_semaphores,
            signal_semaphores,
        )
    }

    pub fn begin_rendering(
        &self,
        cinder: &Cinder,
        render_area: Rect2D<i32, u32>,
        color_attachments: &[RenderAttachment],
        depth_attahcment: Option<RenderAttachment>,
    ) {
        let color_attachments = unsafe {
            std::mem::transmute::<&[RenderAttachment], &[vk::RenderingAttachmentInfo]>(
                color_attachments,
            )
        };

        let rendering_info = vk::RenderingInfo::builder()
            .render_area(rect_to_vk(render_area).unwrap())
            .color_attachments(color_attachments)
            .layer_count(1);
        let rendering_info = if let Some(depth_attachment) = &depth_attahcment {
            rendering_info.depth_attachment(&depth_attachment.0).build()
        } else {
            rendering_info.build()
        };

        unsafe {
            cinder
                .device()
                .cmd_begin_rendering(self.shared.command_buffer, &rendering_info);
        }
    }

    pub fn end_rendering(&self, cinder: &Cinder) {
        unsafe {
            cinder
                .device()
                .cmd_end_rendering(self.shared.command_buffer)
        };
    }

    pub fn bind_graphics_pipeline(&self, cinder: &Cinder, pipeline: &GraphicsPipeline) {
        unsafe {
            cinder.device().cmd_bind_pipeline(
                self.shared.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.common.pipeline,
            );
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        sets: &[vk::DescriptorSet],
    ) {
        unsafe {
            cinder.device().cmd_bind_descriptor_sets(
                self.shared.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.common.pipeline_layout,
                0,
                sets,
                &[],
            );
        }
    }

    pub fn bind_vertex_buffer(&self, cinder: &Cinder, buffer: &Buffer) {
        unsafe {
            cinder.device().cmd_bind_vertex_buffers(
                self.shared.command_buffer,
                0,
                &[buffer.raw],
                &[0],
            )
        }
    }

    pub fn bind_index_buffer(&self, cinder: &Cinder, buffer: &Buffer) {
        unsafe {
            cinder.device().cmd_bind_index_buffer(
                self.shared.command_buffer,
                buffer.raw,
                0,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn bind_scissor(&self, cinder: &Cinder, rect: Rect2D<i32, u32>) {
        unsafe {
            cinder.device().cmd_set_scissor(
                self.shared.command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: rect.offset().x(),
                        y: rect.offset().y(),
                    },
                    extent: vk::Extent2D {
                        width: rect.width(),
                        height: rect.height(),
                    },
                }],
            )
        }
    }

    pub fn bind_viewport(&self, cinder: &Cinder, rect: Rect2D<i32, u32>, flipped: bool) {
        let (y, height) = if flipped {
            (
                rect.height() as f32 - rect.offset().y() as f32,
                -(rect.height() as f32),
            )
        } else {
            (rect.offset().y() as f32, rect.height() as f32)
        };
        unsafe {
            cinder.device().cmd_set_viewport(
                self.shared.command_buffer,
                0,
                &[vk::Viewport {
                    x: rect.offset().x() as f32,
                    y,
                    width: rect.width() as f32,
                    height,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            )
        }
    }

    pub fn draw_offset(
        &self,
        cinder: &Cinder,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) {
        unsafe {
            cinder.device().cmd_draw_indexed(
                self.shared.command_buffer,
                index_count,
                1,
                first_index,
                vertex_offset,
                1,
            )
        }
    }

    pub fn push_constant(
        &self,
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        shader_stage: ShaderStage,
        idx: u32,
        data: &[u8],
    ) -> Result<(), PipelineError> {
        if let Some(push_constant) = pipeline.get_push_constant(shader_stage, idx) {
            unsafe {
                cinder.device().cmd_push_constants(
                    self.shared.command_buffer,
                    pipeline.common.pipeline_layout,
                    push_constant.stage.into(),
                    push_constant.offset,
                    data,
                );
            };
            Ok(())
        } else {
            Err(PipelineError::InvalidPushConstant)
        }
    }

    pub fn write_timestamp(&self, device: &Device, query_pool: &QueryPool, query: u32) {
        unsafe {
            device.cmd_write_timestamp(
                self.shared.command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                query_pool.raw,
                query,
            )
        }
    }

    pub fn reset_query_pool(&self, device: &Device, query_pool: &QueryPool) {
        unsafe {
            device.cmd_reset_query_pool(
                self.shared.command_buffer,
                query_pool.raw,
                0,
                query_pool.count,
            )
        }
    }

    // TODO: helpers
    pub fn transition_undefined_to_color(&self, cinder: &Cinder, present_index: u32) {
        let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
            .image(cinder.swapchain().present_images[present_index as usize])
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .level_count(1)
                    .build(),
            )
            .build();

        unsafe {
            cinder.device().cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_transition_barriers],
            )
        };
    }

    pub fn transition_color_to_present(&self, cinder: &Cinder, present_index: u32) {
        let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
            .image(cinder.swapchain().present_images[present_index as usize])
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .level_count(1)
                    .build(),
            )
            .build();

        unsafe {
            cinder.device().cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_transition_barriers],
            )
        };
    }
}
