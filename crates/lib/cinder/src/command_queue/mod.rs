use crate::{
    device::{cmd_begin_label, cmd_end_label, cmd_insert_label, Device, MAX_FRAMES_IN_FLIGHT},
    resources::{
        bind_group::BindGroup,
        buffer::Buffer,
        image::{Image, ImageUsage, Layout},
        pipeline::{graphics::GraphicsPipeline, PipelineCommon, PipelineError},
        shader::ShaderStage,
    },
    swapchain::SwapchainImage,
};
use anyhow::Result;
use ash::vk;
use math::rect::Rect2D;
use serde::Deserialize;

///
/// TEMP START: Not convinced about this, keeping it for now
///

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

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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

#[repr(transparent)]
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

    pub fn color(swapchain_image: SwapchainImage, desc: RenderAttachmentDesc) -> Self {
        Self::from_parts(swapchain_image.image_view, desc)
    }

    pub fn depth(depth_image: &Image, desc: RenderAttachmentDesc) -> Self {
        Self::from_parts(depth_image.view, desc)
    }
}

///
/// TEMP END
///

pub struct ImageBarrierDescription {
    base_mip_level: u32,
    level_count: u32,
    base_array_layer: u32,
    layer_count: u32,
}

impl Default for ImageBarrierDescription {
    fn default() -> Self {
        Self {
            base_mip_level: 0,
            level_count: vk::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CommandList {
    command_buffer: vk::CommandBuffer,
}

impl CommandList {
    fn new(
        device: &Device,
        command_pool: vk::CommandPool,
        idx: Option<usize>,
    ) -> Result<Self, vk::Result> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };

        let command_buffer = unsafe {
            device
                .raw()
                .allocate_command_buffers(&command_buffer_allocate_info)?[0]
        };

        device.set_name(
            vk::ObjectType::COMMAND_BUFFER,
            command_buffer,
            &if let Some(idx) = idx {
                format!("Command Buffer {idx}")
            } else {
                "Immediate Command Buffer".to_owned()
            },
        );

        Ok(Self { command_buffer })
    }

    pub fn begin(&self, device: &Device) -> Result<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device
                .raw()
                .begin_command_buffer(self.command_buffer, &command_buffer_begin_info)
        }?;

        Ok(())
    }

    pub fn end(&self, device: &Device) -> Result<()> {
        unsafe { device.raw().end_command_buffer(self.command_buffer) }?;

        Ok(())
    }

    pub fn reset(&self, device: &Device) -> Result<()> {
        unsafe {
            device.raw().reset_command_buffer(
                self.command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )?;
        }

        Ok(())
    }

    pub fn immediate_submit(&self, device: &Device, queue: vk::Queue) -> Result<()> {
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[self.command_buffer])
            .build();

        unsafe {
            device
                .raw()
                .queue_submit(queue, &[submit_info], vk::Fence::null())?;
            device.raw().queue_wait_idle(queue)?;
        }

        Ok(())
    }

    pub fn buffer(&self) -> vk::CommandBuffer {
        self.command_buffer
    }

    pub fn set_image_memory_barrier(
        &self,
        device: &Device,
        image: vk::Image,
        aspect_mask: vk::ImageAspectFlags,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        desc: ImageBarrierDescription,
    ) {
        set_image_memory_barrier(
            device.raw(),
            self.command_buffer,
            image,
            aspect_mask,
            old_layout,
            new_layout,
            desc,
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
            .render_area(crate::util::rect_to_vk(render_area).unwrap())
            .color_attachments(color_attachments)
            .layer_count(1);
        let rendering_info = if let Some(depth_attachment) = &depth_attahcment {
            rendering_info.depth_attachment(&depth_attachment.0).build()
        } else {
            rendering_info.build()
        };

        unsafe {
            device
                .dynamic_rendering()
                .cmd_begin_rendering(self.command_buffer, &rendering_info);
        }
    }

    pub fn end_rendering(&self, device: &Device) {
        unsafe {
            device
                .dynamic_rendering()
                .cmd_end_rendering(self.command_buffer)
        };
    }

    pub fn bind_graphics_pipeline(&self, device: &Device, pipeline: &GraphicsPipeline) {
        unsafe {
            device.raw().cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.common.pipeline(),
            )
        }
    }

    pub fn bind_scissor(&self, device: &Device, rect: Rect2D<i32, u32>) {
        unsafe {
            device.raw().cmd_set_scissor(
                self.command_buffer,
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
                self.command_buffer,
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

    pub fn bind_vertex_buffer(&self, device: &Device, buffer: &Buffer) {
        unsafe {
            device
                .raw()
                .cmd_bind_vertex_buffers(self.command_buffer, 0, &[buffer.raw], &[0])
        }
    }

    pub fn bind_index_buffer(&self, device: &Device, buffer: &Buffer) {
        unsafe {
            device.raw().cmd_bind_index_buffer(
                self.command_buffer,
                buffer.raw,
                0,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        device: &Device,
        pipeline: &GraphicsPipeline,
        first_set: u32,
        bind_groups: &[BindGroup],
    ) {
        let descriptor_sets =
            unsafe { std::mem::transmute::<&[BindGroup], &[vk::DescriptorSet]>(bind_groups) };
        unsafe {
            device.raw().cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.common.pipeline_layout(),
                first_set,
                descriptor_sets,
                &[],
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
                    self.command_buffer,
                    pipeline_common.pipeline_layout(),
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
        pipeline: &GraphicsPipeline,
        data: &T,
        idx: u32,
    ) -> Result<(), PipelineError> {
        self.push_constant(
            device,
            &pipeline.common,
            ShaderStage::Vertex,
            idx,
            util::as_u8_slice(data),
        )
    }

    pub fn set_fragment_bytes<T: Sized>(
        &self,
        device: &Device,
        pipeline: &GraphicsPipeline,
        data: &T,
        idx: u32,
    ) -> Result<(), PipelineError> {
        self.push_constant(
            device,
            &pipeline.common,
            ShaderStage::Fragment,
            idx,
            util::as_u8_slice(data),
        )
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
                self.command_buffer,
                index_count,
                1,
                first_index,
                vertex_offset,
                1,
            )
        }
    }

    pub fn copy_buffer_to_image(&self, device: &Device, buffer: &Buffer, image: &Image) {
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(image.desc.usage.into())
                    .layer_count(1)
                    .build(),
            )
            .image_extent(vk::Extent3D {
                width: image.size.width(),
                height: image.size.height(),
                depth: 1,
            })
            .build();

        unsafe {
            device.raw().cmd_copy_buffer_to_image(
                self.command_buffer,
                buffer.raw,
                image.raw,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions],
            )
        };
    }

    pub fn begin_label(&self, device: &Device, name: &str, color: [f32; 4]) {
        cmd_begin_label(device.instance().debug(), self.command_buffer, name, color);
    }

    pub fn end_label(&self, device: &Device) {
        cmd_end_label(device.instance().debug(), self.command_buffer);
    }

    pub fn insert_label(&self, device: &Device, name: &str, color: [f32; 4]) {
        cmd_insert_label(device.instance().debug(), self.command_buffer, name, color);
    }
}

pub struct CommandQueue {
    command_pool: vk::CommandPool,
    command_lists: [CommandList; MAX_FRAMES_IN_FLIGHT],
}

impl CommandQueue {
    pub fn new(device: &Device) -> Result<Self> {
        let command_pool = unsafe {
            device.raw().create_command_pool(
                &vk::CommandPoolCreateInfo {
                    flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    queue_family_index: device.queue_family_index(),
                    ..Default::default()
                },
                None,
            )
        }?;

        device.set_name(vk::ObjectType::COMMAND_POOL, command_pool, "Command Pool");

        let command_lists = {
            let mut lists = [CommandList::default(); MAX_FRAMES_IN_FLIGHT];
            for idx in 0..MAX_FRAMES_IN_FLIGHT {
                lists[idx] = CommandList::new(device, command_pool, Some(idx))?;
            }
            lists
        };

        Ok(Self {
            command_pool,
            command_lists,
        })
    }

    pub fn get_command_list(&self, device: &Device) -> Result<CommandList> {
        let cmd_list = self.command_lists[device.current_frame_in_flight()];
        cmd_list.begin(device)?;
        Ok(cmd_list)
    }

    pub fn get_immediate_command_list(&self, device: &Device) -> Result<CommandList> {
        let cmd_list = CommandList::new(device, self.command_pool, None)?;
        cmd_list.begin(device)?;
        Ok(cmd_list)
    }

    pub fn transition_image(
        &self,
        device: &Device,
        image: &Image,
        aspect_mask: ImageUsage,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<()> {
        let instant_command_list = self.get_immediate_command_list(device)?;
        instant_command_list.set_image_memory_barrier(
            device,
            image.raw,
            aspect_mask.into(),
            old_layout.into(),
            new_layout.into(),
            Default::default(),
        );
        instant_command_list.end(device)?;
        instant_command_list.immediate_submit(device, device.present_queue())?;
        instant_command_list.reset(device)?;

        Ok(())
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.raw().destroy_command_pool(self.command_pool, None);
        }
    }
}

pub fn set_image_memory_barrier(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    aspect_mask: vk::ImageAspectFlags,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    desc: ImageBarrierDescription,
) {
    let depth_stage_mask =
        vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;

    let sampled_stage_mask = vk::PipelineStageFlags::VERTEX_SHADER
        | vk::PipelineStageFlags::FRAGMENT_SHADER
        | vk::PipelineStageFlags::COMPUTE_SHADER;

    let mut src_stage_mask = vk::PipelineStageFlags::TOP_OF_PIPE;
    let mut dst_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;

    let mut src_access_mask = vk::AccessFlags::empty();
    let mut dst_access_mask = vk::AccessFlags::empty();

    match old_layout {
        vk::ImageLayout::UNDEFINED => {}

        vk::ImageLayout::GENERAL => {
            src_stage_mask = vk::PipelineStageFlags::ALL_COMMANDS;
            src_access_mask = vk::AccessFlags::MEMORY_WRITE;
        }

        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
            src_stage_mask = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            src_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
        }

        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
            src_stage_mask = depth_stage_mask;
            src_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
        }

        vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => {
            src_stage_mask = depth_stage_mask | sampled_stage_mask;
        }

        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
            src_stage_mask = sampled_stage_mask;
        }

        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => {
            src_stage_mask = vk::PipelineStageFlags::TRANSFER;
        }

        vk::ImageLayout::TRANSFER_DST_OPTIMAL => {
            src_stage_mask = vk::PipelineStageFlags::TRANSFER;
            src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        }

        vk::ImageLayout::PREINITIALIZED => {
            src_stage_mask = vk::PipelineStageFlags::HOST;
            src_access_mask = vk::AccessFlags::HOST_WRITE;
        }

        vk::ImageLayout::PRESENT_SRC_KHR => {}

        _ => unreachable!(),
    };

    match new_layout {
        vk::ImageLayout::GENERAL => {
            dst_stage_mask = vk::PipelineStageFlags::ALL_COMMANDS;
            dst_access_mask = vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE;
        }

        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
            dst_stage_mask = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            dst_access_mask =
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
        }

        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
            dst_stage_mask = depth_stage_mask;
            dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
        }

        vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => {
            dst_stage_mask = depth_stage_mask | sampled_stage_mask;
            dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::SHADER_READ
                | vk::AccessFlags::INPUT_ATTACHMENT_READ;
        }

        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
            dst_stage_mask = sampled_stage_mask;
            dst_access_mask = vk::AccessFlags::SHADER_READ | vk::AccessFlags::INPUT_ATTACHMENT_READ;
        }

        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => {
            dst_stage_mask = vk::PipelineStageFlags::TRANSFER;
            dst_access_mask = vk::AccessFlags::TRANSFER_READ;
        }

        vk::ImageLayout::TRANSFER_DST_OPTIMAL => {
            dst_stage_mask = vk::PipelineStageFlags::TRANSFER;
            dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        }

        vk::ImageLayout::PRESENT_SRC_KHR => {}

        _ => unreachable!(),
    }

    let image_memory_barrier = vk::ImageMemoryBarrier {
        src_access_mask,
        dst_access_mask,
        old_layout,
        new_layout,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        image,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: desc.base_mip_level,
            level_count: desc.level_count,
            base_array_layer: desc.base_array_layer,
            layer_count: desc.layer_count,
        },
        ..Default::default()
    };

    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage_mask,
            dst_stage_mask,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[image_memory_barrier],
        )
    }
}
