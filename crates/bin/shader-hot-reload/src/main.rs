use std::path::Path;

use anyhow::Result;
use cinder::{
    App, AttachmentType, BindGroup, BindGroupBindInfo, BindGroupWriteData, Buffer,
    BufferDescription, BufferUsage, Bump, Cinder, GraphicsPipeline, Layout, PipelineError,
    RenderGraph, RenderPass, Renderer, ResourceId,
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
    fn new(renderer: &mut Renderer, _width: u32, _height: u32) -> Result<Self> {
        //
        // Setup Shader Hot-reloading
        //
        let vertex_shader_handle =
            renderer
                .resource_manager
                .insert_shader(renderer.device.create_shader(
                    include_bytes!("../shaders/spv/hot_reload.vert.spv"),
                    Default::default(),
                )?);
        let fragment_shader_handle =
            renderer
                .resource_manager
                .insert_shader(renderer.device.create_shader(
                    include_bytes!("../shaders/spv/hot_reload.frag.spv"),
                    Default::default(),
                )?);
        let pipeline = renderer.device.create_graphics_pipeline(
            renderer
                .resource_manager
                .shaders
                .get(vertex_shader_handle)
                .unwrap(),
            renderer
                .resource_manager
                .shaders
                .get(fragment_shader_handle),
            Default::default(),
        )?;
        let bind_group = BindGroup::new(&renderer.device, pipeline.bind_group_data(0).unwrap())?;
        let pipeline_handle = renderer.resource_manager.insert_graphics_pipeline(pipeline);
        renderer.shader_hot_reloader.set_graphics(
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

        let sampler = renderer.device.create_sampler(Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        let texture = renderer.device.create_image_with_data_immediate(
            Size2D::new(width, height),
            &image_data,
            &renderer.command_queue,
            Default::default(),
        )?;
        let _pipeline = renderer
            .resource_manager
            .graphics_pipelines
            .get(pipeline_handle)
            .unwrap();
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(texture.bind_info(
                &sampler,
                Layout::ShaderReadOnly,
                None,
            )),
        }])?;
        let vertex_buffer = renderer.device.create_buffer_with_data(
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
        let index_buffer = renderer.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        //
        // Add resources to ResourceManager
        //
        renderer.resource_manager.insert_sampler(sampler);
        renderer.resource_manager.insert_image(texture);

        renderer.init();

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

    fn update(&mut self, renderer: &mut Renderer) -> Result<()> {
        // TODO: This should probably also be automated/abstracted into cinder itself
        for update_data in renderer.shader_hot_reloader.drain()? {
            if let Some(pipeline_shader_set) = renderer
                .shader_hot_reloader
                .get_pipeline(update_data.shader_handle)
            {
                renderer.device.recreate_shader(
                    &mut renderer.resource_manager,
                    update_data.shader_handle,
                    &update_data.bytes,
                )?;
                renderer.device.recreate_graphics_pipeline(
                    &mut renderer.resource_manager,
                    pipeline_shader_set.pipeline_handle,
                    pipeline_shader_set.vertex_handle,
                    Some(pipeline_shader_set.fragment_handle),
                )?;
            }
        }
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
