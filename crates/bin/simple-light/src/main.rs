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
use math::{
    mat::Mat4,
    size::Size2D,
    vec::{Vec3, Vec4},
};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/light_shader_structs.rs"
));

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/lit_mesh_shader_structs.rs"
));

struct MeshData {
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl MeshData {
    pub fn new<T: Copy>(
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        vertex_buffer_data: &[T],
        index_buffer_data: &[u32],
    ) -> Result<Self> {
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(1).unwrap())?;
        let ubo_buffer = cinder.device.create_buffer(
            std::mem::size_of::<LitMeshModelUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        ubo_buffer.mem_copy(0, &[Mat4::identity()])?;
        cinder.device.write_bind_group(
            pipeline,
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

pub struct LightData {
    position: Vec4,
    look_at: Vec4,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    light_data_buffer: Buffer,
}

impl LightData {
    pub fn new(cinder: &Cinder, position: Vec4, look_at: Vec4) -> Result<Self> {
        Ok(Self {
            position,
            look_at,
            vertex_buffer: cinder.device.create_buffer_with_data(
                &[
                    LightVertex {
                        i_pos: [-0.4, 0.2, -0.2],
                    },
                    LightVertex {
                        i_pos: [0.4, 0.2, -0.2],
                    },
                    LightVertex {
                        i_pos: [-0.4, -0.2, -0.2],
                    },
                    LightVertex {
                        i_pos: [0.4, -0.2, -0.2],
                    },
                    LightVertex {
                        i_pos: [-0.4, 0.2, 0.2],
                    },
                    LightVertex {
                        i_pos: [0.4, 0.2, 0.2],
                    },
                    LightVertex {
                        i_pos: [-0.4, -0.2, 0.2],
                    },
                    LightVertex {
                        i_pos: [0.4, -0.2, 0.2],
                    },
                ],
                BufferDescription {
                    usage: BufferUsage::VERTEX,
                    ..Default::default()
                },
            )?,
            index_buffer: cinder.device.create_buffer_with_data(
                &[
                    0, 1, 2, 2, 1, 3, // First plane
                    5, 4, 7, 7, 4, 6, // Second plane
                    3, 7, 2, 2, 7, 6, // Third Plane
                    0, 4, 1, 1, 4, 5, // Fourth Plane
                    1, 5, 3, 3, 5, 7, // Fifth Plane
                    4, 0, 6, 6, 0, 2, // Sixth Plane
                ],
                BufferDescription {
                    usage: BufferUsage::INDEX,
                    ..Default::default()
                },
            )?,
            light_data_buffer: cinder.device.create_buffer_with_data(
                &[LightGlobalLightData {
                    position: position.into(),
                    look_at: look_at.into(),
                }],
                BufferDescription {
                    usage: BufferUsage::UNIFORM,
                    ..Default::default()
                },
            )?,
        })
    }

    pub fn update(&mut self, scale: f32) -> Result<()> {
        self.position = Vec4::new(scale.sin() * 2.0, 0.0, 0.0, 1.0);
        self.light_data_buffer.mem_copy(0, &[self.position])?;

        Ok(())
    }

    pub fn cleanup(&self, cinder: &Cinder) {
        self.vertex_buffer.destroy(&cinder.device);
        self.index_buffer.destroy(&cinder.device);
        self.light_data_buffer.destroy(&cinder.device);
    }
}

pub struct HelloCube {
    cinder: Cinder,
    depth_image: Image,
    mesh_pipeline: GraphicsPipeline,
    light_pipeline: GraphicsPipeline,
    camera_mesh_bind_group: BindGroup,
    camera_light_bind_group: BindGroup,
    cube_mesh_data: MeshData,
    plane_mesh_data: MeshData,
    light_data: LightData,
    camera_ubo_buffer: Buffer,
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

        let light_vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/light.vert.spv"),
            Default::default(),
        )?;
        let light_fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/light.frag.spv"),
            Default::default(),
        )?;
        let light_pipeline = cinder.device.create_graphics_pipeline(
            &light_vertex_shader,
            &light_fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let mesh_vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/lit_mesh.vert.spv"),
            Default::default(),
        )?;
        let mesh_fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/lit_mesh.frag.spv"),
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

        let cube_mesh_data = MeshData::new(
            &cinder,
            &mesh_pipeline,
            &[
                // Plane at z: -0.5
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                // Plane at z: 0.5
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.5, 0.5, 0.5],
                },
                LitMeshVertex {
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

        let plane_mesh_data = MeshData::new(
            &cinder,
            &mesh_pipeline,
            &[
                LitMeshVertex {
                    i_pos: [-5.0, -1.0, 5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [5.0, -1.0, 5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-5.0, -1.0, -5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [5.0, -1.0, -5.0],
                    i_normal: [1.0, 1.0, 0.0],
                },
            ],
            &[0, 1, 2, 2, 1, 3],
        )?;

        let camera_mesh_bind_group =
            BindGroup::new(&cinder.device, mesh_pipeline.bind_group_data(0).unwrap())?;
        let camera_light_bind_group =
            BindGroup::new(&cinder.device, light_pipeline.bind_group_data(0).unwrap())?;
        let eye = Vec3::new(6.0, 4.0, 0.0);
        let front = (Vec3::zero() - eye).normalized();
        let camera_ubo_buffer = cinder.device.create_buffer_with_data(
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(eye, front, Vec3::new(0.0, 1.0, 0.0)).into(),
                proj: camera::new_infinite_perspective_proj(
                    surface_rect.width() as f32 / surface_rect.height() as f32,
                    30.0,
                    0.01,
                )
                .into(),
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;

        let light_data = LightData::new(&cinder, Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::zero())?;

        cinder.device.write_bind_group(
            &mesh_pipeline,
            &[
                BindGroupBindInfo {
                    group: camera_mesh_bind_group,
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(camera_ubo_buffer.bind_info()),
                },
                BindGroupBindInfo {
                    group: camera_mesh_bind_group,
                    dst_binding: 1,
                    data: BindGroupWriteData::Uniform(light_data.light_data_buffer.bind_info()),
                },
            ],
        )?;
        cinder.device.write_bind_group(
            &mesh_pipeline,
            &[
                BindGroupBindInfo {
                    group: camera_light_bind_group,
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(camera_ubo_buffer.bind_info()),
                },
                BindGroupBindInfo {
                    group: camera_light_bind_group,
                    dst_binding: 1,
                    data: BindGroupWriteData::Uniform(light_data.light_data_buffer.bind_info()),
                },
            ],
        )?;

        mesh_vertex_shader.destroy(&cinder.device);
        mesh_fragment_shader.destroy(&cinder.device);
        light_vertex_shader.destroy(&cinder.device);
        light_fragment_shader.destroy(&cinder.device);

        Ok(Self {
            cinder,
            depth_image,
            mesh_pipeline,
            light_pipeline,
            camera_ubo_buffer,
            camera_mesh_bind_group,
            camera_light_bind_group,
            cube_mesh_data,
            plane_mesh_data,
            light_data,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale =
            (self.cinder.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);

        self.cube_mesh_data
            .ubo_buffer
            .mem_copy(0, &[Mat4::rotate(scale, Vec3::new(0.0, 1.0, 0.0))])?;

        self.light_data.update(scale)?;

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
            &[RenderAttachment::color(
                swapchain_image,
                RenderAttachmentDesc {
                    clear_value: ClearValue::Color {
                        color: [0.9, 0.9, 0.9, 1.0],
                    },
                    ..Default::default()
                },
            )],
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

        cmd_list.bind_viewport(&self.cinder.device, surface_rect, false);
        cmd_list.bind_scissor(&self.cinder.device, surface_rect);

        // Bind Mesh Data
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.mesh_pipeline,
            0,
            &[self.camera_mesh_bind_group],
        );
        cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.mesh_pipeline);

        // Draw Cube
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.mesh_pipeline,
            1,
            &[self.cube_mesh_data.bind_group],
        );
        cmd_list.bind_index_buffer(&self.cinder.device, &self.cube_mesh_data.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.cube_mesh_data.vertex_buffer);
        cmd_list.set_vertex_bytes(
            &self.cinder.device,
            &self.mesh_pipeline,
            &[LitMeshConstants {
                color: [161.0 / 255.0, 29.0 / 255.0, 194.0 / 255.0],
            }],
            0,
        )?;
        cmd_list.draw_offset(&self.cinder.device, 36, 0, 0);

        // Draw Plane
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.mesh_pipeline,
            1,
            &[self.plane_mesh_data.bind_group],
        );
        cmd_list.bind_index_buffer(&self.cinder.device, &self.plane_mesh_data.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.plane_mesh_data.vertex_buffer);
        cmd_list.set_vertex_bytes(
            &self.cinder.device,
            &self.mesh_pipeline,
            &[LitMeshConstants {
                color: [201.0 / 255.0, 114.0 / 255.0, 38.0 / 255.0],
            }],
            0,
        )?;
        cmd_list.draw_offset(&self.cinder.device, 6, 0, 0);

        // Draw Light
        cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.light_pipeline);
        cmd_list.bind_descriptor_sets(
            &self.cinder.device,
            &self.light_pipeline,
            0,
            &[self.camera_light_bind_group],
        );
        cmd_list.bind_index_buffer(&self.cinder.device, &self.light_data.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.light_data.vertex_buffer);
        let scale =
            (self.cinder.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        cmd_list.set_vertex_bytes(
            &self.cinder.device,
            &self.light_pipeline,
            &Vec3::new(
                (scale.sin() + 1.0) / 2.0,
                (scale.cos() + 1.0) / 2.0,
                ((scale * 1.5).cos() + 1.0) / 2.0,
            ),
            0,
        )?;
        cmd_list.draw_offset(&self.cinder.device, 36, 0, 0);

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
        self.mesh_pipeline.destroy(&self.cinder.device);
        self.light_pipeline.destroy(&self.cinder.device);
        self.depth_image.destroy(&self.cinder.device);
        self.cube_mesh_data.cleanup(&self.cinder);
        self.plane_mesh_data.cleanup(&self.cinder);
        self.light_data.cleanup(&self.cinder);
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
