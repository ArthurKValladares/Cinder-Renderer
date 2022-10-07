use super::ContextShared;
use crate::{
    cinder::{self, Cinder},
    resoruces::{
        buffer::Buffer,
        pipeline::{push_constant::PushConstant, GraphicsPipeline},
        render_pass::RenderPass,
    },
};
use anyhow::Result;
use ash::vk::{self};
use math::rect::Rect2D;

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

    pub fn end(&self, cinder: &Cinder) -> Result<()> {
        self.shared.end(cinder.device())
    }

    pub fn begin_render_pass(&self, cinder: &Cinder, render_pass: &RenderPass, present_index: u32) {
        let create_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass.render_pass)
            .framebuffer(render_pass.framebuffers[present_index as usize])
            .render_area(render_pass.render_area)
            .clear_values(&render_pass.clear_values);

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

    pub fn bind_scissor(&self, cinder: &Cinder, rect: Rect2D<u32>) {
        unsafe {
            cinder.device().cmd_set_scissor(
                self.shared.command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: rect.width(),
                        height: rect.height(),
                    },
                }],
            )
        }
    }

    pub fn bind_viewport(&self, cinder: &Cinder, rect: Rect2D<u32>) {
        unsafe {
            cinder.device().cmd_set_viewport(
                self.shared.command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0 as f32,
                    width: rect.width() as f32,
                    height: rect.height() as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            )
        }
    }

    // TODO: This should maybe be a user-land abstraction
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
        push_constant: &PushConstant,
        data: &[u8],
    ) {
        unsafe {
            cinder.device().cmd_push_constants(
                self.shared.command_buffer,
                pipeline.common.pipeline_layout,
                push_constant.stage.into(),
                push_constant.offset,
                data,
            );
        }
    }
}
