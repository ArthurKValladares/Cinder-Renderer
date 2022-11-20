use super::ContextShared;
use crate::{
    cinder::Cinder,
    device::Device,
    profiling::QueryPool,
    resoruces::{
        buffer::Buffer,
        pipeline::GraphicsPipeline,
        render_pass::{ClearValue, RenderPass},
        shader::ShaderStage,
    },
};
use anyhow::Result;
use ash::vk::{self};
use math::rect::Rect2D;
use thiserror::Error;

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

    pub fn begin_render_pass(
        &self,
        cinder: &Cinder,
        render_pass: &RenderPass,
        present_index: u32,
        render_area: Rect2D<i32, u32>,
        clear_values: &[ClearValue],
    ) {
        let render_area = vk::Rect2D {
            offset: vk::Offset2D {
                x: render_area.offset().x(),
                y: render_area.offset().y(),
            },
            extent: vk::Extent2D {
                width: render_area.width(),
                height: render_area.height(),
            },
        };
        let clear_values =
            unsafe { std::mem::transmute::<&[ClearValue], &[vk::ClearValue]>(clear_values) };

        let create_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass.render_pass)
            .framebuffer(render_pass.framebuffers[present_index as usize])
            .render_area(render_area.into())
            .clear_values(clear_values);

        unsafe {
            cinder.device().cmd_begin_render_pass(
                self.shared.command_buffer,
                &create_info,
                vk::SubpassContents::INLINE,
            )
        }
    }

    pub fn end_render_pass(&self, cinder: &Cinder) {
        unsafe {
            cinder
                .device()
                .cmd_end_render_pass(self.shared.command_buffer)
        }
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

    pub fn draw(&self, cinder: &Cinder, index_count: u32) {
        Self::draw_offset(&self, cinder, index_count, 0, 0)
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
}
