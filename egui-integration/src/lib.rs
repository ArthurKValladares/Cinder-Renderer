use anyhow::Result;
use cinder::{
    cinder::EguiConstants,
    context::{
        render_context::{RenderAttachment, RenderContext},
        upload_context::UploadContext,
    },
    device::Device,
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupPool, BindGroupWriteData},
        buffer::{vk::Fence, Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageViewDescription, Usage},
        memory::MemoryType,
        pipeline::graphics::{ColorBlendState, GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        shader::ShaderStage,
    },
    util::MemoryMappablePointer,
    view::{Drawable, View},
};
use core::panic;
pub use egui;
use egui::{
    epaint::{ImageDelta, Primitive},
    ClippedPrimitive, ImageData, Mesh, PaintCallbackInfo, TextureId, TexturesDelta,
};
use math::{point::Point2D, rect::Rect2D, size::Size2D};
use std::collections::HashMap;
use util::{as_u8_slice, size_of_slice};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

static VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 4;
static INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 2;

type DrawCallback = dyn for<'a, 'b> Fn(PaintCallbackInfo, &Device) + Sync + Send;

pub struct EguiCallbackFn {
    pub draw: Box<DrawCallback>,
}

// TODO: Share image buffer with rest of the codebase
pub struct EguiIntegration {
    egui_context: egui::Context,
    egui_winit: egui_winit::State,
    pipeline: GraphicsPipeline,
    // TODO: won't need separate pool in the future, set will be a part of GraphicsPipeline?
    _bind_group_pool: BindGroupPool,
    bind_group_set: BindGroup,
    sampler: Sampler,
    image_staging_buffer: Option<Buffer>,
    image_map: HashMap<TextureId, Image>,
    vertex_buffers: Vec<Buffer>,
    index_buffers: Vec<Buffer>,
}

impl EguiIntegration {
    pub fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        device: &Device,
        view: &View,
        surface_format: Format,
        visuals: egui::Visuals,
        pixels_per_point: f32,
    ) -> Result<Self> {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(visuals);
        let mut egui_winit = egui_winit::State::new(event_loop);
        egui_context.set_pixels_per_point(pixels_per_point);
        egui_winit.set_pixels_per_point(pixels_per_point);

        let bind_group_pool = BindGroupPool::new(&device).unwrap();
        let pipeline = device.create_graphics_pipeline(
            device.create_shader(include_bytes!("../shaders/spv/egui.vert.spv"))?,
            device.create_shader(include_bytes!("../shaders/spv/egui.frag.spv"))?,
            GraphicsPipelineDescription {
                blending: ColorBlendState::pma(),
                surface_format,
                ..Default::default()
            },
        )?;
        let bind_group_set = BindGroup::new(
            &device,
            &bind_group_pool,
            &pipeline.common.bind_group_layouts()[0],
            true,
        )
        .unwrap();

        let sampler = device.create_sampler()?;

        let (vertex_buffers, index_buffers) = {
            let len = view.drawables_len();
            let mut vertex_buffers = Vec::with_capacity(len);
            let mut index_buffers = Vec::with_capacity(len);
            for _ in 0..len {
                let vertex_buffer = device.create_buffer(
                    VERTEX_BUFFER_SIZE,
                    BufferDescription {
                        usage: BufferUsage::VERTEX,
                        memory_ty: MemoryType::CpuVisible,
                    },
                )?;
                vertex_buffers.push(vertex_buffer);

                let index_buffer = device.create_buffer(
                    INDEX_BUFFER_SIZE,
                    BufferDescription {
                        usage: BufferUsage::INDEX,
                        memory_ty: MemoryType::CpuVisible,
                    },
                )?;
                index_buffers.push(index_buffer);
            }
            (vertex_buffers, index_buffers)
        };

        Ok(Self {
            egui_context,
            egui_winit,
            sampler,
            _bind_group_pool: bind_group_pool,
            bind_group_set,
            pipeline,
            image_staging_buffer: None,
            image_map: Default::default(),
            vertex_buffers,
            index_buffers,
        })
    }

    pub fn context(&self) -> &egui::Context {
        &self.egui_context
    }

    pub fn on_event(&mut self, event: &WindowEvent<'_>) {
        let response = self.egui_winit.on_event(&self.egui_context, event);
        if response.repaint {
            // TODO
        }
    }

    pub fn run(
        &mut self,
        device: &Device,
        drawable: Drawable,
        upload_context: &UploadContext,
        upload_fence: Fence,
        render_context: &RenderContext,
        render_area: Rect2D<i32, u32>,
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
        self.set_textures(device, upload_context, upload_fence, &textures_delta)?;

        self.paint(
            device,
            drawable,
            render_context,
            render_area,
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
        device: &Device,
        drawable: Drawable,
        render_context: &RenderContext,
        render_area: Rect2D<i32, u32>,
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
            &device,
            render_area,
            &[RenderAttachment::color(drawable, Default::default())],
            None,
        );
        {
            render_context.bind_graphics_pipeline(device, &self.pipeline);
            render_context.bind_vertex_buffer(device, vertex_buffer);
            render_context.bind_index_buffer(device, index_buffer);
            render_context.bind_viewport(
                device,
                Rect2D::from_width_height(size.width, size.height),
                false,
            );

            render_context.set_vertex_bytes(
                device,
                &EguiConstants {
                    screen_size: [
                        size.width as f32 / pixels_per_point,
                        size.height as f32 / pixels_per_point,
                    ],
                },
                &self.pipeline.common,
                0,
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
                        device,
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
                            device,
                            render_context,
                            present_index,
                            mesh,
                            &mut vertex_buffer_ptr,
                            &mut vertex_base,
                            &mut index_buffer_ptr,
                            &mut index_base,
                        )?;
                    }
                    Primitive::Callback(callback) => {
                        let cbfn =
                            if let Some(c) = callback.callback.downcast_ref::<EguiCallbackFn>() {
                                c
                            } else {
                                println!(
                                    "Could not cast callback to type required by the egui backend"
                                );
                                continue;
                            };
                        let screen_size_px = {
                            let size = window.inner_size();
                            [size.width, size.height]
                        };
                        let paint_callback_info = PaintCallbackInfo {
                            viewport: callback.rect,
                            clip_rect: *clip_rect,
                            pixels_per_point,
                            screen_size_px,
                        };
                        (cbfn.draw)(paint_callback_info, device);
                    }
                }
            }
        }
        render_context.end_rendering(&device);

        Ok(())
    }

    fn paint_mesh(
        &mut self,
        device: &Device,
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

        render_context.bind_descriptor_sets(
            device,
            &self.pipeline.common,
            &[self.bind_group_set.0],
            false,
        );

        let index = match mesh.texture_id {
            TextureId::Managed(index) => index as usize,
            TextureId::User(_) => unimplemented!(),
        };

        render_context
            .set_fragment_bytes(device, &index, &self.pipeline.common, 0)
            .unwrap();

        render_context.draw_offset(device, indices.len() as u32, *index_base, *vertex_base);

        *vertex_base += vertices.len() as i32;
        *index_base += indices.len() as u32;

        Ok(())
    }

    pub fn resize(&mut self, _device: &Device) -> Result<()> {
        Ok(())
    }

    pub fn clean(&mut self, _device: &Device) {
        // TODO
    }

    fn set_image_helper(
        &mut self,
        device: &Device,
        upload_context: &UploadContext,
        id: &TextureId,
        width: u32,
        height: u32,
        data: &[egui::Color32],
    ) -> Result<()> {
        // TODO: Revisit image abstraction
        if let Some(mut buffer) = self.image_staging_buffer.take() {
            buffer.clean(device);
        }
        let image_staging_buffer = device.create_buffer(
            size_of_slice(data),
            BufferDescription {
                usage: BufferUsage::TRANSFER_SRC,
                memory_ty: MemoryType::CpuVisible,
            },
        )?;
        image_staging_buffer.mem_copy(0, data)?;

        let mut image = device.create_image(ImageDescription {
            format: Format::R8_G8_B8_A8_Unorm,
            usage: Usage::Texture,
            size: Size2D::new(width, height),
        })?;
        let image_view_desc = ImageViewDescription {
            format: Format::R8_G8_B8_A8_Unorm,
            usage: Usage::Texture,
        };
        image.add_view(device, image_view_desc)?;
        upload_context.image_barrier_start(device, &image);
        upload_context.copy_buffer_to_image(device, &image_staging_buffer, &image);
        upload_context.image_barrier_end(device, &image);

        let index = match id {
            TextureId::Managed(index) => *index,
            TextureId::User(_) => unimplemented!(),
        };

        self.bind_group_set.write(
            device,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(image.bind_info(
                    &self.sampler,
                    image_view_desc,
                    index as u32,
                    cinder::resources::buffer::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // TODO: cleanup
                )),
            }],
        );

        self.image_map.insert(*id, image);
        self.image_staging_buffer = Some(image_staging_buffer);

        Ok(())
    }

    fn set_image(
        &mut self,
        device: &Device,
        upload_context: &UploadContext,
        id: &TextureId,
        delta: &ImageDelta,
    ) -> Result<()> {
        match &delta.image {
            ImageData::Color(color_data) => {
                let (width, height) = (color_data.size[0] as u32, color_data.size[1] as u32);

                self.set_image_helper(
                    device,
                    upload_context,
                    id,
                    width,
                    height,
                    &color_data.pixels,
                )
            }
            ImageData::Font(font_data) => {
                let (width, height) = (font_data.width() as u32, font_data.height() as u32);
                let data = font_data.srgba_pixels(Some(1.0)).collect::<Vec<_>>();

                self.set_image_helper(device, upload_context, id, width, height, &data)
            }
        }
    }

    fn set_textures(
        &mut self,
        device: &Device,
        context: &UploadContext,
        fence: Fence,
        textures_delta: &TexturesDelta,
    ) -> Result<()> {
        context.begin(device, fence)?;
        for (id, delta) in &textures_delta.set {
            self.set_image(device, context, id, delta)?;
        }
        context.end(device, fence, device.present_queue(), &[], &[], &[])?;
        Ok(())
    }

    fn free_textures(&mut self, _textures_delta: TexturesDelta) {}

    pub fn set_ui_scale(&mut self, scale: f32) {
        self.egui_context.set_pixels_per_point(scale);
        self.egui_winit.set_pixels_per_point(scale);
    }
}
