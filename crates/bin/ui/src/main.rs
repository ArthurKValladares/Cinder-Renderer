use anyhow::Result;
use bumpalo::Bump;
use cinder::{
    command_queue::{AttachmentStoreOp, ClearValue, RenderAttachmentDesc},
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
    },
    Cinder, ResourceId,
};
use egui_integration::{egui, helpers::HelperEguiMenu, EguiIntegration};
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

pub struct Renderer {
    cinder: Cinder,
    ui: EguiIntegration,
    helper_egui_menu: HelperEguiMenu,
    model_data: ModelData,
    depth_image_handle: ResourceId<Image>,
    pipeline: GraphicsPipeline,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
    allocator: Bump,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let (width, height) = window.drawable_size();
        let mut cinder = Cinder::new(window, width, height)?;

        //
        // Create App Resources
        //
        let surface_rect = cinder.device.surface_rect();
        let depth_image = cinder.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/ui.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/ui.frag.spv"),
            Default::default(),
        )?;
        let pipeline = cinder.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(0).unwrap())?;
        let ubo_buffer = cinder.device.create_buffer(
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
        cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
        }])?;
        let ui = EguiIntegration::new(
            &mut cinder.resource_manager,
            &cinder.device,
            &cinder.swapchain,
        )?;
        let vertex_buffer = cinder.device.create_buffer_with_data(
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
        let index_buffer = cinder.device.create_buffer_with_data(
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
        vertex_shader.destroy(&cinder.device);
        fragment_shader.destroy(&cinder.device);

        let depth_image_handle = cinder.resource_manager.insert_image(depth_image);

        Ok(Self {
            cinder,
            depth_image_handle,
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
            ubo_buffer,
            ui,
            helper_egui_menu: HelperEguiMenu::default(),
            model_data: Default::default(),
            allocator: Bump::new(),
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale = self.model_data.scale;
        self.ubo_buffer.mem_copy(
            util::offset_of!(UiUniformBufferObject, model) as u64,
            &[Mat4::scale(Vec3::new(scale, scale, scale))
                * Mat4::rotate(self.model_data.rotation, Vec3::new(1.0, 1.0, 0.0))],
        )?;
        Ok(())
    }

    pub fn draw(&mut self, window: &Window) -> Result<bool> {
        let mut graph = RenderGraph::new(&self.allocator);
        graph.add_pass(
            &self.allocator,
            RenderPass::new(&self.allocator)
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
                .set_callback(&self.allocator, |cinder, cmd_list| {
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

        let pc = graph.run(&self.allocator, &mut self.cinder)?;

        self.ui.run(
            &mut self.cinder.resource_manager,
            &self.cinder.device,
            window,
            &pc.cmd_list,
            pc.present_rect,
            pc.swapchain_image,
            |ctx| {
                let pi_2 = std::f32::consts::PI * 2.0;
                egui::Window::new("UI").show(ctx, |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.model_data.rotation, -pi_2..=pi_2)
                            .text("Rotation"),
                    );
                    ui.add(egui::Slider::new(&mut self.model_data.scale, 1.0..=2.0).text("Scale"));
                    self.helper_egui_menu.draw(ui);
                });
            },
        )?;
        self.helper_egui_menu.update(&mut self.ui);

        pc.present(&mut self.cinder)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)?;
        let depth_image = self
            .cinder
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&self.cinder.device, Size2D::new(width, height))?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
        self.index_buffer.destroy(&self.cinder.device);
        self.vertex_buffer.destroy(&self.cinder.device);
        self.ubo_buffer.destroy(&self.cinder.device);
        self.pipeline.destroy(&self.cinder.device);
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

    let mut renderer = Renderer::new(&sdl.window).unwrap();

    'running: loop {
        renderer.allocator.reset();
        renderer.cinder.start_frame().unwrap();

        for event in sdl.event_pump.poll_iter() {
            let response = renderer.ui.on_event(&event);
            if !response.consumed {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => {
                        break 'running;
                    }
                    Event::Window {
                        win_event: sdl2::event::WindowEvent::SizeChanged(width, height),
                        ..
                    } => {
                        renderer.resize(width as u32, height as u32).unwrap();
                    }
                    _ => {}
                }
            }
        }

        renderer.update().unwrap();
        renderer.draw(&sdl.window).unwrap();

        renderer.cinder.end_frame();
    }
}
