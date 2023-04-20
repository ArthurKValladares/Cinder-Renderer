use anyhow::Result;
use cinder::{
    cinder::Cinder,
    command_queue::{
        AttachmentLoadOp, AttachmentStoreOp, ClearValue, RenderAttachment, RenderAttachmentDesc,
    },
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{vk, Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
    },
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
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
    cinder: Cinder,
    depth_image: Image,
    mesh_pipeline: GraphicsPipeline,
    texture_pipeline: GraphicsPipeline,
    cube_vertex_buffer: Buffer,
    cube_index_buffer: Buffer,
    ubo_buffer: Buffer,
    quad_vertex_buffer: Buffer,
    quad_index_buffer: Buffer,
    sampler: Sampler,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let (width, height) = window.drawable_size();
        let cinder = Cinder::new(window, width, height)?;

        //
        // Create App Resources
        //
        let surface_rect = cinder.device.surface_rect();
        let depth_image = cinder.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::DepthSampled,
                ..Default::default()
            },
        )?;
        cinder.command_queue.transition_image(
            &cinder.device,
            &depth_image,
            // TODO: get rid of `vk`
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        )?;
        let mesh_vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.vert.spv"),
            Default::default(),
        )?;
        let mesh_fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.frag.spv"),
            Default::default(),
        )?;
        let mesh_pipeline = cinder.device.create_graphics_pipeline(
            &mesh_vertex_shader,
            &mesh_fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let texture_vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.vert.spv"),
            Default::default(),
        )?;
        let texture_fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.frag.spv"),
            Default::default(),
        )?;
        let texture_pipeline = cinder.device.create_graphics_pipeline(
            &texture_vertex_shader,
            &texture_fragment_shader,
            Default::default(),
        )?;

        let cube_vertex_buffer = cinder.device.create_buffer_with_data(
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
        )?;
        let cube_index_buffer = cinder.device.create_buffer_with_data(
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

        let ubo_buffer = cinder.device.create_buffer(
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
            cinder.device.write_bind_group(
                &mesh_pipeline,
                &[BindGroupBindInfo {
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
                }],
            )?;
        }
        let quad_vertex_buffer = cinder.device.create_buffer_with_data(
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
        )?;
        let quad_index_buffer = cinder.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let sampler = cinder.device.create_sampler(Default::default())?;
        cinder.device.write_bind_group(
            &texture_pipeline,
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
        // Cleanup
        //
        texture_vertex_shader.destroy(&cinder.device);
        texture_fragment_shader.destroy(&cinder.device);
        mesh_vertex_shader.destroy(&cinder.device);
        mesh_fragment_shader.destroy(&cinder.device);

        Ok(Self {
            cinder,
            depth_image,
            mesh_pipeline,
            texture_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            ubo_buffer,
            quad_vertex_buffer,
            quad_index_buffer,
            sampler,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale =
            (self.cinder.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        self.ubo_buffer.mem_copy(
            util::offset_of!(DepthMeshUniformBufferObject, model) as u64,
            &[Mat4::rotate(scale, Vec3::new(1.0, 1.0, 0.0))],
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.cinder.device.surface_rect();

        let cmd_list = self
            .cinder
            .command_queue
            .get_command_list(&self.cinder.device)?;
        let swapchain_image = self
            .cinder
            .swapchain
            .acquire_image(&self.cinder.device, &cmd_list)?;

        cmd_list.bind_viewport(&self.cinder.device, surface_rect, true);
        cmd_list.bind_scissor(&self.cinder.device, surface_rect);

        // Mesh render pass
        cmd_list.begin_rendering(
            &self.cinder.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
            Some(RenderAttachment::depth(
                &self.depth_image,
                RenderAttachmentDesc {
                    store_op: AttachmentStoreOp::Store,
                    layout: Layout::DepthAttachment,
                    clear_value: ClearValue::default_depth(),
                    ..Default::default()
                },
            )),
        );
        cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.mesh_pipeline);
        cmd_list.bind_index_buffer(&self.cinder.device, &self.cube_index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.cube_vertex_buffer);
        // TODO: re-think API later when using more than one set
        cmd_list.bind_descriptor_sets(&self.cinder.device, &self.mesh_pipeline);
        cmd_list.draw_offset(&self.cinder.device, 36, 0, 0);
        cmd_list.end_rendering(&self.cinder.device);

        // Depth image render pass
        cmd_list.begin_rendering(
            &self.cinder.device,
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
        cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.texture_pipeline);
        cmd_list.bind_index_buffer(&self.cinder.device, &self.quad_index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.quad_vertex_buffer);
        // TODO: re-think API later when using more than one set
        cmd_list.bind_descriptor_sets(&self.cinder.device, &self.texture_pipeline);
        cmd_list.draw_offset(&self.cinder.device, 6, 0, 0);
        cmd_list.end_rendering(&self.cinder.device);

        self.cinder
            .swapchain
            .present(&self.cinder.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)?;
        self.depth_image
            .resize(&self.cinder.device, Size2D::new(width, height))?;

        self.cinder.device.write_bind_group(
            &self.texture_pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(self.depth_image.bind_info(
                    &self.sampler,
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
        self.cinder.device.wait_idle().ok();
        self.depth_image.destroy(&self.cinder.device);
        self.mesh_pipeline.destroy(&self.cinder.device);
        self.texture_pipeline.destroy(&self.cinder.device);
        self.cube_vertex_buffer.destroy(&self.cinder.device);
        self.cube_index_buffer.destroy(&self.cinder.device);
        self.ubo_buffer.destroy(&self.cinder.device);
        self.quad_vertex_buffer.destroy(&self.cinder.device);
        self.quad_index_buffer.destroy(&self.cinder.device);
        self.sampler.destroy(&self.cinder.device);
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
        renderer.cinder.start_frame().unwrap();

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

        renderer.cinder.end_frame();
    }
}
