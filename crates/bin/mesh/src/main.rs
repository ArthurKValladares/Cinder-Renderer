use anyhow::Result;
use cinder::{
    context::{
        render_context::{
            AttachmentStoreOp, ClearValue, Layout, RenderAttachment, RenderAttachmentDesc,
            RenderContext,
        },
        upload_context::UploadContext,
    },
    device::{Device, ResourceManager},
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
    },
    view::View,
    ResourceHandle,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use scene::{ObjMesh, Scene, Vertex};
use std::{path::PathBuf, time::Instant};
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
    "/gen/mesh_shader_structs.rs"
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

impl Vertex for MeshVertex {
    fn from_obj_mesh_index(mesh: &ObjMesh, i: usize) -> Self {
        let i_pos = [
            mesh.positions[i * 3],
            mesh.positions[i * 3 + 1],
            mesh.positions[i * 3 + 2],
        ];

        let i_uv = if !mesh.texcoords.is_empty() {
            [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]]
        } else {
            [0.0, 0.0]
        };

        Self { i_pos, i_uv }
    }

    fn pos_3d(&self) -> [f32; 3] {
        self.i_pos
    }
}

pub struct Renderer {
    resource_manager: ResourceManager,
    device: Device,
    view: View,
    depth_image: ResourceHandle<Image>,
    render_pipeline: ResourceHandle<GraphicsPipeline>,
    render_context: RenderContext,
    _upload_context: UploadContext,
    vertex_buffer_handle: ResourceHandle<Buffer>,
    index_buffer_handle: ResourceHandle<Buffer>,
    ubo_buffer_handle: ResourceHandle<Buffer>,
    image_buffer_handle: ResourceHandle<Buffer>,
    sampler: ResourceHandle<Sampler>,
    texture_handle: ResourceHandle<Image>,
    init_time: Instant,
    index_count: u32,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let device = Device::new(window, Default::default())?;
        let render_context = RenderContext::new(&device, Default::default())?;
        let upload_context = UploadContext::new(&device, Default::default())?;
        let view = View::new(&device, Default::default())?;
        let surface_rect = device.surface_rect();
        let depth_image = device.create_image(
            &mut resource_manager,
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;

        let vertex_shader = device.create_shader(
            &mut resource_manager,
            include_bytes!("../shaders/spv/mesh.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = device.create_shader(
            &mut resource_manager,
            include_bytes!("../shaders/spv/mesh.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            &mut resource_manager,
            vertex_shader,
            fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let scene = Scene::<MeshVertex>::from_obj(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("models"),
            "viking_room.obj",
        )?;
        let mesh = scene.meshes.first().unwrap();

        let vertex_buffer_handle = device.create_buffer_with_data(
            &mut resource_manager,
            &mesh.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer_handle = device.create_buffer_with_data(
            &mut resource_manager,
            &mesh.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let ubo_buffer_handle = device.create_buffer(
            &mut resource_manager,
            std::mem::size_of::<MeshUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        {
            let ubo_buffer = resource_manager.get_buffer_mut(ubo_buffer_handle).unwrap();
            ubo_buffer.mem_copy(
                util::offset_of!(MeshUniformBufferObject, view) as u64,
                &[
                    look_to(
                        Vec3::new(2.0, -0.5, 0.0),
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
        }
        let sampler = device.create_sampler(&mut resource_manager, &device, Default::default())?;

        let image = image::load_from_memory(include_bytes!("../assets/textures/viking_room.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let texture_handle = device.create_image(
            &mut resource_manager,
            Size2D::new(width, height),
            Default::default(),
        )?;
        let image_data = image.into_raw();

        let image_buffer_handle = device.create_buffer_with_data(
            &mut resource_manager,
            &image_data,
            BufferDescription {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
        )?;
        let image_buffer = resource_manager.get_buffer(image_buffer_handle).unwrap();
        let texture = resource_manager.get_image(texture_handle).unwrap();
        upload_context.begin(&device, device.setup_fence())?;
        {
            upload_context.image_barrier_start(&device, &texture);
            upload_context.copy_buffer_to_image(&device, image_buffer, &texture);
            upload_context.image_barrier_end(&device, &texture);
        }
        upload_context.end(
            &device,
            device.setup_fence(),
            device.present_queue(),
            &[],
            &[],
            &[],
        )?;

        let ubo_buffer = resource_manager.get_buffer(ubo_buffer_handle).unwrap();
        let s = resource_manager.get_sampler(sampler).unwrap();
        device.write_bind_group(
            &resource_manager,
            render_pipeline,
            &[
                BindGroupBindInfo {
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
                },
                BindGroupBindInfo {
                    dst_binding: 1,
                    data: BindGroupWriteData::SampledImage(texture.bind_info(
                        s,
                        Layout::ShaderReadOnly,
                        0,
                    )),
                },
            ],
        )?;

        let init_time = Instant::now();

        Ok(Self {
            resource_manager,
            device,
            view,
            depth_image,
            render_context,
            _upload_context: upload_context,
            render_pipeline,
            vertex_buffer_handle,
            index_buffer_handle,
            image_buffer_handle,
            sampler,
            texture_handle,
            ubo_buffer_handle,
            init_time,
            index_count: mesh.indices.len() as u32,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale = (self.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        let ubo_buffer = self
            .resource_manager
            .get_buffer_mut(self.ubo_buffer_handle)
            .unwrap();
        ubo_buffer.mem_copy(
            util::offset_of!(MeshUniformBufferObject, model) as u64,
            &[
                Mat4::rotate(std::f32::consts::PI / 2.0, Vec3::new(1.0, 0.0, 0.0))
                    * Mat4::rotate(scale, Vec3::new(0.0, 0.0, 1.0)),
            ],
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

            // TODO: remove get from user code?
            let depth_image = self.resource_manager.get_image(self.depth_image).unwrap();
            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(drawable, Default::default())],
                Some(RenderAttachment::depth(
                    depth_image,
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::DontCare,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )),
            );
            {
                self.render_context.bind_graphics_pipeline(
                    &self.resource_manager,
                    &self.device,
                    self.render_pipeline,
                )?;
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                let index_buffer = self
                    .resource_manager
                    .get_buffer(self.index_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_index_buffer(&self.device, index_buffer);
                let vertex_buffer = self
                    .resource_manager
                    .get_buffer(self.vertex_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_vertex_buffer(&self.device, vertex_buffer);
                // TODO: re-think API later when using more than one set
                self.render_context
                    .bind_descriptor_sets(&self.resource_manager, &self.device)?;

                self.render_context
                    .draw_offset(&self.device, self.index_count, 0, 0);
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
        let depth_image = self
            .resource_manager
            .get_image_mut(self.depth_image)
            .unwrap();
        depth_image.resize(&self.device, Size2D::new(width, height))?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();

        self.view.destroy(&self.device);
        self.resource_manager.clean(&self.device);
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
