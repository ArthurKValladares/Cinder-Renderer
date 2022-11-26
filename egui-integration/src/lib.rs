use anyhow::Result;
use cinder::{
    cinder::Cinder,
    cinder::Eguiconstants,
    context::{
        render_context::{
            AttachmentLoadOp, AttachmentStoreOp, Layout, RenderAttachment, RenderContext,
        },
        upload_context::UploadContext,
    },
    resoruces::{
        bind_group::{BindGroupSet, BindGroupType, BindGroupWriteBuilder},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{ColorBlendState, GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        shader::{ShaderDescription, ShaderStage},
    },
    util::MemoryMappablePointer,
};
pub use egui;
use egui::{
    epaint::{ImageDelta, Primitive},
    ClippedPrimitive, ImageData, Mesh, TextureId, TexturesDelta,
};
use math::{point::Point2D, rect::Rect2D, size::Size2D};
use std::collections::HashMap;
use util::{as_u8_slice, size_of_slice};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

static VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 4;
static INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 2;

pub struct EguiIntegration {
    egui_context: egui::Context,
    egui_winit: egui_winit::State,
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
        pixels_per_point: f32,
    ) -> Result<Self> {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(visuals);
        let mut egui_winit = egui_winit::State::new(event_loop);
        egui_context.set_pixels_per_point(pixels_per_point);
        egui_winit.set_pixels_per_point(pixels_per_point);

        let vertex_shader = cinder.create_shader(ShaderDescription {
            bytes: include_bytes!("../shaders/spv/egui.vert.spv"),
        })?;
        let fragment_shader = cinder.create_shader(ShaderDescription {
            bytes: include_bytes!("../shaders/spv/egui.frag.spv"),
        })?;

        let pipeline = cinder.create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            blending: ColorBlendState::pma(),
            depth_testing_enabled: false,
            backface_culling: false,
            uses_depth: false,
        })?;
        // TODO: bind group layout stuff is bad here
        let bind_group_set = BindGroupSet::allocate(cinder, &pipeline.bind_group_layouts()[0])?;

        let sampler = cinder.create_sampler()?;

        let (vertex_buffers, index_buffers) = {
            let len = cinder.swapchain().present_image_views.len();
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
            sampler,
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

        render_context.begin_rendering(
            &cinder,
            cinder.surface_rect(), // TODO: Might be able to use a better rect
            &[RenderAttachment::color(cinder.swapchain(), present_index)
                .load_op(AttachmentLoadOp::Load)
                .store_op(AttachmentStoreOp::Store)
                .layout(Layout::ColorAttachment)],
            None,
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
                ShaderStage::Vertex,
                0,
                as_u8_slice(&Eguiconstants {
                    screen_size: [
                        size.width as f32 / pixels_per_point,
                        size.height as f32 / pixels_per_point,
                    ],
                }),
            )?;

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
        render_context.end_rendering(&cinder);

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
        // TODO: Use staging buffer
        image_staging_buffer.mem_copy(0, &data)?;

        let image = cinder.create_image(ImageDescription {
            format: Format::R8_G8_B8_A8_Unorm,
            usage: Usage::Texture,
            size: Size2D::new(width, height),
        })?;

        upload_context.image_barrier_start(&cinder, &image);
        upload_context.copy_buffer_to_image(&cinder, &image_staging_buffer, &image);
        upload_context.image_barrier_end(&cinder, &image);

        BindGroupWriteBuilder::default()
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
