use super::ContextShared;
use crate::{
    device::Device,
    profiling::QueryPool,
    resources::{
        buffer::Buffer,
        image::Image,
        pipeline::{compute::ComputePipeline, graphics::GraphicsPipeline, PipelineCommon},
        shader::ShaderStage,
        ResourceHandle,
    },
    util::rect_to_vk,
    view::Drawable,
};
use anyhow::Result;
use ash::vk;
use math::rect::Rect2D;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub enum Layout {
    Undefined,
    General,
    ColorAttachment,
    DepthAttachment,
    Present,
    TransferDst,
}

impl Default for Layout {
    fn default() -> Self {
        Self::ColorAttachment
    }
}

impl From<Layout> for vk::ImageLayout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Undefined => vk::ImageLayout::UNDEFINED,
            Layout::General => vk::ImageLayout::GENERAL,
            Layout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            Layout::DepthAttachment => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            Layout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
            Layout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        }
    }
}

pub enum Filter {
    Linear,
    Nearest,
}

impl From<Filter> for vk::Filter {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Linear => vk::Filter::LINEAR,
            Filter::Nearest => vk::Filter::NEAREST,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum ClearValue {
    Color { color: [f32; 4] },
    Depth { depth: f32, stencil: u32 },
}

impl Default for ClearValue {
    fn default() -> Self {
        ClearValue::Color {
            color: [1.0, 0.0, 1.0, 1.0],
        }
    }
}

impl ClearValue {
    pub fn default_color() -> Self {
        Default::default()
    }

    pub fn default_depth() -> Self {
        Self::Depth {
            depth: 0.0,
            stencil: 0,
        }
    }
}

impl From<ClearValue> for vk::ClearValue {
    fn from(value: ClearValue) -> Self {
        match value {
            ClearValue::Color { color } => vk::ClearValue {
                color: vk::ClearColorValue { float32: color },
            },
            ClearValue::Depth { depth, stencil } => vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue { depth, stencil },
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AttachmentLoadOp {
    Clear,
    Load,
    DontCare,
}

impl Default for AttachmentLoadOp {
    fn default() -> Self {
        Self::Clear
    }
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

#[derive(Debug, Clone, Copy)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

impl Default for AttachmentStoreOp {
    fn default() -> Self {
        Self::Store
    }
}

impl From<AttachmentStoreOp> for vk::AttachmentStoreOp {
    fn from(op: AttachmentStoreOp) -> Self {
        match op {
            AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
            AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RenderAttachmentDesc {
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub layout: Layout,
    pub clear_value: ClearValue,
}

pub struct RenderAttachment(vk::RenderingAttachmentInfo);

impl RenderAttachment {
    fn from_parts(image_view: vk::ImageView, desc: RenderAttachmentDesc) -> Self {
        Self(
            vk::RenderingAttachmentInfo::builder()
                .image_view(image_view)
                .load_op(desc.load_op.into())
                .store_op(desc.store_op.into())
                .clear_value(desc.clear_value.into())
                .image_layout(desc.layout.into())
                .build(),
        )
    }

    pub fn color(drawable: Drawable, desc: RenderAttachmentDesc) -> Self {
        Self::from_parts(drawable.image_view, desc)
    }

    pub fn depth(depth_image: &Image, desc: RenderAttachmentDesc) -> Self {
        Self::from_parts(depth_image.view, desc)
    }
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("invalid push constant")]
    InvalidPushConstant,
    #[error("invalid pipeline handle")]
    InvalidPipelineHandle,
    #[error("no bound pipeline")]
    NoBoundPipeline,
}

pub struct RenderContextDescription {}

pub struct RenderContext {
    pub shared: ContextShared,
    bound_pipeline: Option<ResourceHandle<GraphicsPipeline>>,
}

impl RenderContext {
    pub fn new(device: &Device) -> Result<Self> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(device.command_pool())
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            device
                .raw()
                .allocate_command_buffers(&command_buffer_allocate_info)?
        }[0];

        Ok(Self {
            shared: ContextShared::from_command_buffer(command_buffer),
            bound_pipeline: None,
        })
    }

    pub fn begin(&self, device: &Device) -> Result<()> {
        self.shared
            .begin(device.raw(), device.draw_commands_reuse_fence)
    }

    pub fn end(&self, device: &Device) -> Result<()> {
        // TODO: This stuff will be much better later with a RenderGraph impl
        self.shared.end(
            device.raw(),
            device.draw_commands_reuse_fence,
            device.present_queue(),
            &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
            &[device.present_complete_semaphore],
            &[device.rendering_complete_semaphore],
        )
    }

    pub fn begin_rendering(
        &self,
        device: &Device,
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
            device
                .raw()
                .cmd_begin_rendering(self.shared.command_buffer, &rendering_info);
        }
    }

    pub fn end_rendering(&mut self, device: &Device) {
        unsafe { device.raw().cmd_end_rendering(self.shared.command_buffer) };
        // TODO: Is this the right way to reset bound pipeline?
        self.bound_pipeline = None;
    }

    pub fn bind_graphics_pipeline(
        &mut self,
        device: &Device,
        handle: ResourceHandle<GraphicsPipeline>,
    ) -> Result<(), PipelineError> {
        if let Some(pipeline) = device.get_graphics_pipeline(handle) {
            unsafe {
                device.raw().cmd_bind_pipeline(
                    self.shared.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.common.pipeline,
                );
            };
            self.bound_pipeline = Some(handle);
            Ok(())
        } else {
            Err(PipelineError::InvalidPipelineHandle)
        }
    }

    pub fn bind_compute_pipeline(&self, device: &Device, pipeline: &ComputePipeline) {
        unsafe {
            device.raw().cmd_bind_pipeline(
                self.shared.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.common.pipeline,
            );
        }
    }

    pub fn dispatch(
        &self,
        device: &Device,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) {
        unsafe {
            device.raw().cmd_dispatch(
                self.shared.command_buffer,
                group_count_x,
                group_count_y,
                group_count_z,
            );
        }
    }

    pub fn bind_descriptor_sets(&self, device: &Device) -> Result<(), PipelineError> {
        if let Some(handle) = &self.bound_pipeline {
            let pipeline = device
                .get_graphics_pipeline(*handle)
                .ok_or(PipelineError::InvalidPipelineHandle)?;

            unsafe {
                device.raw().cmd_bind_descriptor_sets(
                    self.shared.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.common.pipeline_layout,
                    0,
                    &[pipeline.bind_group.as_ref().unwrap().0],
                    &[],
                );
            };
            Ok(())
        } else {
            Err(PipelineError::NoBoundPipeline)
        }
    }

    pub fn bind_vertex_buffer(&self, device: &Device, buffer: &Buffer) {
        unsafe {
            device
                .raw()
                .cmd_bind_vertex_buffers(self.shared.command_buffer, 0, &[buffer.raw], &[0])
        }
    }

    pub fn bind_index_buffer(&self, device: &Device, buffer: &Buffer) {
        unsafe {
            device.raw().cmd_bind_index_buffer(
                self.shared.command_buffer,
                buffer.raw,
                0,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn bind_scissor(&self, device: &Device, rect: Rect2D<i32, u32>) {
        unsafe {
            device.raw().cmd_set_scissor(
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

    pub fn bind_viewport(&self, device: &Device, rect: Rect2D<i32, u32>, flipped: bool) {
        let (y, height) = if flipped {
            (
                rect.height() as f32 - rect.offset().y() as f32,
                -(rect.height() as f32),
            )
        } else {
            (rect.offset().y() as f32, rect.height() as f32)
        };
        unsafe {
            device.raw().cmd_set_viewport(
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
        device: &Device,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) {
        unsafe {
            device.raw().cmd_draw_indexed(
                self.shared.command_buffer,
                index_count,
                1,
                first_index,
                vertex_offset,
                1,
            )
        }
    }

    fn push_constant(
        &self,
        device: &Device,
        pipeline_common: &PipelineCommon,
        shader_stage: ShaderStage,
        idx: u32,
        data: &[u8],
    ) -> Result<(), PipelineError> {
        if let Some(push_constant) = pipeline_common.get_push_constant(shader_stage, idx) {
            unsafe {
                device.raw().cmd_push_constants(
                    self.shared.command_buffer,
                    pipeline_common.pipeline_layout,
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

    pub fn set_vertex_bytes<T: Sized>(
        &self,
        device: &Device,
        data: &T,
        idx: u32,
    ) -> Result<(), PipelineError> {
        if let Some(handle) = &self.bound_pipeline {
            let pipeline = device
                .get_graphics_pipeline(*handle)
                .ok_or(PipelineError::InvalidPipelineHandle)?;
            self.push_constant(
                device,
                &pipeline.common,
                ShaderStage::Vertex,
                idx,
                util::as_u8_slice(data),
            )
        } else {
            Err(PipelineError::NoBoundPipeline)
        }
    }

    pub fn set_fragment_bytes<T: Sized>(
        &self,
        device: &Device,
        data: &T,
        idx: u32,
    ) -> Result<(), PipelineError> {
        if let Some(handle) = &self.bound_pipeline {
            let pipeline = device
                .get_graphics_pipeline(*handle)
                .ok_or(PipelineError::InvalidPipelineHandle)?;
            self.push_constant(
                device,
                &pipeline.common,
                ShaderStage::Fragment,
                idx,
                util::as_u8_slice(data),
            )
        } else {
            Err(PipelineError::NoBoundPipeline)
        }
    }

    pub fn write_timestamp(&self, device: &Device, query_pool: &QueryPool, query: u32) {
        unsafe {
            device.raw().cmd_write_timestamp(
                self.shared.command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                query_pool.raw,
                query,
            )
        }
    }

    pub fn reset_query_pool(&self, device: &Device, query_pool: &QueryPool) {
        unsafe {
            device.raw().cmd_reset_query_pool(
                self.shared.command_buffer,
                query_pool.raw,
                0,
                query_pool.count,
            )
        }
    }

    // TODO: helpers
    pub fn transition_undefined_to_color(&self, device: &Device, drawable: Drawable) {
        let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
            .image(drawable.image)
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
            device.raw().cmd_pipeline_barrier(
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

    pub fn transition_color_to_present(&self, device: &Device, drawable: Drawable) {
        let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
            .image(drawable.image)
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
            device.raw().cmd_pipeline_barrier(
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

    pub fn blit_image(
        &self,
        device: &Device,
        src_image: vk::Image,
        src_aspect_mask: vk::ImageAspectFlags,
        src_layout: Layout,
        src_rect: Rect2D<i32, i32>,
        dst_image: vk::Image,
        dst_aspect_mask: vk::ImageAspectFlags,
        dst_layout: Layout,
        dst_rect: Rect2D<i32, i32>,
        filter: Filter,
    ) {
        let region = vk::ImageBlit::builder()
            .src_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(src_aspect_mask)
                    .layer_count(1)
                    .build(),
            )
            .src_offsets([
                vk::Offset3D {
                    x: src_rect.offset().x(),
                    y: src_rect.offset().y(),
                    z: 0,
                },
                vk::Offset3D {
                    x: src_rect.width(),
                    y: src_rect.height(),
                    z: 1,
                },
            ])
            .dst_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(dst_aspect_mask)
                    .layer_count(1)
                    .build(),
            )
            .dst_offsets([
                vk::Offset3D {
                    x: dst_rect.offset().x(),
                    y: dst_rect.offset().y(),
                    z: 0,
                },
                vk::Offset3D {
                    x: dst_rect.width(),
                    y: dst_rect.height(),
                    z: 1,
                },
            ])
            .build();

        unsafe {
            device.raw().cmd_blit_image(
                self.shared.command_buffer,
                src_image,
                src_layout.into(),
                dst_image,
                dst_layout.into(),
                &[region],
                filter.into(),
            );
        }
    }

    // TODO: Temp
    pub fn pipeline_barrier(
        &self,
        device: &Device,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        dependency_flags: vk::DependencyFlags,
        buffer_barriers: &[vk::BufferMemoryBarrier],
        image_barriers: &[vk::ImageMemoryBarrier],
    ) {
        unsafe {
            device.raw().cmd_pipeline_barrier(
                self.shared.command_buffer,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                &[],
                buffer_barriers,
                image_barriers,
            )
        }
    }
}

pub fn image_barrier(
    image: vk::Image,
    src_access_mask: vk::AccessFlags,
    dst_access_mask: vk::AccessFlags,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    aspect_mask: vk::ImageAspectFlags,
    base_mip_level: u32,
    level_count: u32,
) -> vk::ImageMemoryBarrier {
    vk::ImageMemoryBarrier {
        src_access_mask,
        dst_access_mask,
        old_layout,
        new_layout,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        image,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level,
            level_count,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
            ..Default::default()
        },
        ..Default::default()
    }
}
