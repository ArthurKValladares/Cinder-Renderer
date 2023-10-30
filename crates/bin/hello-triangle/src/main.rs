use cinder::{
    App, AttachmentType, Buffer, BufferDescription, BufferUsage, Bump, Cinder, GraphicsPipeline,
    InitContext, RenderGraph, RenderPass, Renderer,
};
use math::{mat::Mat4, vec::Vec3};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/triangle_shader_structs.rs"
));

pub struct HelloTriangle {
    pipeline: GraphicsPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl App for HelloTriangle {
    fn new(context: InitContext<'_>) -> anyhow::Result<Self> {
        let vertex_shader = context.renderer.device.create_shader(
            include_bytes!("../shaders/spv/triangle.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = context.renderer.device.create_shader(
            include_bytes!("../shaders/spv/triangle.frag.spv"),
            Default::default(),
        )?;
        let pipeline = context.renderer.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            Default::default(),
        )?;

        let vertex_buffer = context.renderer.device.create_buffer_with_data(
            &[
                TriangleVertex {
                    i_pos: [0.0, 0.5],
                    i_color: [1.0, 0.0, 0.0, 1.0],
                },
                TriangleVertex {
                    i_pos: [-0.5, -0.5],
                    i_color: [0.0, 1.0, 0.0, 1.0],
                },
                TriangleVertex {
                    i_pos: [0.5, -0.5],
                    i_color: [0.0, 0.0, 1.0, 1.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = context.renderer.device.create_buffer_with_data(
            &[0, 1, 2],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        vertex_shader.destroy(&context.renderer.device);
        fragment_shader.destroy(&context.renderer.device);

        Ok(Self {
            pipeline,
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
            allocator,
            RenderPass::new(allocator)
                .add_color_attachment(AttachmentType::SwapchainImage, Default::default())
                .set_callback(allocator, |cinder, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&cinder.device, &self.pipeline);
                    cmd_list.bind_index_buffer(&cinder.device, &self.index_buffer);
                    cmd_list.bind_vertex_buffer(&cinder.device, &self.vertex_buffer);
                    cmd_list.set_vertex_bytes(
                        &cinder.device,
                        &self.pipeline,
                        &Mat4::rotate(
                            (cinder.init_time().elapsed().as_secs_f32() / 5.0)
                                * (2.0 * std::f32::consts::PI),
                            Vec3::new(0.0, 0.0, 1.0),
                        ),
                        0,
                    )?;
                    cmd_list.draw_offset(&cinder.device, 3, 0, 0);

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
            title: "hello-triangle",
            ..Default::default()
        },
    )
    .unwrap();
    let mut cinder = Cinder::<HelloTriangle>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
