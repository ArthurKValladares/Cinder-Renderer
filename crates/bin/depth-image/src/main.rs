use anyhow::Result;
use cinder::{
    command_queue::{
        AttachmentLoadOp, AttachmentStoreOp, ClearValue, CommandQueue, RenderAttachment,
        RenderAttachmentDesc,
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{vk, Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        ResourceManager,
    },
    swapchain::Swapchain,
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

pub struct Renderer {
    resource_manager: ResourceManager,
    device: Device,
    swapchain: Swapchain,
    command_queue: CommandQueue,
    init_time: Instant,
    depth_image_handle: ResourceId<Image>,
    mesh_render_pipeline_handle: ResourceId<GraphicsPipeline>,
    texture_render_pipeline_handle: ResourceId<GraphicsPipeline>,
    cube_vertex_buffer_handle: ResourceId<Buffer>,
    cube_index_buffer_handle: ResourceId<Buffer>,
    ubo_buffer_handle: ResourceId<Buffer>,
    quad_vertex_buffer_handle: ResourceId<Buffer>,
    quad_index_buffer_handle: ResourceId<Buffer>,
    sampler_handle: ResourceId<Sampler>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height)?;
        let command_queue: CommandQueue = CommandQueue::new(&device)?;
        let swapchain = Swapchain::new(&device)?;
        let surface_rect = device.surface_rect();
        let depth_image = device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::DepthSampled,
                ..Default::default()
            },
        )?;
        command_queue.transition_image(
            &device,
            &depth_image,
            // TODO: get rid of `vk`
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        )?;
        let mesh_vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.vert.spv"),
            Default::default(),
        )?;
        let mesh_fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.frag.spv"),
            Default::default(),
        )?;
        let mesh_render_pipeline = device.create_graphics_pipeline(
            &mesh_vertex_shader,
            &mesh_fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let texture_vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.vert.spv"),
            Default::default(),
        )?;
        let texture_fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.frag.spv"),
            Default::default(),
        )?;
        let texture_render_pipeline = device.create_graphics_pipeline(
            &texture_vertex_shader,
            &texture_fragment_shader,
            Default::default(),
        )?;

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

        let ubo_buffer = device.create_buffer(
            std::mem::size_of::<DepthMeshUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        {
            ubo_buffer.mem_copy(
                util::offset_of!(DepthMeshUniformBufferObject, view) as u64,
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
            device.write_bind_group(
                &mesh_render_pipeline,
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

        let sampler = device.create_sampler(&device, Default::default())?;
        device.write_bind_group(
            &texture_render_pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(depth_image.bind_info(
                    &sampler,
                    Layout::DepthStencilReadOnly,
                    None,
                )),
            }],
        )?;

        //
        // Add resources to ResourceManager
        //
        let depth_image_handle = resource_manager.insert_image(depth_image);
        let mesh_render_pipeline_handle =
            resource_manager.insert_graphics_pipeline(mesh_render_pipeline);
        let texture_render_pipeline_handle =
            resource_manager.insert_graphics_pipeline(texture_render_pipeline);
        let ubo_buffer_handle = resource_manager.insert_buffer(ubo_buffer);
        let sampler_handle = resource_manager.insert_sampler(sampler);

        //
        // Cleanup
        //
        texture_vertex_shader.destroy(device.raw());
        texture_fragment_shader.destroy(device.raw());
        mesh_vertex_shader.destroy(device.raw());
        mesh_fragment_shader.destroy(device.raw());

        let init_time = Instant::now();
        Ok(Self {
            resource_manager,
            device,
            swapchain,
            depth_image_handle,
            command_queue,
            mesh_render_pipeline_handle,
            texture_render_pipeline_handle,
            cube_vertex_buffer_handle,
            cube_index_buffer_handle,
            ubo_buffer_handle,
            quad_vertex_buffer_handle,
            quad_index_buffer_handle,
            sampler_handle,
            init_time,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale = (self.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        let ubo_buffer = self
            .resource_manager
            .buffers
            .get_mut(self.ubo_buffer_handle)
            .unwrap();
        ubo_buffer.mem_copy(
            util::offset_of!(DepthMeshUniformBufferObject, model) as u64,
            &[Mat4::rotate(scale, Vec3::new(1.0, 1.0, 0.0))],
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.device.surface_rect();
        let depth_image = self
            .resource_manager
            .images
            .get(self.depth_image_handle)
            .unwrap();
        let mesh_pipeline = self
            .resource_manager
            .graphics_pipelines
            .get(self.mesh_render_pipeline_handle)
            .unwrap();
        let texture_pipeline = self
            .resource_manager
            .graphics_pipelines
            .get(self.texture_render_pipeline_handle)
            .unwrap();
        let cube_index_buffer = self
            .resource_manager
            .buffers
            .get(self.cube_index_buffer_handle)
            .unwrap();
        let cube_vertex_buffer = self
            .resource_manager
            .buffers
            .get(self.cube_vertex_buffer_handle)
            .unwrap();
        let quad_index_buffer = self
            .resource_manager
            .buffers
            .get(self.quad_index_buffer_handle)
            .unwrap();
        let quad_vertex_buffer = self
            .resource_manager
            .buffers
            .get(self.quad_vertex_buffer_handle)
            .unwrap();

        let cmd_list = self.command_queue.get_command_list(&self.device)?;
        let swapchain_image = self.swapchain.acquire_image(&self.device, &cmd_list)?;

        cmd_list.bind_viewport(&self.device, surface_rect, true);
        cmd_list.bind_scissor(&self.device, surface_rect);

        // Mesh render pass
        cmd_list.begin_rendering(
            &self.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
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
        cmd_list.bind_graphics_pipeline(&self.device, mesh_pipeline);
        cmd_list.bind_index_buffer(&self.device, cube_index_buffer);
        cmd_list.bind_vertex_buffer(&self.device, cube_vertex_buffer);
        // TODO: re-think API later when using more than one set
        cmd_list.bind_descriptor_sets(&self.device, mesh_pipeline);
        cmd_list.draw_offset(&self.device, 36, 0, 0);
        cmd_list.end_rendering(&self.device);

        // Depth image render pass
        cmd_list.begin_rendering(
            &self.device,
            surface_rect,
            &[RenderAttachment::color(
                swapchain_image,
                RenderAttachmentDesc {
                    load_op: AttachmentLoadOp::Load,
                    ..Default::default()
                },
            )],
            None,
        );
        cmd_list.bind_graphics_pipeline(&self.device, texture_pipeline);
        cmd_list.bind_index_buffer(&self.device, quad_index_buffer);
        cmd_list.bind_vertex_buffer(&self.device, quad_vertex_buffer);
        // TODO: re-think API later when using more than one set
        cmd_list.bind_descriptor_sets(&self.device, texture_pipeline);
        cmd_list.draw_offset(&self.device, 6, 0, 0);
        cmd_list.end_rendering(&self.device);

        self.swapchain
            .present(&self.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.swapchain.resize(&self.device)?;

        let depth_image = self
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&self.device, Size2D::new(width, height))?;
        self.command_queue.transition_image(
            &self.device,
            depth_image,
            // TODO: get rid of `vk`
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        )?;

        let pipeline = self
            .resource_manager
            .graphics_pipelines
            .get(self.texture_render_pipeline_handle)
            .unwrap();
        let sampler = self
            .resource_manager
            .samplers
            .get(self.sampler_handle)
            .unwrap();
        self.device.write_bind_group(
            pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(depth_image.bind_info(
                    sampler,
                    Layout::DepthStencilReadOnly,
                    None,
                )),
            }],
        )?;

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
        WindowDescription {
            title: "depth-image",
        },
    )
    .unwrap();

    let mut renderer = Renderer::new(&sdl.window).unwrap();

    'running: loop {
        renderer.device.new_frame().unwrap();

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
