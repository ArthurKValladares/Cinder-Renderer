use std::path::Path;

use anyhow::Result;
use cinder::{
    App, AttachmentType, BindGroup, BindGroupBindInfo, BindGroupWriteData, Buffer,
    BufferDescription, BufferUsage, Bump, Cinder, GraphicsPipeline, InitContext, Layout,
    PipelineError, RenderGraph, RenderPass, Renderer, ResourceId,
};
use math::size::Size2D;

use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/hot_reload_shader_structs.rs"
));

pub struct ShaderHotReloadSample {
    pipeline_handle: ResourceId<GraphicsPipeline>,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl App for ShaderHotReloadSample {
    fn new(context: InitContext<'_>) -> Result<Self> {
        //
        // Setup Shader Hot-reloading
        //
        let vertex_shader_handle = context.renderer.resource_manager.insert_shader(
            context.renderer.device.create_shader(
                include_bytes!("../shaders/spv/hot_reload.vert.spv"),
                Default::default(),
            )?,
        );
        let fragment_shader_handle = context.renderer.resource_manager.insert_shader(
            context.renderer.device.create_shader(
                include_bytes!("../shaders/spv/hot_reload.frag.spv"),
                Default::default(),
            )?,
        );
        let pipeline = context.renderer.device.create_graphics_pipeline(
            context
                .renderer
                .resource_manager
                .shaders
                .get(vertex_shader_handle)
                .unwrap(),
            context
                .renderer
                .resource_manager
                .shaders
                .get(fragment_shader_handle),
            Default::default(),
        )?;
        let bind_group = BindGroup::new(
            &context.renderer.device,
            pipeline.bind_group_data(0).unwrap(),
        )?;
        let pipeline_handle = context
            .renderer
            .resource_manager
            .insert_graphics_pipeline(pipeline);
        context.shader_hot_reloader.set_graphics(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("shaders")
                .join("hot_reload.vert")
                .canonicalize()?,
            vertex_shader_handle,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("shaders")
                .join("hot_reload.frag")
                .canonicalize()?,
            fragment_shader_handle,
            pipeline_handle,
        )?;

        let sampler = context.renderer.device.create_sampler(Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        let texture = context.renderer.device.create_image_with_data_immediate(
            Size2D::new(width, height),
            &image_data,
            &context.renderer.command_queue,
            Default::default(),
        )?;
        let _pipeline = context
            .renderer
            .resource_manager
            .graphics_pipelines
            .get(pipeline_handle)
            .unwrap();
        context
            .renderer
            .device
            .write_bind_group(&[BindGroupBindInfo {
                group: bind_group,
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(texture.bind_info(
                    &sampler,
                    Layout::ShaderReadOnly,
                    None,
                )),
            }])?;
        let vertex_buffer = context.renderer.device.create_buffer_with_data(
            &[
                HotReloadVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                HotReloadVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                HotReloadVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                HotReloadVertex {
                    i_pos: [-0.5, 0.5],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = context.renderer.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        //
        // Add resources to ResourceManager
        //
        context.renderer.resource_manager.insert_sampler(sampler);
        context.renderer.resource_manager.insert_image(texture);

        Ok(Self {
            pipeline_handle,
            bind_group,
            vertex_buffer,
            index_buffer,
        })
    }

    fn draw<'a>(
        &'a mut self,
        allocator: &'a Bump,
        graph: &mut RenderGraph<'a>,
    ) -> anyhow::Result<()> {
        graph.add_pass(
            &allocator,
            RenderPass::new(allocator)
                .add_color_attachment(AttachmentType::SwapchainImage, Default::default())
                .set_callback(allocator, |renderer, cmd_list| {
                    let pipeline = renderer
                        .resource_manager
                        .graphics_pipelines
                        .get(self.pipeline_handle)
                        .ok_or(PipelineError::InvalidPipelineHandle)?;
                    cmd_list.bind_graphics_pipeline(&renderer.device, pipeline);
                    cmd_list.bind_index_buffer(&renderer.device, &self.index_buffer);
                    cmd_list.bind_vertex_buffer(&renderer.device, &self.vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        pipeline,
                        0,
                        &[self.bind_group],
                    );
                    cmd_list.draw_offset(&renderer.device, 6, 0, 0);

                    Ok(())
                }),
        );
        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.vertex_buffer.destroy(&renderer.device);
        self.index_buffer.destroy(&renderer.device);
        Ok(())
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "shader-hot-reload",
            ..Default::default()
        },
    )
    .unwrap();
    let mut cinder = Cinder::<ShaderHotReloadSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
