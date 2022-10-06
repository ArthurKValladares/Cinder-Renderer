use std::{collections::HashMap, path::Path};

use anyhow::Result;
use cinder::{
    cinder::Cinder,
    context::{
        render_context::{self, RenderContext},
        upload_context::UploadContext,
    },
    resoruces::{
        bind_group::{BindGroupLayout, BindGroupLayoutBuilder, BindGroupSet, BindGroupType},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{
            push_constant::PushConstant, GraphicsPipeline, GraphicsPipelineDescription,
            VertexAttributeDesc, VertexInputStateDesc,
        },
        render_pass::{
            Layout, LayoutTransition, RenderPass, RenderPassAttachmentDesc, RenderPassDescription,
        },
        sampler::Sampler,
        shader::{ShaderDescription, ShaderStage},
    },
};
use egui::{epaint::ImageDelta, ImageData, RawInput, TextureId, TexturesDelta};
use math::{size::Size2D, vec::Vec2};
use util::size_of_slice;
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
    bind_group_layout: BindGroupLayout,
    bind_group_set: BindGroupSet,
    pipeline: GraphicsPipeline,
    sampler: Sampler,
    image_staging_buffer: Option<Buffer>,
    image_map: HashMap<TextureId, Image>,
    vertex_buffers: Vec<Buffer>,
    index_buffers: Vec<Buffer>,
}

impl EguiIntegration {
    pub fn new<T>(event_loop: &EventLoopWindowTarget<T>, cinder: &mut Cinder) -> Result<Self> {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(egui::Visuals::light());
        let egui_winit = egui_winit::State::new(event_loop);

        let render_pass = cinder.create_render_pass(RenderPassDescription {
            color_attachments: [
                RenderPassAttachmentDesc::load_store(cinder.surface_format())
                    .with_layout_transition(LayoutTransition {
                        initial_layout: Layout::ColorAttachment,
                        final_layout: Layout::Present,
                    }),
            ],
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
                attributes: vec![
                    VertexAttributeDesc {
                        format: Format::R32_G32_SFloat,
                        offset: 0,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_G32_SFloat,
                        offset: 8,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_SFloat,
                        offset: 16,
                    },
                ],
            },
            render_pass: &render_pass,
            desc_set_layouts: vec![bind_group_layout.layout],
            push_constants: vec![&push_constant],
        })?;

        let sampler = cinder.create_sampler()?;

        let (vertex_buffers, index_buffers) = {
            let len = render_pass.framebuffers.len();
            let mut vertex_buffers = Vec::with_capacity(len);
            let mut index_buffers = Vec::with_capacity(len);
            for _ in 0..len {
                // TODO: Should these be GPU-side?
                let vertex_buffer = cinder.create_buffer(BufferDescription {
                    size: VERTEX_BUFFER_SIZE,
                    usage: BufferUsage::Index,
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })?;
                vertex_buffers.push(vertex_buffer);
                let index_buffer = cinder.create_buffer(BufferDescription {
                    size: INDEX_BUFFER_SIZE,
                    usage: BufferUsage::Vertex,
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
            bind_group_layout,
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
    ) {
        let raw_input = self.gather_input(window);
        // TODO: Hook up needs_repaint
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            repaint_after,
        } = self.egui_context.run(raw_input, f);

        let clipped_primitives = self.egui_context.tessellate(shapes);

        // TOOD: Separate this step maybe?
        self.egui_winit
            .handle_platform_output(window, &self.egui_context, platform_output);

        // TODO? Make this a separate step
        self.set_textures(cinder, upload_context, &textures_delta);

        render_context.begin_render_pass(cinder, &self.render_pass, present_index);
        {}
        render_context.end_render_pass(cinder);

        // TODO: render
        self.free_textures(textures_delta);
    }

    pub fn resize(&mut self, cinder: &Cinder) -> Result<()> {
        cinder.clean_render_pass(&mut self.render_pass);
        self.render_pass = cinder.create_render_pass(RenderPassDescription {
            color_attachments: [
                RenderPassAttachmentDesc::load_store(cinder.surface_format())
                    .with_layout_transition(LayoutTransition {
                        initial_layout: Layout::ColorAttachment,
                        final_layout: Layout::Present,
                    }),
            ],
            depth_attachment: None,
        })?;
        Ok(())
    }

    pub fn clean(&mut self, cinder: &Cinder) {}

    fn gather_input(&mut self, window: &Window) -> RawInput {
        self.egui_winit.take_egui_input(window)
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
            usage: BufferUsage::TransferSrc,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })?;
        cinder.copy_data_to_buffer(&image_staging_buffer, &data)?;
        cinder.bind_buffer(&image_staging_buffer)?;

        let image = cinder.create_image(ImageDescription {
            format: Format::R8_G8_B8_A8_Unorm,
            usage: Usage::Texture,
            size: Size2D::new(width, height),
        })?;

        upload_context.image_barrier_start(&cinder, &image);
        upload_context.copy_buffer_to_image(&cinder, &image_staging_buffer, &image);
        upload_context.image_barrier_end(&cinder, &image);

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
        context.end(cinder)?;
        cinder.submit_upload_work(context)?;
        Ok(())
    }

    fn free_textures(&mut self, _textures_delta: TexturesDelta) {}
}
