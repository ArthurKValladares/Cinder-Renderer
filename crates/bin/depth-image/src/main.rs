use anyhow::Result;
use cinder::{
    context::{
        render_context::{
            AttachmentLoadOp, AttachmentStoreOp, ClearValue, Layout, RenderAttachment,
            RenderAttachmentDesc, RenderContext,
        },
        upload_context::UploadContext,
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        ResourceManager,
    },
    view::View,
    ResourceId,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use std::time::Instant;
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/depth_mesh_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/depth_texture_shader_structs.rs"
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
    resource_manager: ResourceManager,
    device: Device,
    view: View,
    depth_image_handle: ResourceId<Image>,
    mesh_render_pipeline: ResourceId<GraphicsPipeline>,
    texture_render_pipeline: ResourceId<GraphicsPipeline>,
    render_context: RenderContext,
    upload_context: UploadContext,
    cube_vertex_buffer_handle: ResourceId<Buffer>,
    cube_index_buffer_handle: ResourceId<Buffer>,
    ubo_buffer_handle: ResourceId<Buffer>,
    quad_vertex_buffer_handle: ResourceId<Buffer>,
    quad_index_buffer_handle: ResourceId<Buffer>,
    sampler: ResourceId<Sampler>,
    init_time: Instant,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height, Default::default())?;
        let render_context = RenderContext::new(&device, Default::default())?;
        let view = View::new(&device, Default::default())?;
        let surface_rect = device.surface_rect();
        let depth_image_handle = resource_manager.insert_image(device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::DepthSampled,
                ..Default::default()
            },
        )?);
        let mut mesh_vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.vert.spv"),
            Default::default(),
        )?;
        let mut mesh_fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.frag.spv"),
            Default::default(),
        )?;
        let mesh_render_pipeline =
            resource_manager.insert_graphics_pipeline(device.create_graphics_pipeline(
                &mesh_vertex_shader,
                &mesh_fragment_shader,
                GraphicsPipelineDescription {
                    depth_format: Some(Format::D32_SFloat),
                    ..Default::default()
                },
            )?);
        mesh_vertex_shader.destroy(device.raw());
        mesh_fragment_shader.destroy(device.raw());

        let mut texture_vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.vert.spv"),
            Default::default(),
        )?;
        let mut texture_fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.frag.spv"),
            Default::default(),
        )?;
        let texture_render_pipeline =
            resource_manager.insert_graphics_pipeline(device.create_graphics_pipeline(
                &texture_vertex_shader,
                &texture_fragment_shader,
                Default::default(),
            )?);
        texture_vertex_shader.destroy(device.raw());
        texture_fragment_shader.destroy(device.raw());

        let cube_vertex_buffer_handle =
            resource_manager.insert_buffer(device.create_buffer_with_data(
                &[
                    // Plane at z: -0.5
                    DepthMeshVertex {
                        i_pos: [-0.5, 0.5, -0.5],
                        i_normal: [1.0, 0.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, 0.5, -0.5],
                        i_normal: [1.0, 0.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, -0.5, -0.5],
                        i_normal: [1.0, 0.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, -0.5, -0.5],
                        i_normal: [1.0, 0.0, 0.0],
                    },
                    // Plane at z: 0.5
                    DepthMeshVertex {
                        i_pos: [-0.5, 0.5, 0.5],
                        i_normal: [0.0, 0.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, 0.5, 0.5],
                        i_normal: [0.0, 0.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, -0.5, 0.5],
                        i_normal: [0.0, 0.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, -0.5, 0.5],
                        i_normal: [0.0, 0.0, 1.0],
                    },
                    // Plane at x: -0.5
                    DepthMeshVertex {
                        i_pos: [-0.5, -0.5, 0.5],
                        i_normal: [0.0, 1.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, 0.5, 0.5],
                        i_normal: [0.0, 1.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, -0.5, -0.5],
                        i_normal: [0.0, 1.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, 0.5, -0.5],
                        i_normal: [0.0, 1.0, 0.0],
                    },
                    // Plane at x: 0.5
                    DepthMeshVertex {
                        i_pos: [0.5, -0.5, 0.5],
                        i_normal: [1.0, 1.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, 0.5, 0.5],
                        i_normal: [1.0, 1.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, -0.5, -0.5],
                        i_normal: [1.0, 1.0, 0.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, 0.5, -0.5],
                        i_normal: [1.0, 1.0, 0.0],
                    },
                    // Plane at y: -0.5
                    DepthMeshVertex {
                        i_pos: [-0.5, -0.5, 0.5],
                        i_normal: [0.0, 1.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, -0.5, 0.5],
                        i_normal: [0.0, 1.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, -0.5, -0.5],
                        i_normal: [0.0, 1.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, -0.5, -0.5],
                        i_normal: [0.0, 1.0, 1.0],
                    },
                    // Plane at y: 0.5
                    DepthMeshVertex {
                        i_pos: [-0.5, 0.5, 0.5],
                        i_normal: [1.0, 1.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, 0.5, 0.5],
                        i_normal: [1.0, 1.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [-0.5, 0.5, -0.5],
                        i_normal: [1.0, 1.0, 1.0],
                    },
                    DepthMeshVertex {
                        i_pos: [0.5, 0.5, -0.5],
                        i_normal: [1.0, 1.0, 1.0],
                    },
                ],
                BufferDescription {
                    usage: BufferUsage::VERTEX,
                    ..Default::default()
                },
            )?);
        let cube_index_buffer_handle =
            resource_manager.insert_buffer(device.create_buffer_with_data(
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
            )?);

        let ubo_buffer_handle = resource_manager.insert_buffer(device.create_buffer(
            std::mem::size_of::<DepthMeshUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?);
        {
            let ubo_buffer = resource_manager.get_buffer(ubo_buffer_handle).unwrap();
            ubo_buffer.mem_copy(
                util::offset_of!(DepthMeshUniformBufferObject, view) as u64,
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
            let pipeline = resource_manager
                .get_graphics_pipeline(mesh_render_pipeline)
                .unwrap();
            device.write_bind_group(
                pipeline,
                &[BindGroupBindInfo {
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
                }],
            )?;
        }
        let quad_vertex_buffer_handle =
            resource_manager.insert_buffer(device.create_buffer_with_data(
                &[
                    DepthTextureVertex {
                        i_pos: [-1.0, -1.0],
                        i_uv: [0.0, 1.0],
                    },
                    DepthTextureVertex {
                        i_pos: [-0.25, -1.0],
                        i_uv: [1.0, 1.0],
                    },
                    DepthTextureVertex {
                        i_pos: [-0.25, -0.25],
                        i_uv: [1.0, 0.0],
                    },
                    DepthTextureVertex {
                        i_pos: [-1.0, -0.25],
                        i_uv: [0.0, 0.0],
                    },
                ],
                BufferDescription {
                    usage: BufferUsage::VERTEX,
                    ..Default::default()
                },
            )?);
        let quad_index_buffer_handle =
            resource_manager.insert_buffer(device.create_buffer_with_data(
                &[0, 1, 2, 2, 3, 0],
                BufferDescription {
                    usage: BufferUsage::INDEX,
                    ..Default::default()
                },
            )?);

        let sampler =
            resource_manager.insert_sampler(device.create_sampler(&device, Default::default())?);

        let init_time = Instant::now();

        let upload_context = UploadContext::new(&device, Default::default())?;
        let depth_image = resource_manager.get_image(depth_image_handle).unwrap();
        upload_context.begin(&device, device.setup_fence())?;
        {
            upload_context.transition_depth_to_read_only(&device, depth_image);
        }
        upload_context.end(
            &device,
            device.setup_fence(),
            device.present_queue(),
            &[],
            &[],
            &[],
        )?;

        let pipeline = resource_manager
            .get_graphics_pipeline(texture_render_pipeline)
            .unwrap();
        device.write_bind_group(
            pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(depth_image.bind_info(
                    resource_manager.get_sampler(sampler).unwrap(),
                    Layout::DepthStencilReadOnly,
                    0,
                )),
            }],
        )?;

        Ok(Self {
            resource_manager,
            device,
            view,
            depth_image_handle,
            render_context,
            upload_context,
            mesh_render_pipeline,
            texture_render_pipeline,
            cube_vertex_buffer_handle,
            cube_index_buffer_handle,
            ubo_buffer_handle,
            quad_vertex_buffer_handle,
            quad_index_buffer_handle,
            sampler,
            init_time,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale = (self.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        let ubo_buffer = self
            .resource_manager
            .get_buffer_mut(self.ubo_buffer_handle)
            .unwrap();
        ubo_buffer.mem_copy(
            util::offset_of!(DepthMeshUniformBufferObject, model) as u64,
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
                .bind_viewport(&self.device, surface_rect, true);
            self.render_context.bind_scissor(&self.device, surface_rect);

            self.render_context
                .transition_undefined_to_color(&self.device, drawable);

            // Mesh render pass
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
                        store_op: AttachmentStoreOp::Store,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )),
            );
            {
                let pipeline = self
                    .resource_manager
                    .get_graphics_pipeline(self.mesh_render_pipeline)
                    .unwrap();
                self.render_context
                    .bind_graphics_pipeline(&self.device, pipeline);
                let cube_index_buffer = self
                    .resource_manager
                    .get_buffer(self.cube_index_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_index_buffer(&self.device, cube_index_buffer);
                let cube_vertex_buffer = self
                    .resource_manager
                    .get_buffer(self.cube_vertex_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_vertex_buffer(&self.device, cube_vertex_buffer);
                // TODO: re-think API later when using more than one set
                self.render_context
                    .bind_descriptor_sets(&self.device, pipeline);

                self.render_context.draw_offset(&self.device, 36, 0, 0);
            }
            self.render_context.end_rendering(&self.device);

            // Depth image render pass
            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(
                    drawable,
                    RenderAttachmentDesc {
                        load_op: AttachmentLoadOp::Load,
                        ..Default::default()
                    },
                )],
                None,
            );
            {
                let pipeline = self
                    .resource_manager
                    .get_graphics_pipeline(self.texture_render_pipeline)
                    .unwrap();
                self.render_context
                    .bind_graphics_pipeline(&self.device, pipeline);
                let quad_index_buffer = self
                    .resource_manager
                    .get_buffer(self.quad_index_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_index_buffer(&self.device, quad_index_buffer);
                let quad_vertex_buffer = self
                    .resource_manager
                    .get_buffer(self.quad_vertex_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_vertex_buffer(&self.device, quad_vertex_buffer);
                // TODO: re-think API later when using more than one set
                self.render_context
                    .bind_descriptor_sets(&self.device, pipeline);

                self.render_context.draw_offset(&self.device, 6, 0, 0);
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

        // TODO: This is really messy
        {
            let depth_image = self
                .resource_manager
                .get_image_mut(self.depth_image_handle)
                .unwrap();
            depth_image.resize(&self.device, Size2D::new(width, height))?;
        }

        let depth_image = self
            .resource_manager
            .get_image(self.depth_image_handle)
            .unwrap();
        self.upload_context
            .begin(&self.device, self.device.setup_fence())?;
        {
            self.upload_context
                .transition_depth_to_read_only(&self.device, depth_image);
        }

        self.upload_context.end(
            &self.device,
            self.device.setup_fence(),
            self.device.present_queue(),
            &[],
            &[],
            &[],
        )?;

        let pipeline = self
            .resource_manager
            .get_graphics_pipeline(self.texture_render_pipeline)
            .unwrap();
        self.device.write_bind_group(
            pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(depth_image.bind_info(
                    self.resource_manager.get_sampler(self.sampler).unwrap(),
                    Layout::DepthStencilReadOnly,
                    0,
                )),
            }],
        )?;

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

        renderer.update().unwrap();
        renderer.draw().unwrap();

        renderer.resource_manager.consume(&renderer.device);
        renderer.device.bump_frame();
    }
}
