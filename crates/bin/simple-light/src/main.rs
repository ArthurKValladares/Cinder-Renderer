use anyhow::Result;
use cinder::{
    command_queue::{
        AttachmentLoadOp, AttachmentStoreOp, ClearValue, RenderAttachment, RenderAttachmentDesc,
    },
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupData, BindGroupWriteData},
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

struct TexturedQuadData {
    index_buffer: Buffer,
    vertex_buffer: Buffer,
}

impl TexturedQuadData {
    pub fn new(
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        bind_group: BindGroup,
        image: &Image,
        sampler: &Sampler,
    ) -> Result<Self> {
        let vertex_buffer = cinder.device.create_buffer_with_data(
            &[
                ShadowMapQuadVertex {
                    i_pos: [-1.0, -1.0],
                    i_uv: [0.0, 0.0],
                },
                ShadowMapQuadVertex {
                    i_pos: [-0.25, -1.0],
                    i_uv: [1.0, 0.0],
                },
                ShadowMapQuadVertex {
                    i_pos: [-0.25, -0.25],
                    i_uv: [1.0, 1.0],
                },
                ShadowMapQuadVertex {
                    i_pos: [-1.0, -0.25],
                    i_uv: [0.0, 1.0],
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

        cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(image.bind_info(
                sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;

        Ok(Self {
            index_buffer,
            vertex_buffer,
        })
    }

    pub fn cleanup(&self, cinder: &Cinder) {
        self.index_buffer.destroy(&cinder.device);
        self.vertex_buffer.destroy(&cinder.device);
    }
}

struct MeshData {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    model_bind_group: BindGroup,
    model_transform_buffer: Buffer,
}

impl MeshData {
    pub fn new<T: Copy>(
        cinder: &Cinder,
        pipeline: &GraphicsPipeline,
        vertex_buffer_data: &[T],
        index_buffer_data: &[u32],
    ) -> Result<Self> {
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

        let model_bind_group =
            BindGroup::new(&cinder.device, &pipeline.bind_group_data(1).unwrap())?;

        let model_transform_buffer = cinder.device.create_buffer(
            std::mem::size_of::<LitMeshModelUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        model_transform_buffer.mem_copy(0, &[Mat4::identity()])?;
        cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: model_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::Uniform(model_transform_buffer.bind_info()),
        }])?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            model_bind_group,
            model_transform_buffer,
        })
    }

    pub fn cleanup(&self, cinder: &Cinder) {
        self.index_buffer.destroy(&cinder.device);
        self.vertex_buffer.destroy(&cinder.device);
        self.model_transform_buffer.destroy(&cinder.device);
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
    data_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl LightData {
    fn new(cinder: &Cinder, camera: &CameraData, position: Vec3, look_at: Vec3) -> Result<Self> {
        let cylinder_mesh = geometry::SurfaceMesh::cylinder::<30>(0.3, 0.1);

        let data_buffer = cinder.device.create_buffer_with_data(
            &[LitMeshGlobalLightData {
                position: position.into(),
                look_at: look_at.into(),
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;

        cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: camera.bind_group,
            dst_binding: 1,
            data: BindGroupWriteData::Uniform(data_buffer.bind_info()),
        }])?;

        let vertex_buffer = cinder.device.create_buffer_with_data(
            &cylinder_mesh.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;

        let index_buffer = cinder.device.create_buffer_with_data(
            &cylinder_mesh.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        Ok(Self {
            start_position: position,
            position,
            look_at,
            data_buffer,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn update(&mut self, angle: f32, aspect_ratio: f32) -> Result<()> {
        let p = Point2D::new(self.start_position.x(), self.start_position.z());
        let rotated_p = rotate_point(p, Point2D::zero(), angle);
        self.position = Vec3::new(rotated_p.x(), self.start_position.y(), rotated_p.y());
        self.data_buffer.mem_copy(0, &[self.position])?;

        self.data_buffer.mem_copy(
            0,
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(
                    self.position,
                    (self.look_at - self.position).normalized(),
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
        self.data_buffer.destroy(&cinder.device);
    }
}

struct CameraData {
    bind_group: BindGroup,
    transforms_buffer: Buffer,
}

impl CameraData {
    pub fn new(
        cinder: &Cinder,
        bind_group_data: &BindGroupData,
        pos: Vec3,
        front: Vec3,
        aspect_ratio: f32,
    ) -> Result<Self> {
        let bind_group = BindGroup::new(&cinder.device, bind_group_data)?;
        let transforms_buffer = cinder.device.create_buffer_with_data(
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(pos, front, Vec3::new(0.0, 1.0, 0.0)).into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 0.01).into(),
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::Uniform(transforms_buffer.bind_info()),
        }])?;

        Ok(Self {
            bind_group,
            transforms_buffer,
        })
    }
}

struct Pipelines {
    lit_mesh: GraphicsPipeline,
    light_caster: GraphicsPipeline,
    shadow_map_depth: GraphicsPipeline,
    shadow_map_quad: GraphicsPipeline,
}

pub struct HelloCube {
    cinder: Cinder,
    pipelines: Pipelines,
    sampler: Sampler,
    depth_image: Image,
    shadow_map_image: Image,
    eye_pos: Vec3,
    eye_camera: CameraData,
    light_data: LightData,
    light_camera: CameraData,
    texture_bind_group: BindGroup,
    quad_data: TexturedQuadData,
    cube_mesh_data: MeshData,
    plane_mesh_data: MeshData,
    show_shadow_map_image: bool,
}

impl HelloCube {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let (width, height) = window.drawable_size();
        let cinder = Cinder::new(window, width, height)?;

        //
        // Create Shaders and Pipelines
        //
        let light_vs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/light.vert.spv"),
            Default::default(),
        )?;
        let light_fs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/light.frag.spv"),
            Default::default(),
        )?;

        let lit_mesh_vs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/lit_mesh.vert.spv"),
            Default::default(),
        )?;
        let lit_mesh_fs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/lit_mesh.frag.spv"),
            Default::default(),
        )?;

        let shadow_map_vs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map.vert.spv"),
            Default::default(),
        )?;

        let shadow_map_quad_vs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map_quad.vert.spv"),
            Default::default(),
        )?;
        let shadow_map_quad_fs = cinder.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map_quad.frag.spv"),
            Default::default(),
        )?;

        let lit_mesh_pipeline = cinder.device.create_graphics_pipeline(
            &lit_mesh_vs,
            Some(&lit_mesh_fs),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                backface_culling: false,
                ..Default::default()
            },
        )?;

        let light_caster_pipeline = cinder.device.create_graphics_pipeline(
            &light_vs,
            Some(&light_fs),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let shadow_map_depth_pipeline = cinder.device.create_graphics_pipeline(
            &shadow_map_vs,
            None,
            GraphicsPipelineDescription {
                color_format: None,
                depth_format: Some(Format::D32_SFloat),
                backface_culling: false,
                ..Default::default()
            },
        )?;

        let shadow_map_quad_pipeline = cinder.device.create_graphics_pipeline(
            &shadow_map_quad_vs,
            Some(&shadow_map_quad_fs),
            Default::default(),
        )?;

        let pipelines = Pipelines {
            lit_mesh: lit_mesh_pipeline,
            light_caster: light_caster_pipeline,
            shadow_map_depth: shadow_map_depth_pipeline,
            shadow_map_quad: shadow_map_quad_pipeline,
        };

        //
        // Create Cameras
        //
        let surface_rect = cinder.device.surface_rect();
        let aspect_ratio = cinder.device.surface_aspect_ratio();

        let camera_bind_group_data = pipelines.lit_mesh.bind_group_data(0).unwrap();

        let eye_pos = Vec3::new(4.0, 4.0, 0.0);
        let eye_front = (Vec3::zero() - eye_pos).normalized();
        let eye_camera = CameraData::new(
            &cinder,
            camera_bind_group_data,
            eye_pos,
            eye_front,
            aspect_ratio,
        )?;

        let light_pos = Vec3::new(4.0, 2.0, 0.0);
        let light_look_at = Vec3::zero();
        let light_front = (light_look_at - light_pos).normalized();
        let light_camera = CameraData::new(
            &cinder,
            camera_bind_group_data,
            light_pos,
            light_front,
            aspect_ratio,
        )?;
        let light_data = LightData::new(&cinder, &light_camera, light_pos, light_look_at)?;
        //
        // Create Bind Groups
        //
        let texture_bind_group = BindGroup::new(
            &cinder.device,
            pipelines.shadow_map_quad.bind_group_data(0).unwrap(),
        )?;

        //
        // Create Images
        //
        let sampler = cinder.device.create_sampler(Default::default())?;

        let depth_image = cinder.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let shadow_map_image = cinder.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
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

        //
        // Create Meshes
        //

        let quad_data = TexturedQuadData::new(
            &cinder,
            &pipelines.shadow_map_quad,
            texture_bind_group,
            &shadow_map_image,
            &sampler,
        )?;

        let cube_mesh_data = MeshData::new(
            &cinder,
            &pipelines.lit_mesh,
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
            &pipelines.lit_mesh,
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

        //
        // Cleanup
        //

        lit_mesh_vs.destroy(&cinder.device);
        lit_mesh_fs.destroy(&cinder.device);
        light_vs.destroy(&cinder.device);
        light_fs.destroy(&cinder.device);
        shadow_map_vs.destroy(&cinder.device);
        shadow_map_quad_vs.destroy(&cinder.device);
        shadow_map_quad_fs.destroy(&cinder.device);

        Ok(Self {
            cinder,
            pipelines,
            sampler,
            depth_image,
            shadow_map_image,
            eye_pos,
            eye_camera,
            light_data,
            light_camera,
            texture_bind_group,
            quad_data,
            cube_mesh_data,
            plane_mesh_data,
            show_shadow_map_image: false,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let elapsed = self.cinder.init_time.elapsed().as_secs_f32();
        let scale = (elapsed / 5.0) * (2.0 * std::f32::consts::PI);

        self.cube_mesh_data
            .model_transform_buffer
            .mem_copy(0, &[Mat4::rotate(scale, Vec3::new(0.0, 1.0, 0.0))])?;

        self.light_data
            .update(elapsed, self.cinder.device.surface_aspect_ratio())?;

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
                    store_op: AttachmentStoreOp::Store,
                    layout: Layout::DepthAttachment,
                    clear_value: ClearValue::default_depth(),
                    ..Default::default()
                },
            )),
        );
        {
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.pipelines.shadow_map_depth,
                0,
                &[self.light_camera.bind_group],
            );
            cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.pipelines.shadow_map_depth);

            // Draw Cube
            cmd_list.bind_descriptor_sets(
                &self.cinder.device,
                &self.pipelines.shadow_map_depth,
                1,
                &[self.cube_mesh_data.model_bind_group],
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
                &self.pipelines.shadow_map_depth,
                1,
                &[self.plane_mesh_data.model_bind_group],
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
                &self.pipelines.lit_mesh,
                0,
                &[self.eye_camera.bind_group],
            );
            cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.pipelines.lit_mesh);

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
                &self.pipelines.lit_mesh,
                1,
                &[self.cube_mesh_data.model_bind_group],
            );
            cmd_list.bind_index_buffer(&self.cinder.device, &self.cube_mesh_data.index_buffer);
            cmd_list.bind_vertex_buffer(&self.cinder.device, &self.cube_mesh_data.vertex_buffer);
            cmd_list.set_vertex_bytes(
                &self.cinder.device,
                &self.pipelines.lit_mesh,
                &[LitMeshConstants {
                    color: [161.0 / 255.0, 29.0 / 255.0, 194.0 / 255.0, 0.0],
                    view_from: [self.eye_pos.x(), self.eye_pos.y(), self.eye_pos.z(), 0.0],
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
                &self.pipelines.lit_mesh,
                1,
                &[self.plane_mesh_data.model_bind_group],
            );
            cmd_list.bind_index_buffer(&self.cinder.device, &self.plane_mesh_data.index_buffer);
            cmd_list.bind_vertex_buffer(&self.cinder.device, &self.plane_mesh_data.vertex_buffer);
            cmd_list.set_vertex_bytes(
                &self.cinder.device,
                &self.pipelines.lit_mesh,
                &[LitMeshConstants {
                    color: [201.0 / 255.0, 114.0 / 255.0, 38.0 / 255.0, 0.0],
                    view_from: [self.eye_pos.x(), self.eye_pos.y(), self.eye_pos.z(), 0.0],
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
            cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.pipelines.light_caster);
            cmd_list.bind_index_buffer(&self.cinder.device, &self.light_data.index_buffer);
            cmd_list.bind_vertex_buffer(&self.cinder.device, &self.light_data.vertex_buffer);
            cmd_list.set_vertex_bytes(
                &self.cinder.device,
                &self.pipelines.light_caster,
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

        if self.show_shadow_map_image {
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
                cmd_list
                    .bind_graphics_pipeline(&self.cinder.device, &self.pipelines.shadow_map_quad);
                cmd_list.bind_descriptor_sets(
                    &self.cinder.device,
                    &self.pipelines.shadow_map_quad,
                    0,
                    &[self.texture_bind_group],
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
        }

        self.cinder
            .swapchain
            .present(&self.cinder.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)?;
        self.depth_image
            .resize(&self.cinder.device, Size2D::new(width, height))?;
        self.shadow_map_image
            .resize(&self.cinder.device, Size2D::new(width, height))?;
        self.cinder.command_queue.transition_image(
            &self.cinder.device,
            &self.shadow_map_image,
            // TODO: get rid of `vk`
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        )?;
        self.cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: self.texture_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(self.shadow_map_image.bind_info(
                &self.sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;
        Ok(())
    }
}

impl Drop for HelloCube {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
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
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    renderer.show_shadow_map_image = !renderer.show_shadow_map_image;
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
