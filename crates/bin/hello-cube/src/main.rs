use anyhow::Result;
use cinder::{
    context::render_context::{
        AttachmentStoreOp, ClearValue, Layout, RenderAttachment, RenderAttachmentDesc,
        RenderContext,
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, Usage},
        memory::MemoryType,
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        ResourceHandle,
    },
    view::View,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use std::time::Instant;
use winit::{
    dpi::PhysicalSize,
    event::VirtualKeyCode,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const WINDOW_WIDTH: u32 = 2000;
pub const WINDOW_HEIGHT: u32 = 2000;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/cube_shader_structs.rs"
));

#[rustfmt::skip]
fn look_to(eye: Vec3, front: Vec3, world_up: Vec3) -> Mat4 {
    let front = (front * -1.0).normalized();
    let side = world_up.cross(&front).normalized();
    let up = front.cross(&side);

    Mat4::from_data(
        side.x(),  side.y(),  side.z(),  -side.dot(&eye),
        up.x(),    up.y(),    up.z(),    -up.dot(&eye),
        front.x(), front.y(), front.z(), -front.dot(&eye),
        0.0,       0.0,       0.0,       1.0,
    )
}

#[rustfmt::skip]
fn new_infinite_perspective_proj(aspect_ratio: f32, y_fov: f32, z_near: f32) -> Mat4 {
    let f = 1.0 / (y_fov / 2.0).tan();
    Mat4::from_data(
        f / aspect_ratio, 0., 0.0, 0.0,
        0.0,              f,  0.0, 0.0,
        0.0,              0., 0.0, z_near,
        0.0,              0., 1.0, 0.0,
    )
}

pub struct Renderer {
    device: Device,
    view: View,
    depth_image: Image,
    render_pipeline: ResourceHandle<GraphicsPipeline>,
    render_context: RenderContext,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
    init_time: Instant,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let mut device = Device::new(window)?;
        let render_context = RenderContext::new(&device)?;
        let view = View::new(&device)?;
        let surface_rect = device.surface_rect();
        let depth_image = device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: Usage::Depth,
                memory_ty: MemoryType::GpuOnly,
            },
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            device.create_shader(include_bytes!("../shaders/spv/cube.vert.spv"))?,
            device.create_shader(include_bytes!("../shaders/spv/cube.frag.spv"))?,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let vertex_buffer = device.create_buffer_with_data(
            &[
                // Plane at z: -0.5
                CubeVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                CubeVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                CubeVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                CubeVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                // Plane at z: 0.5
                CubeVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                CubeVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                CubeVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                CubeVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                // Plane at x: -0.5
                CubeVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                CubeVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                CubeVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                CubeVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                // Plane at x: 0.5
                CubeVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                CubeVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                CubeVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                CubeVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 1.0, 0.0],
                },
                // Plane at y: -0.5
                CubeVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                CubeVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                CubeVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                CubeVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [0.0, 1.0, 1.0],
                },
                // Plane at y: 0.5
                CubeVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
                CubeVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
                CubeVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
                CubeVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 1.0, 1.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = device.create_buffer_with_data(
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

        let ubo_buffer = device.create_buffer(
            std::mem::size_of::<CubeUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        ubo_buffer.mem_copy(
            util::offset_of!(CubeUniformBufferObject, view) as u64,
            &[
                look_to(
                    Vec3::new(2.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ),
                new_infinite_perspective_proj(
                    surface_rect.width() as f32 / surface_rect.height() as f32,
                    30.0,
                    0.01,
                ),
            ],
        )?;

        device.write_bind_group(
            render_pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
            }],
        )?;

        let init_time = Instant::now();

        Ok(Self {
            device,
            view,
            depth_image,
            render_context,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            ubo_buffer,
            init_time,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale = (self.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        self.ubo_buffer.mem_copy(
            util::offset_of!(CubeUniformBufferObject, model) as u64,
            &[Mat4::rotate(scale, Vec3::new(1.0, 1.0, 0.0))],
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<bool> {
        let drawable = self.view.get_current_drawable(&self.device)?;

        self.render_context.begin(&self.device)?;
        {
            let surface_rect = self.device.surface_rect();

            self.render_context
                .transition_undefined_to_color(&self.device, drawable);

            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(drawable, Default::default())],
                Some(RenderAttachment::depth(
                    &self.depth_image,
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::DontCare,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )),
            );
            {
                self.render_context
                    .bind_graphics_pipeline(&self.device, self.render_pipeline)?;
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                self.render_context
                    .bind_index_buffer(&self.device, &self.index_buffer);
                self.render_context
                    .bind_vertex_buffer(&self.device, &self.vertex_buffer);
                // TODO: re-think API later when using more than one set
                self.render_context.bind_descriptor_sets(&self.device)?;

                self.render_context.draw_offset(&self.device, 36, 0, 0);
            }
            self.render_context.end_rendering(&self.device);

            self.render_context
                .transition_color_to_present(&self.device, drawable);
        }
        self.render_context.end(&self.device)?;

        self.view.present(&self.device, drawable)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.view.resize(&self.device)?;
        self.depth_image.resize(&self.device, Size2D::new(width, height))?;
        Ok(())
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let mut renderer = Renderer::new(&window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        renderer.update().expect("could not update renderer");

        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::Resized(size) => {
                    renderer.resize(size.width, size.height).unwrap();
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::RedrawRequested(_) => {
                renderer.draw().unwrap();
            }
            _ => {}
        }

        window.request_redraw();
    });
}
