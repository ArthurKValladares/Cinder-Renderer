use std::{collections::HashMap, path::Path};

use anyhow::Result;
use cinder::{
    cinder::Cinder,
    context::render_context::RenderContext,
    resoruces::{
        bind_group::{BindGroupLayout, BindGroupLayoutBuilder, BindGroupSet, BindGroupType},
        buffer::Buffer,
        pipeline::{push_constant::PushConstant, GraphicsPipeline, GraphicsPipelineDescription},
        render_pass::{
            Layout, LayoutTransition, RenderPass, RenderPassAttachmentDesc, RenderPassDescription,
        },
        sampler::Sampler,
        shader::{ShaderDescription, ShaderStage},
        texture::{Format, Texture},
    },
};
use egui::{RawInput, TextureId, TexturesDelta};
use math::vec::Vec2;
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

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
    image_map: HashMap<TextureId, Texture>,
}

impl EguiIntegration {
    pub fn new<T>(event_loop: &EventLoopWindowTarget<T>, device: &mut Cinder) -> Result<Self> {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(egui::Visuals::light());
        let egui_winit = egui_winit::State::new(event_loop);

        let render_pass = device.create_render_pass(RenderPassDescription {
            color_attachments: [
                RenderPassAttachmentDesc::load_store(device.surface_format())
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

        let vertex_shader = device
            .create_shader(ShaderDescription {
                stage: ShaderStage::Vertex,
                path: Path::new("egui-integration/shaders/spv/egui.vert.spv"),
            })
            .expect("Could not create vertex shader");
        let fragment_shader = device
            .create_shader(ShaderDescription {
                stage: ShaderStage::Fragment,
                path: Path::new("egui-integration/shaders/spv/egui.frag.spv"),
            })
            .expect("Could not create fragment shader");

        let bind_group_layout = BindGroupLayoutBuilder::default()
            .bind_image(0, BindGroupType::ImageSampler, ShaderStage::Fragment)
            .build(device)?;
        let bind_group_set = BindGroupSet::allocate(device, &bind_group_layout)?;

        let pipeline = device.create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            render_pass: &render_pass,
            desc_set_layouts: vec![bind_group_layout.layout],
            push_constants: vec![&push_constant],
        })?;

        let sampler = device.create_sampler()?;

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
        })
    }

    pub fn on_event(&mut self, event: &WindowEvent<'_>) {
        self.egui_winit.on_event(&self.egui_context, event);
    }

    pub fn run(
        &mut self,
        device: &Cinder,
        context: &RenderContext,
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
        self.set_textures(device, context, &textures_delta);

        context.begin_render_pass(device, &self.render_pass, present_index);
        {}
        context.end_render_pass(device);

        // TODO: render
        self.free_textures(textures_delta);
    }

    pub fn resize(&mut self, device: &Cinder) -> Result<()> {
        // TODO: Clean old render pass
        device.clean_render_pass(&mut self.render_pass);
        self.render_pass = device.create_render_pass(RenderPassDescription {
            color_attachments: [
                RenderPassAttachmentDesc::load_store(device.surface_format())
                    .with_layout_transition(LayoutTransition {
                        initial_layout: Layout::ColorAttachment,
                        final_layout: Layout::Present,
                    }),
            ],
            depth_attachment: None,
        })?;
        Ok(())
    }

    pub fn clean(&mut self, device: &Cinder) {}

    fn gather_input(&mut self, window: &Window) -> RawInput {
        self.egui_winit.take_egui_input(window)
    }

    fn set_textures(
        &mut self,
        device: &Cinder,
        context: &RenderContext,
        textures_delta: &TexturesDelta,
    ) {
    }

    fn free_textures(&mut self, _textures_delta: TexturesDelta) {}
}
