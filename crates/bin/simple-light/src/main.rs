use anyhow::Result;
use cinder::{
    command_queue::{
        AttachmentLoadOp, AttachmentStoreOp, ClearValue, RenderAttachment, RenderAttachmentDesc,
    },
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{vk, Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
    },
    Cinder,
};
use math::{mat::Mat4, point::Point2D, size::Size2D, vec::Vec3};
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
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/shadow_map_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/shadow_map_quad_shader_structs.rs"
));

struct QuadData {
    pipeline: GraphicsPipeline,
    index_buffer: Buffer,
    vertex_buffer: Buffer,
    bind_group: BindGroup,
    sampler: Sampler,
}

impl QuadData {
    pub fn new(cinder: &Cinder) -> Result<Self> {
        let vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map_quad.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map_quad.frag.spv"),
            Default::default(),
        )?;
        let pipeline = cinder.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            Default::default(),
        )?;

        let vertex_buffer = cinder.device.create_buffer_with_data(
            &[
                ShadowMapQuadVertex {
                    i_pos: [-1.0, -1.0],
                    i_uv: [0.0, 1.0],
                },
                ShadowMapQuadVertex {
                    i_pos: [-0.25, -1.0],
                    i_uv: [1.0, 1.0],
                },
                ShadowMapQuadVertex {
                    i_pos: [-0.25, -0.25],
                    i_uv: [1.0, 0.0],
                },
                ShadowMapQuadVertex {
                    i_pos: [-1.0, -0.25],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = cinder.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;
        let sampler = cinder.device.create_sampler(Default::default())?;

        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(0).unwrap())?;

        vertex_shader.destroy(&cinder.device);
        fragment_shader.destroy(&cinder.device);

        Ok(Self {
            pipeline,
            index_buffer,
            vertex_buffer,
            bind_group,
            sampler,
        })
    }

    pub fn cleanup(&self, cinder: &Cinder) {
        self.index_buffer.destroy(&cinder.device);
        self.vertex_buffer.destroy(&cinder.device);
        self.pipeline.destroy(&cinder.device);
        self.sampler.destroy(&cinder.device);
    }
}

struct MeshData {
    // TODO: The BindGroup stuff here is insanely messy, will fix very soon
    bind_group: BindGroup,
    shadow_bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl MeshData {
    pub fn new<T: Copy>(
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        shadow_pipeline: &GraphicsPipeline,
        vertex_buffer_data: &[T],
        index_buffer_data: &[u32],
    ) -> Result<Self> {
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(1).unwrap())?;
        let shadow_bind_group =
            BindGroup::new(&cinder.device, shadow_pipeline.bind_group_data(1).unwrap())?;
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
        cinder.device.write_bind_group(
            shadow_pipeline,
            &[BindGroupBindInfo {
                group: shadow_bind_group,
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
            shadow_bind_group,
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

fn rotate_point(p: Point2D<f32>, pivot: Point2D<f32>, angle: f32) -> Point2D<f32> {
    let s = angle.sin();
    let c = angle.cos();

    // translate point to origin
    let p = p - pivot;

    // rotate point
    let x_new = p.x() * c - p.y() * s;
    let y_new = p.x() * s + p.y() * c;

    Point2D::new(x_new + pivot.x(), y_new + pivot.y())
}

pub struct LightData {
    start_position: Vec3,
    position: Vec3,
    look_at: Vec3,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    light_data_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl LightData {
    pub fn new(
        cinder: &Cinder,
        position: Vec3,
        look_at: Vec3,
        _look_from: Vec3,
        aspect_ratio: f32,
    ) -> Result<Self> {
        let cylinder_mesh = geometry::SurfaceMesh::cylinder::<30>(0.3, 0.1);

        let ubo_buffer = cinder.device.create_buffer_with_data(
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(
                    position,
                    (position - look_at).normalized(),
                    Vec3::new(0.0, 1.0, 0.0),
                )
                .into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 0.01).into(),
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;

        Ok(Self {
            start_position: position,
            position,
            look_at,
            vertex_buffer: cinder.device.create_buffer_with_data(
                &cylinder_mesh.vertices,
                BufferDescription {
                    usage: BufferUsage::VERTEX,
                    ..Default::default()
                },
            )?,
            index_buffer: cinder.device.create_buffer_with_data(
                &cylinder_mesh.indices,
                BufferDescription {
                    usage: BufferUsage::INDEX,
                    ..Default::default()
                },
            )?,
            light_data_buffer: cinder.device.create_buffer_with_data(
                &[LitMeshGlobalLightData {
                    position: position.into(),
                    look_at: look_at.into(),
                }],
                BufferDescription {
                    usage: BufferUsage::UNIFORM,
                    ..Default::default()
                },
            )?,
            ubo_buffer,
        })
    }

    pub fn update(&mut self, angle: f32, aspect_ratio: f32) -> Result<()> {
        let p = Point2D::new(self.start_position.x(), self.start_position.z());
        let rotated_p = rotate_point(p, Point2D::zero(), angle);
        self.position = Vec3::new(rotated_p.x(), self.start_position.y(), rotated_p.y());
        self.light_data_buffer.mem_copy(0, &[self.position])?;

        self.ubo_buffer.mem_copy(
            0,
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(
                    self.position,
                    (self.position - self.look_at).normalized(),
                    Vec3::new(0.0, 1.0, 0.0),
                )
                .into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 0.01).into(),
            }],
        )?;

        Ok(())
    }

    pub fn cleanup(&self, cinder: &Cinder) {
        self.vertex_buffer.destroy(&cinder.device);
        self.index_buffer.destroy(&cinder.device);
        self.light_data_buffer.destroy(&cinder.device);
        self.ubo_buffer.destroy(&cinder.device);
    }
}

pub struct HelloCube {
    cinder: Cinder,
    depth_image: Image,
    shadow_map_image: Image,
    mesh_pipeline: GraphicsPipeline,
    light_pipeline: GraphicsPipeline,
    shadow_map_pipeline: GraphicsPipeline,
    quad_data: QuadData,
    camera_mesh_bind_group: BindGroup,
    camera_light_bind_group: BindGroup,
    shadow_map_bind_group: BindGroup,
    cube_mesh_data: MeshData,
    plane_mesh_data: MeshData,
    light_data: LightData,
    camera_ubo_buffer: Buffer,
    eye: Vec3,
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
        let shadow_map_image = cinder.device.create_image(
            Size2D::new(
                (surface_rect.width() as f32 / 2.0) as u32,
                (surface_rect.height() as f32 / 2.0) as u32,
            ),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::DepthSampled,
                ..Default::default()
            },
        )?;
        cinder.command_queue.transition_image(
            &cinder.device,
            &shadow_map_image,
            // TODO: get rid of `vk`
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
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
            Some(&light_fragment_shader),
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
            Some(&mesh_fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                backface_culling: false,
                ..Default::default()
            },
        )?;

        let shadow_map_vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map.vert.spv"),
            Default::default(),
        )?;
        let shadow_map_pipeline = cinder.device.create_graphics_pipeline(
            &shadow_map_vertex_shader,
            None,
            GraphicsPipelineDescription {
                color_format: None,
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;
        let shadow_map_bind_group = BindGroup::new(
            &cinder.device,
            shadow_map_pipeline.bind_group_data(0).unwrap(),
        )?;

        let quad_data = QuadData::new(&cinder)?;

        let cube_mesh_data = MeshData::new(
            &cinder,
            &mesh_pipeline,
            &shadow_map_pipeline,
            &[
                // Plane 1
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.0, 0.0, -1.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [0.0, 0.0, -1.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [0.0, 0.0, -1.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [0.0, 0.0, -1.0],
                },
                // Plane 2
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [0.0, 0.0, 1.0],
                },
                // Plane 3
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [-1.0, 0.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [-1.0, 0.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [-1.0, 0.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [-1.0, 0.0, 0.0],
                },
                // Plane 4
                LitMeshVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [1.0, 0.0, 0.0],
                },
                // Plane 5
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, -0.5],
                    i_normal: [0.0, -1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, -0.5],
                    i_normal: [0.0, -1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, -0.5, 0.5],
                    i_normal: [0.0, -1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, -0.5, 0.5],
                    i_normal: [0.0, -1.0, 0.0],
                },
                // Plane 6
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, -0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, -0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-0.5, 0.5, 0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [0.5, 0.5, 0.5],
                    i_normal: [0.0, 1.0, 0.0],
                },
            ],
            &[
                0, 1, 2, 2, 1, 3, // First plane
                5, 4, 7, 7, 4, 6, // Second plane
                9, 8, 11, 11, 8, 10, // Third Plane
                13, 12, 15, 15, 12, 14, // Fourth Plane
                17, 16, 19, 19, 16, 18, // Fifth Plane
                21, 20, 23, 23, 20, 22, // Sixth Plane
            ],
        )?;

        let plane_mesh_data = MeshData::new(
            &cinder,
            &mesh_pipeline,
            &shadow_map_pipeline,
            &[
                LitMeshVertex {
                    i_pos: [-5.0, -1.0, 5.0],
                    i_normal: [0.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [5.0, -1.0, 5.0],
                    i_normal: [0.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [-5.0, -1.0, -5.0],
                    i_normal: [0.0, 1.0, 0.0],
                },
                LitMeshVertex {
                    i_pos: [5.0, -1.0, -5.0],
                    i_normal: [0.0, 1.0, 0.0],
                },
            ],
            &[0, 1, 2, 2, 1, 3],
        )?;

        let camera_mesh_bind_group =
            BindGroup::new(&cinder.device, mesh_pipeline.bind_group_data(0).unwrap())?;
        let camera_light_bind_group =
            BindGroup::new(&cinder.device, light_pipeline.bind_group_data(0).unwrap())?;
        let eye = Vec3::new(5.0, 5.0, 0.0);
        let front = (Vec3::zero() - eye).normalized();
        let aspect_ratio = surface_rect.width() as f32 / surface_rect.height() as f32;
        let camera_ubo_buffer = cinder.device.create_buffer_with_data(
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(eye, front, Vec3::new(0.0, 1.0, 0.0)).into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 0.01).into(),
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;

        let light_data = LightData::new(
            &cinder,
            Vec3::new(4.0, 2.0, 0.0),
            Vec3::zero(),
            eye,
            aspect_ratio,
        )?;

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
        cinder.device.write_bind_group(
            &shadow_map_pipeline,
            &[BindGroupBindInfo {
                group: shadow_map_bind_group,
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(light_data.ubo_buffer.bind_info()),
            }],
        )?;

        mesh_vertex_shader.destroy(&cinder.device);
        mesh_fragment_shader.destroy(&cinder.device);
        light_vertex_shader.destroy(&cinder.device);
        light_fragment_shader.destroy(&cinder.device);
        shadow_map_vertex_shader.destroy(&cinder.device);

        Ok(Self {
            cinder,
            depth_image,
            shadow_map_image,
            mesh_pipeline,
            light_pipeline,
            shadow_map_pipeline,
            quad_data,
            camera_ubo_buffer,
            camera_mesh_bind_group,
            camera_light_bind_group,
            shadow_map_bind_group,
            cube_mesh_data,
            plane_mesh_data,
            light_data,
            eye,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let elapsed = self.cinder.init_time.elapsed().as_secs_f32();
        let scale = (elapsed / 5.0) * (2.0 * std::f32::consts::PI);

        self.cube_mesh_data
            .ubo_buffer
            .mem_copy(0, &[Mat4::rotate(scale, Vec3::new(0.0, 1.0, 0.0))])?;

        // TODO: Aspect ratio function in Cinder
        let surface_rect = self.cinder.device.surface_rect();
        let aspect_ratio = surface_rect.width() as f32 / surface_rect.height() as f32;
        self.light_data.update(elapsed, aspect_ratio)?;

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

        cmd_list.bind_viewport(&self.cinder.device, surface_rect, false);
        cmd_list.bind_scissor(&self.cinder.device, surface_rect);

        // Pass from light perspective
        cmd_list.begin_rendering(
            &self.cinder.device,
            self.shadow_map_image.size.into(),
            &[],
            Some(RenderAttachment::depth(
                &self.shadow_map_image,
                RenderAttachmentDesc {
                    layout: Layout::DepthAttachment,
                    clear_value: ClearValue::default_depth(),
                    ..Default::default()
                },
            )),
        );
        {
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.shadow_map_pipeline,
                0,
                &[self.shadow_map_bind_group],
            );
            cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.shadow_map_pipeline);

            // Draw Cube
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.shadow_map_pipeline,
                1,
                &[self.cube_mesh_data.shadow_bind_group],
            );
            cmd_list.bind_index_buffer(&self.cinder.device, &self.cube_mesh_data.index_buffer);
            cmd_list.bind_vertex_buffer(&self.cinder.device, &self.cube_mesh_data.vertex_buffer);
            cmd_list.draw_offset(
                &self.cinder.device,
                self.cube_mesh_data.index_buffer.num_elements().unwrap(),
                0,
                0,
            );

            // Draw Plane
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.shadow_map_pipeline,
                1,
                &[self.plane_mesh_data.shadow_bind_group],
            );
            cmd_list.bind_index_buffer(&self.cinder.device, &self.plane_mesh_data.index_buffer);
            cmd_list.bind_vertex_buffer(&self.cinder.device, &self.plane_mesh_data.vertex_buffer);
            cmd_list.draw_offset(
                &self.cinder.device,
                self.plane_mesh_data.index_buffer.num_elements().unwrap(),
                0,
                0,
            );
        }
        cmd_list.end_rendering(&self.cinder.device);

        // Main Pass
        cmd_list.begin_rendering(
            &self.cinder.device,
            surface_rect,
            &[RenderAttachment::color(
                swapchain_image,
                RenderAttachmentDesc {
                    clear_value: ClearValue::Color {
                        color: [0.4, 0.4, 0.4, 1.0],
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
        {
            // Bind Mesh Data
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.mesh_pipeline,
                0,
                &[self.camera_mesh_bind_group],
            );
            cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.mesh_pipeline);

            let scale = (self.cinder.init_time.elapsed().as_secs_f32() / 5.0)
                * (2.0 * std::f32::consts::PI);
            let light_color = [
                (scale.sin() + 1.0) / 2.0,
                (scale.cos() + 1.0) / 2.0,
                ((scale * 1.5).cos() + 1.0) / 2.0,
            ];
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
                    color: [161.0 / 255.0, 29.0 / 255.0, 194.0 / 255.0, 0.0],
                    view_from: [self.eye.x(), self.eye.y(), self.eye.z(), 0.0],
                    light_color,
                }],
                0,
            )?;
            cmd_list.draw_offset(
                &self.cinder.device,
                self.cube_mesh_data.index_buffer.num_elements().unwrap(),
                0,
                0,
            );

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
                    color: [201.0 / 255.0, 114.0 / 255.0, 38.0 / 255.0, 0.0],
                    view_from: [self.eye.x(), self.eye.y(), self.eye.z(), 0.0],
                    light_color,
                }],
                0,
            )?;
            cmd_list.draw_offset(
                &self.cinder.device,
                self.plane_mesh_data.index_buffer.num_elements().unwrap(),
                0,
                0,
            );

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
            cmd_list.set_vertex_bytes(
                &self.cinder.device,
                &self.light_pipeline,
                &light_color,
                0,
            )?;
            cmd_list.draw_offset(
                &self.cinder.device,
                self.light_data.index_buffer.num_elements().unwrap(),
                0,
                0,
            );
        }
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
        {
            cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.quad_data.pipeline);
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.light_pipeline,
                0,
                &[self.quad_data.bind_group],
            );
            cmd_list.bind_index_buffer(&self.cinder.device, &self.quad_data.index_buffer);
            cmd_list.bind_vertex_buffer(&self.cinder.device, &self.quad_data.vertex_buffer);
            cmd_list.draw_offset(
                &self.cinder.device,
                self.quad_data.index_buffer.num_elements().unwrap(),
                0,
                0,
            );
        }
        cmd_list.end_rendering(&self.cinder.device);

        self.cinder
            .swapchain
            .present(&self.cinder.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)?;
        self.depth_image
            .resize(&self.cinder.device, Size2D::new(width, height))?;
        self.shadow_map_image.resize(
            &self.cinder.device,
            Size2D::new((width as f32 / 2.0) as u32, (height as f32 / 2.0) as u32),
        )?;
        self.cinder.command_queue.transition_image(
            &self.cinder.device,
            &self.shadow_map_image,
            // TODO: get rid of `vk`
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        )?;
        Ok(())
    }
}

impl Drop for HelloCube {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
        self.camera_ubo_buffer.destroy(&self.cinder.device);
        self.mesh_pipeline.destroy(&self.cinder.device);
        self.light_pipeline.destroy(&self.cinder.device);
        self.shadow_map_pipeline.destroy(&self.cinder.device);
        self.depth_image.destroy(&self.cinder.device);
        self.shadow_map_image.destroy(&self.cinder.device);
        self.cube_mesh_data.cleanup(&self.cinder);
        self.plane_mesh_data.cleanup(&self.cinder);
        self.light_data.cleanup(&self.cinder);
        self.quad_data.cleanup(&self.cinder);
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
