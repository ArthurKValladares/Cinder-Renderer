use std::path::PathBuf;

use anyhow::Result;
use cinder::{
    App, AttachmentType, BindGroup, BindGroupBindInfo, BindGroupWriteData, Buffer,
    BufferDescription, BufferUsage, Bump, Cinder, GraphicsPipeline, GraphicsPipelineDescription,
    ImageDescription, InitContext, Layout, RenderGraph, RenderPass, Renderer, SamplerDescription,
    ShaderDesc,
};
use math::size::Size2D;

use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/debug_shader_structs.rs"
));

pub struct DebugSample {
    pipeline: GraphicsPipeline,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl App for DebugSample {
    fn new(context: InitContext<'_>) -> Result<Self> {
        //
        // Create App Resources
        //
        let vertex_shader = context.renderer.device.create_shader(
            include_bytes!("../shaders/spv/debug.vert.spv"),
            ShaderDesc {
                name: Some("Debug Vertex Shader"),
            },
        )?;
        let fragment_shader = context.renderer.device.create_shader(
            include_bytes!("../shaders/spv/debug.frag.spv"),
            ShaderDesc {
                name: Some("Debug Fragment Shader"),
            },
        )?;
        let pipeline = context.renderer.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                name: Some(String::from("Debug Graphics Pipeline")),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(
            &context.renderer.device,
            pipeline.bind_group_data(0).unwrap(),
        )?;
        let sampler = context.renderer.device.create_sampler(SamplerDescription {
            name: Some("Debug Sampler"),
            ..Default::default()
        })?;
        let image_data = zero_copy_assets::try_decoded_file::<zero_copy_assets::ImageData>(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("rust.png"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("gen")
                .join("rust.adi"),
        )
        .unwrap();
        let texture = context.renderer.device.create_image_with_data_immediate(
            Size2D::new(image_data.width, image_data.height),
            &image_data.bytes,
            &context.renderer.command_queue,
            ImageDescription {
                name: Some("Debug Image"),
                ..Default::default()
            },
        )?;
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
                // Top-left
                DebugVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                // Top-right
                DebugVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                // Bottom-right
                DebugVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                // Bottom-left
                DebugVertex {
                    i_pos: [-0.5, 0.5],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                name: Some("Vertex Buffer"),
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = context.renderer.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                name: Some("Index Buffer"),
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;
        //
        // Add resources to ResourceManager
        //
        context.renderer.resource_manager.insert_sampler(sampler);
        context.renderer.resource_manager.insert_image(texture);

        //
        // Cleanup
        //
        vertex_shader.destroy(&context.renderer.device);
        fragment_shader.destroy(&context.renderer.device);

        Ok(Self {
            pipeline,
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
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.pipeline);
                    cmd_list.bind_index_buffer(&renderer.device, &self.index_buffer);
                    cmd_list.bind_vertex_buffer(&renderer.device, &self.vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipeline,
                        0,
                        &[self.bind_group],
                    );
                    cmd_list.insert_label(&renderer.device, "Draw Offset", [0.0, 1.0, 0.0, 1.0]);
                    cmd_list.insert_label(&renderer.device, "Draw Offset", [0.0, 1.0, 0.0, 1.0]);
                    cmd_list.draw_offset(&renderer.device, 6, 0, 0);

                    Ok(())
                }),
        );
        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.index_buffer.destroy(&renderer.device);
        self.vertex_buffer.destroy(&renderer.device);
        self.pipeline.destroy(&renderer.device);
        Ok(())
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "debug",
            ..Default::default()
        },
    )
    .unwrap();
    let mut cinder = Cinder::<DebugSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
