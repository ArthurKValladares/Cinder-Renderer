use anyhow::Result;
use cinder::{
    command_queue::{AttachmentStoreOp, ClearValue, RenderAttachment, RenderAttachmentDesc},
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
    },
    Cinder,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use sdl2::{event::Event, keyboard::Keycode, sys::SDL_HapticLeftRight, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/light_shader_structs.rs"
));

struct MeshData {
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl MeshData {
    pub fn new(
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        vertex_buffer_data: &[LightVertex],
        index_buffer_data: &[u32],
    ) -> Result<Self> {
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(1).unwrap())?;
        let ubo_buffer = cinder.device.create_buffer(
            std::mem::size_of::<LightModelUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        ubo_buffer.mem_copy(0, &[Mat4::identity()])?;
        cinder.device.write_bind_group(
            &pipeline,
            &[BindGroupBindInfo {
                group: bind_group,
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
            }],
        )?;
        let vertex_buffer = cinder.device.create_buffer_with_data(
            vertex_buffer_data,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = cinder.device.create_buffer_with_data(
            index_buffer_data,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        Ok(Self {
            bind_group,
            vertex_buffer,
            index_buffer,
            ubo_buffer,
        })
    }

    pub fn cleanup(&self, cinder: &Cinder) {
        self.index_buffer.destroy(&cinder.device);
        self.vertex_buffer.destroy(&cinder.device);
        self.ubo_buffer.destroy(&cinder.device);
    }
}

pub struct HelloCube {
    cinder: Cinder,
    depth_image: Image,
    pipeline: GraphicsPipeline,
    camera_bind_group: BindGroup,
    camera_ubo_buffer: Buffer,
    cube_mesh_data: MeshData,
    plane_mesh_data: MeshData,
    light_mesh_data: MeshData,
}

impl HelloCube {
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
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;

        let vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/light.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/light.frag.spv"),
            Default::default(),
        )?;
        let pipeline = cinder.device.create_graphics_pipeline(
            &vertex_shader,
            &fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;
        let camera_bind_group =
            BindGroup::new(&cinder.device, pipeline.bind_group_data(0).unwrap())?;
        let camera_ubo_buffer = cinder.device.create_buffer(
            std::mem::size_of::<LightCameraUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        let eye = Vec3::new(6.0, 4.0, 0.0);
        let front = (Vec3::zero() - eye).normalized();
        camera_ubo_buffer.mem_copy(
            0,
            &[
                camera::look_to(eye, front, Vec3::new(0.0, 1.0, 0.0)),
                camera::new_infinite_perspective_proj(
                    surface_rect.width() as f32 / surface_rect.height() as f32,
                    30.0,
                    0.01,
                ),
            ],
        )?;
        cinder.device.write_bind_group(
            &pipeline,
            &[BindGroupBindInfo {
                group: camera_bind_group,
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(camera_ubo_buffer.bind_info()),
            }],
        )?;

        let cube_mesh_data = MeshData::new(
            &cinder,
            &pipeline,
            &[
                // Plane at z: -0.5
                LightVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LightVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LightVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LightVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                // Plane at z: 0.5
                LightVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LightVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LightVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LightVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
            ],
            &[
                0, 1, 2, 2, 1, 3, // First plane
                5, 4, 7, 7, 4, 6, // Second plane
                3, 7, 2, 2, 7, 6, // Third Plane
                0, 4, 1, 1, 4, 5, // Fourth Plane
                1, 5, 3, 3, 5, 7, // Fifth Plane
                4, 0, 6, 6, 0, 2, // Sixth Plane
            ],
        )?;

        let light_mesh_data = MeshData::new(
            &cinder,
            &pipeline,
            &[
                LightVertex {
                    i_pos: [-0.4, 0.2, -0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [0.4, 0.2, -0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [-0.4, -0.2, -0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [0.4, -0.2, -0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [-0.4, 0.2, 0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [0.4, 0.2, 0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [-0.4, -0.2, 0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
                LightVertex {
                    i_pos: [0.4, -0.2, 0.2],
                    i_normal: [1.0, 1.0, 1.0],
                },
            ],
            &[
                0, 1, 2, 2, 1, 3, // First plane
                5, 4, 7, 7, 4, 6, // Second plane
                3, 7, 2, 2, 7, 6, // Third Plane
                0, 4, 1, 1, 4, 5, // Fourth Plane
                1, 5, 3, 3, 5, 7, // Fifth Plane
                4, 0, 6, 6, 0, 2, // Sixth Plane
            ],
        )?;

        let plane_mesh_data = MeshData::new(
            &cinder,
            &pipeline,
            &[
                LightVertex {
                    i_pos: [-5.0, -1.0, 5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
                LightVertex {
                    i_pos: [5.0, -1.0, 5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
                LightVertex {
                    i_pos: [-5.0, -1.0, -5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
                LightVertex {
                    i_pos: [5.0, -1.0, -5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
            ],
            &[0, 1, 2, 2, 1, 3],
        )?;

        vertex_shader.destroy(&cinder.device);
        fragment_shader.destroy(&cinder.device);

        Ok(Self {
            cinder,
            depth_image,
            pipeline,
            camera_ubo_buffer,
            camera_bind_group,
            cube_mesh_data,
            plane_mesh_data,
            light_mesh_data,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale =
            (self.cinder.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);

        self.cube_mesh_data
            .ubo_buffer
            .mem_copy(0, &[Mat4::rotate(scale, Vec3::new(0.0, 1.0, 0.0))])?;

        // TODO: need to rotate it down a bit so it actually points towards the box
        self.light_mesh_data.ubo_buffer.mem_copy(
            0,
            &[Mat4::rotate(scale / 2.0, Vec3::new(0.0, 1.0, 0.0))
                * Mat4::translate(Vec3::new(-5.0, (scale * 2.0).sin() * 0.5, 0.0))],
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

        cmd_list.begin_rendering(
            &self.cinder.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
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
        cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.pipeline);
        cmd_list.bind_viewport(&self.cinder.device, surface_rect, false);
        cmd_list.bind_scissor(&self.cinder.device, surface_rect);
        // TODO: review how we get first_set, get it from shader, forced or no?
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.pipeline,
            0,
            &[self.camera_bind_group],
        );

        // Draw Cube
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.pipeline,
            1,
            &[self.cube_mesh_data.bind_group],
        );
        cmd_list.bind_index_buffer(&self.cinder.device, &self.cube_mesh_data.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.cube_mesh_data.vertex_buffer);
        cmd_list.draw_offset(&self.cinder.device, 36, 0, 0);

        // Draw Light
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.pipeline,
            1,
            &[self.light_mesh_data.bind_group],
        );
        cmd_list.bind_index_buffer(&self.cinder.device, &self.light_mesh_data.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.light_mesh_data.vertex_buffer);
        cmd_list.draw_offset(&self.cinder.device, 36, 0, 0);

        // Draw Plane
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.pipeline,
            1,
            &[self.plane_mesh_data.bind_group],
        );
        cmd_list.bind_index_buffer(&self.cinder.device, &self.plane_mesh_data.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.plane_mesh_data.vertex_buffer);
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
        Ok(())
    }
}

impl Drop for HelloCube {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
        self.camera_ubo_buffer.destroy(&self.cinder.device);
        self.pipeline.destroy(&self.cinder.device);
        self.depth_image.destroy(&self.cinder.device);
        self.cube_mesh_data.cleanup(&self.cinder);
        self.plane_mesh_data.cleanup(&self.cinder);
        self.light_mesh_data.cleanup(&self.cinder);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "hello-cube",
            ..Default::default()
        },
    )
    .unwrap();

    let mut renderer = HelloCube::new(&sdl.window).unwrap();

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
