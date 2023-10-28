use anyhow::Result;
use bumpalo::Bump;
use cinder::{
    App, AttachmentStoreOp, BindGroup, BindGroupBindInfo, BindGroupWriteData, Buffer,
    BufferDescription, BufferUsage, Cinder, ClearValue, DebugUiContext, Format, GraphicsPipeline,
    GraphicsPipelineDescription, Image, ImageDescription, ImageUsage, Layout, RenderAttachmentDesc,
    Renderer, ResourceId,
};
use egui_integration::{egui, EguiIntegration};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use render_graph::{AttachmentType, RenderGraph, RenderPass};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/ui_shader_structs.rs"
));

struct ModelData {
    scale: f32,
    rotation: f32,
}

impl Default for ModelData {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: 0.0,
        }
    }
}

pub struct UiSample {
    model_data: ModelData,
    depth_image_handle: ResourceId<Image>,
    pipeline: GraphicsPipeline,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl App for UiSample {
    fn new(renderer: &mut Renderer, _width: u32, _height: u32) -> Result<Self> {
        //
        // Create App Resources
        //
        let surface_rect = renderer.device.surface_rect();
        let depth_image = renderer.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let vertex_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/ui.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/ui.frag.spv"),
            Default::default(),
        )?;
        let pipeline = renderer.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(&renderer.device, pipeline.bind_group_data(0).unwrap())?;
        let ubo_buffer = renderer.device.create_buffer(
            std::mem::size_of::<UiUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        ubo_buffer.mem_copy(
            util::offset_of!(UiUniformBufferObject, view) as u64,
            &[
                camera::look_to(
                    Vec3::new(2.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ),
                camera::new_infinite_perspective_proj(
                    surface_rect.width() as f32 / surface_rect.height() as f32,
                    30.0,
                    0.01,
                ),
            ],
        )?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
        }])?;
        let vertex_buffer = renderer.device.create_buffer_with_data(
            &[
                // Plane at z: -0.5
                UiVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                UiVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                UiVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                UiVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                // Plane at z: 0.5
                UiVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                UiVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                UiVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                UiVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                // Plane at x: -0.5
                UiVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                UiVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                UiVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                UiVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                // Plane at x: 0.5
                UiVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                UiVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                UiVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                UiVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                // Plane at y: -0.5
                UiVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                UiVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                UiVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                UiVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                // Plane at y: 0.5
                UiVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
                UiVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
                UiVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
                UiVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = renderer.device.create_buffer_with_data(
            &[
                0, 1, 2, 2, 1, 3, // First plane
                4, 5, 6, 6, 5, 7, // Second plane
                8, 9, 10, 10, 9, 11, // Third plane
                12, 13, 14, 14, 13, 15, // Fourth plane
                16, 17, 18, 18, 17, 19, // Fifth plane
                20, 21, 22, 22, 21, 23, // Sixth plane
            ],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        //
        // Cleanup
        //
        vertex_shader.destroy(&renderer.device);
        fragment_shader.destroy(&renderer.device);

        let depth_image_handle = renderer.resource_manager.insert_image(depth_image);

        Ok(Self {
            depth_image_handle,
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
            ubo_buffer,
            model_data: Default::default(),
        })
    }

    fn update(&mut self, renderer: &mut Renderer) -> Result<()> {
        let scale = self.model_data.scale;
        self.ubo_buffer.mem_copy(
            util::offset_of!(UiUniformBufferObject, model) as u64,
            &[Mat4::scale(Vec3::new(scale, scale, scale))
                * Mat4::rotate(self.model_data.rotation, Vec3::new(1.0, 1.0, 0.0))],
        )?;
        Ok(())
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
                .set_depth_attachment(
                    AttachmentType::Reference(self.depth_image_handle),
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::DontCare,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )
                .set_callback(allocator, |cinder, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&cinder.device, &self.pipeline);
                    cmd_list.bind_index_buffer(&cinder.device, &self.index_buffer);
                    cmd_list.bind_vertex_buffer(&cinder.device, &self.vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &cinder.device,
                        &self.pipeline,
                        0,
                        &[self.bind_group],
                    );
                    cmd_list.draw_offset(&cinder.device, 36, 0, 0);

                    Ok(())
                }),
        );
        Ok(())
    }

    fn draw_debug_ui(&mut self, context: &DebugUiContext) {
        let pi_2 = std::f32::consts::PI * 2.0;
        egui::Window::new("UI").show(context, |ui| {
            ui.add(egui::Slider::new(&mut self.model_data.rotation, -pi_2..=pi_2).text("Rotation"));
            ui.add(egui::Slider::new(&mut self.model_data.scale, 1.0..=2.0).text("Scale"));
        });
    }

    fn resize(&mut self, renderer: &mut Renderer, width: u32, height: u32) -> Result<()> {
        let depth_image = renderer
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&renderer.device, Size2D::new(width, height))?;
        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.index_buffer.destroy(&renderer.device);
        self.vertex_buffer.destroy(&renderer.device);
        self.ubo_buffer.destroy(&renderer.device);
        self.pipeline.destroy(&renderer.device);
        Ok(())
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "ui",
            ..Default::default()
        },
    )
    .unwrap();
    let mut cinder = Cinder::<UiSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
