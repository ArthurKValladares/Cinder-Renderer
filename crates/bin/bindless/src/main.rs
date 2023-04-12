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
        image::{Format, Image, ImageDescription, ImageUsage},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        ResourceManager,
    },
    view::View,
    ResourceId,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use rayon::iter::*;
use scene::{ObjMesh, Scene, Vertex};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use std::path::PathBuf;
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/bindless_shader_structs.rs"
));

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
        [self.pos[0], self.pos[1], self.pos[2]]
    }

    fn set_pos_3d(mut self, x: f32, y: f32, z: f32) -> Self {
        self.pos[0] = x;
        self.pos[1] = y;
        self.pos[2] = z;
        self
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
    resource_manager: ResourceManager,
    device: Device,
    view: View,
    render_context: RenderContext,
    mesh_draws: Vec<MeshDraw>,
    depth_image_handle: ResourceId<Image>,
    render_pipeline: ResourceId<GraphicsPipeline>,
    index_buffer_handle: ResourceId<Buffer>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height, Default::default())?;
        let render_context = RenderContext::new(&device, Default::default())?;
        let upload_context = UploadContext::new(&device, Default::default())?;
        let view = View::new(&device, Default::default())?;
        let surface_rect = device.surface_rect();
        let depth_image = resource_manager.insert_image(device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?);

        let mut vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/bindless.vert.spv"),
            Default::default(),
        )?;
        let mut fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/bindless.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline =
            resource_manager.insert_graphics_pipeline(device.create_graphics_pipeline(
                &vertex_shader,
                &fragment_shader,
                GraphicsPipelineDescription {
                    depth_format: Some(Format::D32_SFloat),
                    ..Default::default()
                },
            )?);
        vertex_shader.destroy(device.raw());
        fragment_shader.destroy(device.raw());

        let init_time = std::time::Instant::now();
        let scene = zero_copy_assets::try_decoded_file::<Scene<BindlessVertex>>(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("sponza")
                .join("sponza.obj"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("gen")
                .join("sponza.adm"),
        )?;
        println!("Scene creation: {:?}", init_time.elapsed().as_millis());
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

        let vertex_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &vertices,
            BufferDescription {
                usage: BufferUsage::STORAGE | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
        )?);
        let index_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?);

        let ubo_buffer_handle = resource_manager.insert_buffer(device.create_buffer(
            std::mem::size_of::<BindlessUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?);
        {
            let ubo_buffer = resource_manager.get_buffer(ubo_buffer_handle).unwrap();
            ubo_buffer.mem_copy(
                util::offset_of!(BindlessUniformBufferObject, model) as u64,
                &[
                    Mat4::identity(),
                    camera::look_to(
                        Vec3::new(0.0, -50.0, 0.0),
                        Vec3::new(1.0, 0.0, 0.0),
                        Vec3::new(0.0, 1.0, 0.0),
                    ),
                    camera::new_infinite_perspective_proj(
                        surface_rect.width() as f32 / surface_rect.height() as f32,
                        30.0,
                        0.01,
                    ),
                ],
            )?;
            let vertex_buffer = resource_manager.get_buffer(vertex_buffer_handle).unwrap();
            let pipeline = resource_manager
                .get_graphics_pipeline(render_pipeline)
                .unwrap();
            device.write_bind_group(
                pipeline,
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
        }
        let sampler =
            resource_manager.insert_sampler(device.create_sampler(&device, Default::default())?);

        let image_data = scene
            .materials
            .par_iter()
            .enumerate()
            .filter(|(_, material)| material.diffuse.is_some())
            .map(|(idx, material)| (idx, material.diffuse.as_ref().unwrap()))
            .collect::<Vec<_>>();

        upload_context.begin(&device, device.setup_fence())?;
        let (_image_handles, image_buffer_handles) = image_data
            .into_iter()
            .map(|(idx, image_data)| {
                let texture = device
                    .create_image(
                        Size2D::new(image_data.width, image_data.height),
                        Default::default(),
                    )
                    .unwrap();
                let image_buffer = device
                    .create_buffer_with_data(
                        &image_data.bytes,
                        BufferDescription {
                            usage: BufferUsage::TRANSFER_SRC,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                upload_context.image_barrier_start(&device, &texture);
                upload_context.copy_buffer_to_image(&device, &image_buffer, &texture);
                upload_context.image_barrier_end(&device, &texture);

                let pipeline = resource_manager
                    .get_graphics_pipeline(render_pipeline)
                    .unwrap();
                device
                    .write_bind_group(
                        pipeline,
                        &[BindGroupBindInfo {
                            dst_binding: 2,
                            data: BindGroupWriteData::SampledImage(texture.bind_info(
                                resource_manager.get_sampler(sampler).unwrap(),
                                Layout::ShaderReadOnly,
                                Some(idx as u32),
                            )),
                        }],
                    )
                    .unwrap();

                let texture_handle = resource_manager.insert_image(texture);
                let image_buffer_handle = resource_manager.insert_buffer(image_buffer);
                (texture_handle, image_buffer_handle)
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

        for handle in image_buffer_handles {
            resource_manager.delete_buffer(handle, device.frame_index());
        }

        Ok(Self {
            resource_manager,
            device,
            view,
            depth_image_handle: depth_image,
            render_context,
            render_pipeline,
            index_buffer_handle,
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

            let depth_image = self
                .resource_manager
                .get_image(self.depth_image_handle)
                .unwrap();
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
                let pipeline = self
                    .resource_manager
                    .get_graphics_pipeline(self.render_pipeline)
                    .unwrap();
                self.render_context
                    .bind_graphics_pipeline(&self.device, pipeline);
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                let index_buffer = self
                    .resource_manager
                    .get_buffer(self.index_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_index_buffer(&self.device, index_buffer);
                // TODO: re-think API later when using more than one se
                self.render_context
                    .bind_descriptor_sets(&self.device, pipeline);

                for mesh_draw in &self.mesh_draws {
                    self.render_context.set_fragment_bytes(
                        &self.device,
                        pipeline,
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
        let depth_image = self
            .resource_manager
            .get_image_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&self.device, Size2D::new(width, height))?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();

        self.view.destroy(&self.device);
        self.resource_manager.force_destroy(&self.device);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription { title: "bindless" },
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
