use anyhow::Result;
use cinder::{
    cinder::Cinder,
    context::{render_context::RenderContext, upload_context::UploadContext},
    resoruces::{
        bind_group::{
            BindGroupLayout, BindGroupLayoutBuilder, BindGroupSet, BindGroupSetBuilder,
            BindGroupType,
        },
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{
            push_constant::PushConstant, ColorBlendState, GraphicsPipeline,
            GraphicsPipelineDescription, VertexAttributeDesc, VertexInputStateDesc,
        },
        render_pass::{
            AttachmentLoadOp, AttachmentStoreOp, Layout, RenderPass, RenderPassAttachmentDesc,
            RenderPassDescription,
        },
        sampler::Sampler,
        shader::{ShaderDescription, ShaderStage},
    },
    util::MemoryMappablePointer,
};
pub use egui;
use egui::{
    epaint::{ImageDelta, Primitive},
    ClippedPrimitive, ImageData, Mesh, Rect, TextureId, TexturesDelta,
};
use math::{point::Point2D, rect::Rect2D, size::Size2D, vec::Vec2};
use smallvec::smallvec;
use std::{collections::HashMap, path::Path};
use util::{as_u8_slice, size_of_slice};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

static VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 4;
static INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 2;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct EguiPushConstantData {
    size: Vec2,
}

pub struct EguiIntegration {
    egui_context: egui::Context,
    egui_winit: egui_winit::State,
    render_pass: RenderPass,
    push_constant: PushConstant,
    _bind_group_layout: BindGroupLayout,
    bind_group_set: BindGroupSet,
    pipeline: GraphicsPipeline,
    sampler: Sampler,
    image_staging_buffer: Option<Buffer>,
    image_map: HashMap<TextureId, Image>,
    vertex_buffers: Vec<Buffer>,
    index_buffers: Vec<Buffer>,
}

impl EguiIntegration {
    pub fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        cinder: &mut Cinder,
        visuals: egui::Visuals,
    ) -> Result<Self> {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(visuals);
        let egui_winit = egui_winit::State::new(event_loop);

        let render_pass = cinder.create_render_pass(RenderPassDescription {
            color_attachment: RenderPassAttachmentDesc::new(cinder.surface_format())
                .load_op(AttachmentLoadOp::Load)
                .store_op(AttachmentStoreOp::Store)
                .initial_layout(Layout::ColorAttachment)
                .final_layout(Layout::Present),
            depth_attachment: None,
        })?;

        let push_constant = PushConstant {
            stage: ShaderStage::Vertex,
            offset: 0,
            size: std::mem::size_of::<EguiPushConstantData>() as u32,
        };

        let vertex_shader = cinder.create_shader(ShaderDescription {
            stage: ShaderStage::Vertex,
            path: Path::new("egui-integration/shaders/spv/egui.vert.spv"),
        })?;
        let fragment_shader = cinder.create_shader(ShaderDescription {
            stage: ShaderStage::Fragment,
            path: Path::new("egui-integration/shaders/spv/egui.frag.spv"),
        })?;

        let bind_group_layout = BindGroupLayoutBuilder::default()
            .bind_image(0, BindGroupType::ImageSampler, ShaderStage::Fragment)
            .build(cinder)?;
        let bind_group_set = BindGroupSet::allocate(cinder, &bind_group_layout)?;

        let pipeline = cinder.create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            vertex_state: VertexInputStateDesc {
                binding: 0,
                stride: 4 * std::mem::size_of::<f32>() as u32
                    + 4 * std::mem::size_of::<u8>() as u32,
                attributes: smallvec![
                    VertexAttributeDesc {
                        format: Format::R32_G32_SFloat,
                        offset: 0,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_G32_SFloat,
                        offset: 8,
                    },
                    VertexAttributeDesc {
                        format: Format::R8_G8_B8_A8_Unorm,
                        offset: 16,
                    },
                ],
            },
            blending: ColorBlendState::pma(),
            render_pass: &render_pass,
            desc_set_layouts: vec![bind_group_layout.layout],
            push_constants: vec![&push_constant],
            depth_testing_enabled: false,
            backface_culling: false,
        })?;

        let sampler = cinder.create_sampler()?;

        let (vertex_buffers, index_buffers) = {
            let len = render_pass.framebuffers.len();
            let mut vertex_buffers = Vec::with_capacity(len);
            let mut index_buffers = Vec::with_capacity(len);
            for _ in 0..len {
                let vertex_buffer = cinder.create_buffer(BufferDescription {
                    size: VERTEX_BUFFER_SIZE,
                    usage: BufferUsage::empty().vertex(),
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })?;
                vertex_buffers.push(vertex_buffer);

                let index_buffer = cinder.create_buffer(BufferDescription {
                    size: INDEX_BUFFER_SIZE,
                    usage: BufferUsage::empty().index(),
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })?;
                index_buffers.push(index_buffer);
            }
            (vertex_buffers, index_buffers)
        };

        Ok(Self {
            egui_context,
            egui_winit,
            render_pass,
            sampler,
            push_constant,
            _bind_group_layout: bind_group_layout,
            bind_group_set,
            pipeline,
            image_staging_buffer: None,
            image_map: Default::default(),
            vertex_buffers,
            index_buffers,
        })
    }

    pub fn on_event(&mut self, event: &WindowEvent<'_>) {
        self.egui_winit.on_event(&self.egui_context, event);
    }

    pub fn run(
        &mut self,
        cinder: &Cinder,
        upload_context: &UploadContext,
        render_context: &RenderContext,
        present_index: u32,
        window: &Window,
        f: impl FnOnce(&egui::Context),
    ) -> Result<()> {
        self.egui_winit
            .set_pixels_per_point(self.egui_context.pixels_per_point());

        let raw_input = self.egui_winit.take_egui_input(window);
        // TODO: Hook up repaint_after
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            repaint_after: _,
        } = self.egui_context.run(raw_input, f);

        let clipped_primitives = self.egui_context.tessellate(shapes);

        // TOOD: Separate this step maybe?
        self.egui_winit
            .handle_platform_output(window, &self.egui_context, platform_output);

        // TODO? Make this a separate step
        self.set_textures(cinder, upload_context, &textures_delta)?;

        self.paint(
            cinder,
            render_context,
            window,
            present_index,
            self.egui_context.pixels_per_point(),
            &clipped_primitives,
        )?;

        self.free_textures(textures_delta);

        Ok(())
    }

    fn paint(
        &mut self,
        cinder: &Cinder,
        render_context: &RenderContext,
        window: &Window,
        present_index: u32,
        pixels_per_point: f32,
        clipped_primitives: &[ClippedPrimitive],
    ) -> Result<()> {
        let size = window.inner_size();

        let mut vertex_buffer_ptr = self.vertex_buffers[present_index as usize].ptr().unwrap();
        let mut index_buffer_ptr = self.index_buffers[present_index as usize].ptr().unwrap();

        let mut vertex_base = 0;
        let mut index_base = 0;

        let vertex_buffer = &self.vertex_buffers[present_index as usize];
        let index_buffer = &self.index_buffers[present_index as usize];

        render_context.begin_render_pass(
            cinder,
            &self.render_pass,
            present_index,
            cinder.surface_rect(),
            &[],
        );
        {
            render_context.bind_graphics_pipeline(cinder, &self.pipeline);
            render_context.bind_vertex_buffer(cinder, vertex_buffer);
            render_context.bind_index_buffer(cinder, index_buffer);
            render_context.bind_viewport(
                cinder,
                Rect2D::from_width_height(size.width, size.height),
                false,
            );

            render_context.push_constant(
                cinder,
                &self.pipeline,
                &self.push_constant,
                as_u8_slice(&EguiPushConstantData {
                    size: Vec2::new(
                        size.width as f32 / pixels_per_point,
                        size.height as f32 / pixels_per_point,
                    ),
                }),
            );

            for egui::ClippedPrimitive {
                clip_rect,
                primitive,
            } in clipped_primitives
            {
                {
                    let min = {
                        let min = clip_rect.min;

                        egui::Pos2 {
                            x: f32::clamp(min.x * pixels_per_point, 0.0, size.width as f32),
                            y: f32::clamp(min.y * pixels_per_point, 0.0, size.height as f32),
                        }
                    };
                    let max = {
                        let max = clip_rect.max;
                        egui::Pos2 {
                            x: f32::clamp(max.x * pixels_per_point, min.x, size.width as f32),
                            y: f32::clamp(max.y * pixels_per_point, min.y, size.height as f32),
                        }
                    };
                    render_context.bind_scissor(
                        cinder,
                        Rect2D::from_offset_and_size(
                            Point2D::new(min.x.round() as i32, min.y.round() as i32),
                            Size2D::new(
                                (max.x.round() - min.x) as u32,
                                (max.y.round() - min.y) as u32,
                            ),
                        ),
                    );
                }

                match primitive {
                    Primitive::Mesh(mesh) => {
                        self.paint_mesh(
                            cinder,
                            render_context,
                            present_index,
                            mesh,
                            &mut vertex_buffer_ptr,
                            &mut vertex_base,
                            &mut index_buffer_ptr,
                            &mut index_base,
                        )?;
                    }
                    Primitive::Callback(_) => {
                        todo!("Custom rendering callbacks are not implemented");
                    }
                }
            }
        }
        render_context.end_render_pass(cinder);

        Ok(())
    }

    fn paint_mesh(
        &mut self,
        cinder: &Cinder,
        render_context: &RenderContext,
        present_index: u32,
        mesh: &Mesh,
        vertex_buffer_ptr: &mut MemoryMappablePointer,
        vertex_base: &mut i32,
        index_buffer_ptr: &mut MemoryMappablePointer,
        index_base: &mut u32,
    ) -> Result<()> {
        let vertices = &mesh.vertices;
        let vertex_copy_size = std::mem::size_of_val(&vertices[0]) * vertices.len();

        let indices = &mesh.indices;
        let index_copy_size = std::mem::size_of_val(&indices[0]) * indices.len();

        vertex_buffer_ptr.copy_from(vertices, vertex_copy_size);
        index_buffer_ptr.copy_from(indices, index_copy_size);

        let vertex_buffer_ptr_next = vertex_buffer_ptr.add(vertex_copy_size);
        let index_buffer_ptr_next = index_buffer_ptr.add(index_copy_size);

        if vertex_buffer_ptr_next
            >= self.vertex_buffers[present_index as usize]
                .end_ptr()
                .unwrap()
            || index_buffer_ptr_next
                >= self.index_buffers[present_index as usize]
                    .end_ptr()
                    .unwrap()
        {
            panic!("egui out of memory");
        }

        vertex_buffer_ptr.copy_from(&vertices, vertex_copy_size);
        index_buffer_ptr.copy_from(&indices, index_copy_size);

        *vertex_buffer_ptr = vertex_buffer_ptr_next;
        *index_buffer_ptr = index_buffer_ptr_next;
        if let egui::TextureId::User(_id) = mesh.texture_id {
            todo!();
        } else {
            render_context.bind_descriptor_sets(cinder, &self.pipeline, &[self.bind_group_set.set]);
        }

        render_context.draw_offset(cinder, indices.len() as u32, *index_base, *vertex_base);

        *vertex_base += vertices.len() as i32;
        *index_base += indices.len() as u32;

        Ok(())
    }

    pub fn resize(&mut self, cinder: &Cinder) -> Result<()> {
        cinder.clean_render_pass(&mut self.render_pass);
        self.render_pass = cinder.create_render_pass(RenderPassDescription {
            color_attachment: RenderPassAttachmentDesc::new(cinder.surface_format())
                .load_op(AttachmentLoadOp::Load)
                .store_op(AttachmentStoreOp::Store)
                .initial_layout(Layout::ColorAttachment)
                .final_layout(Layout::Present),
            depth_attachment: None,
        })?;
        Ok(())
    }

    pub fn clean(&mut self, _cinder: &Cinder) {
        // TODO
    }

    fn set_image(
        &mut self,
        cinder: &Cinder,
        upload_context: &UploadContext,
        id: &TextureId,
        delta: &ImageDelta,
    ) -> Result<()> {
        let ((width, height), data) = match &delta.image {
            ImageData::Color(_) => todo!(),
            ImageData::Font(font_data) => {
                let dimensions = (font_data.width() as u32, font_data.height() as u32);
                let data = font_data.srgba_pixels(1.0).collect::<Vec<_>>();
                (dimensions, data)
            }
        };

        // TODO: Revisit image abstraction
        if let Some(mut buffer) = self.image_staging_buffer.take() {
            buffer.clean(cinder.device());
        }
        let image_staging_buffer = cinder.create_buffer(BufferDescription {
            size: size_of_slice(&data),
            usage: BufferUsage::empty().transfer_src(),
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })?;
        image_staging_buffer.mem_copy(&data)?;

        let image = cinder.create_image(ImageDescription {
            format: Format::R8_G8_B8_A8_Unorm,
            usage: Usage::Texture,
            size: Size2D::new(width, height),
        })?;

        upload_context.image_barrier_start(&cinder, &image);
        upload_context.copy_buffer_to_image(&cinder, &image_staging_buffer, &image);
        upload_context.image_barrier_end(&cinder, &image);

        BindGroupSetBuilder::default()
            .bind_image(
                0,
                &image.bind_info(&self.sampler),
                BindGroupType::ImageSampler,
            )
            .update(cinder, &self.bind_group_set);

        self.image_map.insert(*id, image);
        self.image_staging_buffer = Some(image_staging_buffer);

        Ok(())
    }

    fn set_textures(
        &mut self,
        cinder: &Cinder,
        context: &UploadContext,
        textures_delta: &TexturesDelta,
    ) -> Result<()> {
        context.begin(cinder)?;
        for (id, delta) in &textures_delta.set {
            self.set_image(cinder, context, id, delta)?;
        }
        context.end(
            cinder,
            cinder.setup_fence(),
            cinder.present_queue(),
            &[],
            &[],
            &[],
        )?;
        Ok(())
    }

    fn free_textures(&mut self, _textures_delta: TexturesDelta) {}

    pub fn set_ui_scale(&mut self, scale: f32) {
        self.egui_context.set_pixels_per_point(scale);
        self.egui_winit.set_pixels_per_point(scale);
    }
}
