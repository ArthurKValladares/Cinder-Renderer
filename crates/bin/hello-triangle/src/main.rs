use anyhow::Result;
use cinder::{
    command_queue::{CommandQueue, RenderAttachment},
    device::Device,
    resources::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        pipeline::graphics::GraphicsPipeline,
        ResourceManager,
    },
    swapchain::Swapchain,
    ResourceId,
};
use math::{mat::Mat4, vec::Vec3};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use std::time::Instant;
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/triangle_shader_structs.rs"
));

pub struct Renderer {
    resource_manager: ResourceManager,
    device: Device,
    swapchain: Swapchain,
    command_queue: CommandQueue,
    render_pipeline: ResourceId<GraphicsPipeline>,
    vertex_buffer_handle: ResourceId<Buffer>,
    index_buffer_handle: ResourceId<Buffer>,
    init_time: Instant,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height, Default::default())?;
        let command_queue = CommandQueue::new(&device)?;
        let swapchain = Swapchain::new(&device, Default::default())?;

        let mut vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/triangle.vert.spv"),
            Default::default(),
        )?;
        let mut fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/triangle.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline =
            resource_manager.insert_graphics_pipeline(device.create_graphics_pipeline(
                &vertex_shader,
                &fragment_shader,
                Default::default(),
            )?);
        vertex_shader.destroy(device.raw());
        fragment_shader.destroy(device.raw());

        let vertex_buffer = resource_manager.insert_buffer(device.create_buffer_with_data(
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
        )?);
        let index_buffer = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[0, 1, 2],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?);

        let init_time = Instant::now();

        Ok(Self {
            resource_manager,
            device,
            swapchain,
            command_queue,
            render_pipeline,
            vertex_buffer_handle: vertex_buffer,
            index_buffer_handle: index_buffer,
            init_time,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.device.surface_rect();

        let (mut submit_desc, cmd_list) = self.command_queue.begin(&self.device)?;

        let swapchain_image = self.swapchain.acquire_image(&self.device, &cmd_list)?;

        self.command_queue.begin_rendering(
            &self.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
            None,
        );
        let index_buffer = self
            .resource_manager
            .get_buffer(self.index_buffer_handle)
            .unwrap();
        let vertex_buffer = self
            .resource_manager
            .get_buffer(self.vertex_buffer_handle)
            .unwrap();

        let pipeline = self
            .resource_manager
            .get_graphics_pipeline(self.render_pipeline)
            .unwrap();
        self.command_queue
            .bind_graphics_pipeline(&self.device, pipeline);
        self.command_queue
            .bind_viewport(&self.device, surface_rect, true);
        self.command_queue.bind_scissor(&self.device, surface_rect);
        self.command_queue
            .bind_index_buffer(&self.device, index_buffer);
        self.command_queue
            .bind_vertex_buffer(&self.device, vertex_buffer);

        self.command_queue.set_vertex_bytes(
            &self.device,
            pipeline,
            &Mat4::rotate(
                (self.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            0,
        )?;

        self.command_queue.draw_offset(&self.device, 3, 0, 0);

        self.command_queue.end_rendering(&self.device);

        self.swapchain
            .transition_image(&self.device, &cmd_list, swapchain_image);

        self.command_queue.end(&self.device, &submit_desc)?;

        self.swapchain.present(&self.device, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.swapchain.resize(&self.device)?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();
        self.command_queue.destroy(&self.device);
        self.swapchain.destroy(&self.device);
        self.resource_manager.force_destroy(&self.device);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription { title: "ui" },
    )
    .unwrap();

    let mut renderer = Renderer::new(&sdl.window).unwrap();

    'running: loop {
        for event in sdl.event_pump.poll_iter() {
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
        renderer.draw().unwrap();

        renderer.resource_manager.consume(&renderer.device);
        renderer.device.bump_frame();
    }
}
