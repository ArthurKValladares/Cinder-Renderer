use anyhow::Result;
use cinder::{
    context::{
        render_context::{
            AttachmentStoreOp, ClearValue, Layout, RenderAttachment, RenderAttachmentDesc,
            RenderContext,
        },
        upload_context::UploadContext,
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, Usage},
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
    "/gen/bindless_shader_structs.rs"
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

impl Vertex for BindlessVertex {
    fn from_obj_mesh_index(mesh: &ObjMesh, i: usize) -> Self {
        let pos = [
            mesh.positions[i * 3],
            -mesh.positions[i * 3 + 1],
            mesh.positions[i * 3 + 2],
            1.0,
        ];

        let color = if !mesh.vertex_color.is_empty() {
            [
                mesh.vertex_color[i * 3],
                mesh.vertex_color[i * 3 + 1],
                mesh.vertex_color[i * 3 + 2],
            ]
        } else {
            [1.0; 3]
        };

        let normal = if !mesh.normals.is_empty() {
            [
                mesh.normals[i * 3],
                mesh.normals[i * 3 + 1],
                mesh.normals[i * 3 + 2],
            ]
        } else {
            [1.0; 3]
        };

        let uv = if !mesh.texcoords.is_empty() {
            [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]]
        } else {
            [0.0; 2]
        };

        Self {
            pos,
            color,
            normal,
            uv,
        }
    }

    fn pos_3d(&self) -> [f32; 3] {
        // TODO: Maybe rethink this function?
        [self.pos[0], self.pos[1], self.pos[2]]
    }
}

#[derive(Debug)]
pub struct MeshDraw {
    vertex_buffer_offset: i32,
    index_buffer_offset: u32,
    num_indices: u32,
    image_index: u32,
}

pub struct Renderer {
    device: Device,
    view: View,
    depth_image: Image,
    render_pipeline: ResourceHandle<GraphicsPipeline>,
    render_context: RenderContext,
    _upload_context: UploadContext,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
    sampler: Sampler,
    images: Vec<Image>,
    image_buffers: Vec<Buffer>,
    init_time: Instant,
    mesh_draws: Vec<MeshDraw>,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let mut device = Device::new(window, Default::default())?;
        let render_context = RenderContext::new(&device, Default::default())?;
        let upload_context = UploadContext::new(&device, Default::default())?;
        let view = View::new(&device, Default::default())?;
        let surface_rect = device.surface_rect();
        let depth_image = device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: Usage::Depth,
                ..Default::default()
            },
        )?;

        let mut vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/bindless.vert.spv"),
            Default::default(),
        )?;
        let mut fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/bindless.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            &vertex_shader,
            &fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;
        vertex_shader.destroy(&device);
        fragment_shader.destroy(&device);

        let scene = Scene::<BindlessVertex>::from_obj(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("sponza"),
            "sponza.obj",
        )?;
        let (vertices, indices, mesh_draws) = {
            let mut vertices: Vec<BindlessVertex> = Default::default();
            let mut indices: Vec<u32> = Default::default();
            let mut mesh_draws: Vec<MeshDraw> = Default::default();
            for mesh in scene.meshes {
                let first_vertex = vertices.len();

                let first_index = indices.len();
                let num_indices = mesh.indices.len() as u32;

                vertices.extend(mesh.vertices);
                indices.extend(mesh.indices);

                mesh_draws.push(MeshDraw {
                    vertex_buffer_offset: first_vertex as i32,
                    index_buffer_offset: first_index as u32,
                    num_indices,
                    image_index: mesh.material_index.unwrap_or(0) as u32, // TODO: handle the None case better
                });
            }
            (vertices, indices, mesh_draws)
        };

        let vertex_buffer = device.create_buffer_with_data(
            &vertices,
            BufferDescription {
                usage: BufferUsage::STORAGE | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
        )?;
        let index_buffer = device.create_buffer_with_data(
            &indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let ubo_buffer = device.create_buffer(
            std::mem::size_of::<BindlessUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        ubo_buffer.mem_copy(
            util::offset_of!(BindlessUniformBufferObject, model) as u64,
            &[
                Mat4::identity(),
                look_to(
                    Vec3::new(0.0, -50.0, 0.0),
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

        let sampler = device.create_sampler(&device, Default::default())?;

        device.write_bind_group(
            render_pipeline,
            &[
                BindGroupBindInfo {
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
                },
                BindGroupBindInfo {
                    dst_binding: 1,
                    data: BindGroupWriteData::Storage(vertex_buffer.bind_info()),
                },
            ],
        )?;

        upload_context.begin(&device, device.setup_fence())?;
        let (images, image_buffers) = scene
            .materials
            .iter()
            .enumerate()
            .filter(|(_, material)| material.diffuse.is_some())
            .map(|(idx, material)| {
                let diffuse = material.diffuse.as_ref().unwrap();
                let image = diffuse.to_rgba8();
                let (width, height) = image.dimensions();

                let texture = device
                    .create_image(Size2D::new(width, height), Default::default())
                    .unwrap();

                let image_data = image.into_raw();
                let image_buffer = device
                    .create_buffer_with_data(
                        &image_data,
                        BufferDescription {
                            usage: BufferUsage::TRANSFER_SRC,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                upload_context.image_barrier_start(&device, &texture);
                upload_context.copy_buffer_to_image(&device, &image_buffer, &texture);
                upload_context.image_barrier_end(&device, &texture);

                device
                    .write_bind_group(
                        render_pipeline,
                        &[BindGroupBindInfo {
                            dst_binding: 2,
                            data: BindGroupWriteData::SampledImage(texture.bind_info(
                                &sampler,
                                Layout::ShaderReadOnly,
                                idx as u32,
                            )),
                        }],
                    )
                    .unwrap();

                (texture, image_buffer)
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();
        upload_context.end(
            &device,
            device.setup_fence(),
            device.present_queue(),
            &[],
            &[],
            &[],
        )?;

        let init_time = Instant::now();

        Ok(Self {
            device,
            view,
            depth_image,
            render_context,
            _upload_context: upload_context,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            sampler,
            images,
            image_buffers,
            ubo_buffer,
            init_time,
            mesh_draws,
        })
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
                // TODO: re-think API later when using more than one se
                self.render_context.bind_descriptor_sets(&self.device)?;

                for mesh_draw in &self.mesh_draws {
                    self.render_context.set_fragment_bytes(
                        &self.device,
                        &[mesh_draw.image_index],
                        0,
                    )?;

                    self.render_context.draw_offset(
                        &self.device,
                        mesh_draw.num_indices,
                        mesh_draw.index_buffer_offset,
                        mesh_draw.vertex_buffer_offset,
                    );
                }
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
        self.depth_image
            .resize(&self.device, Size2D::new(width, height))?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();

        self.depth_image.destroy(self.device.raw());
        self.sampler.destroy(self.device.raw());
        for mut image in self.images.drain(..) {
            image.destroy(self.device.raw());
        }
        for mut buffer in self.image_buffers.drain(..) {
            buffer.destroy(self.device.raw());
        }

        self.vertex_buffer.destroy(self.device.raw());
        self.index_buffer.destroy(self.device.raw());
        self.ubo_buffer.destroy(self.device.raw());

        self.view.destroy(&self.device);
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
