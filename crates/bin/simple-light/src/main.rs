use anyhow::Result;
use cinder::{
    AddressMode, App, AttachmentLoadOp, AttachmentStoreOp, AttachmentType, BindGroup,
    BindGroupBindInfo, BindGroupData, BindGroupWriteData, BorderColor, Buffer, BufferDescription,
    BufferUsage, Bump, Cinder, ClearValue, Format, GraphicsPipeline, GraphicsPipelineDescription,
    Image, ImageDescription, ImageUsage, Layout, MipmapMode, RenderAttachmentDesc, RenderGraph,
    RenderPass, RenderPassResource, Renderer, ResourceId, Sampler, SamplerDescription,
    VertexAttributeDescription, VertexBindingDesc, VertexDescription, VertexInputRate,
};
use math::{mat::Mat4, point::Point2D, size::Size2D, vec::Vec3};

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
        renderer: &Renderer,
        bind_group: BindGroup,
        image: &Image,
        sampler: &Sampler,
    ) -> Result<Self> {
        let vertex_buffer = renderer.device.create_buffer_with_data(
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
        let index_buffer = renderer.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        renderer.device.write_bind_group(&[BindGroupBindInfo {
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

    pub fn cleanup(&self, renderer: &Renderer) {
        self.index_buffer.destroy(&renderer.device);
        self.vertex_buffer.destroy(&renderer.device);
    }
}

struct MeshData {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    model_bind_group: BindGroup,
    shadow_texture_bind_group: BindGroup,
    model_transform_buffer: Buffer,
}

impl MeshData {
    pub fn new<T: Copy>(
        renderer: &Renderer,
        pipeline: &GraphicsPipeline,
        shadow_texture: &Image,
        sampler: &Sampler,
        vertex_buffer_data: &[T],
        index_buffer_data: &[u32],
    ) -> Result<Self> {
        let vertex_buffer = renderer.device.create_buffer_with_data(
            vertex_buffer_data,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = renderer.device.create_buffer_with_data(
            index_buffer_data,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let model_bind_group =
            BindGroup::new(&renderer.device, pipeline.bind_group_data(1).unwrap())?;
        let shadow_texture_bind_group =
            BindGroup::new(&renderer.device, pipeline.bind_group_data(2).unwrap())?;

        let model_transform_buffer = renderer.device.create_buffer(
            std::mem::size_of::<LitMeshModelUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        model_transform_buffer.mem_copy(0, &[Mat4::identity()])?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: model_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::Uniform(model_transform_buffer.bind_info()),
        }])?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: shadow_texture_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(shadow_texture.bind_info(
                sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            model_bind_group,
            shadow_texture_bind_group,
            model_transform_buffer,
        })
    }

    pub fn resize(
        &self,
        renderer: &Renderer,
        shadow_texture: &Image,
        sampler: &Sampler,
    ) -> Result<()> {
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: self.shadow_texture_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(shadow_texture.bind_info(
                sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;
        Ok(())
    }

    pub fn cleanup(&self, renderer: &Renderer) {
        self.index_buffer.destroy(&renderer.device);
        self.vertex_buffer.destroy(&renderer.device);
        self.model_transform_buffer.destroy(&renderer.device);
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

struct FlashligthMesh {
    // TODO: Could consolidate buffers
    cylinder_vb: Buffer,
    cylinder_ib: Buffer,
    cone_vb: Buffer,
    cone_ib: Buffer,
}

impl FlashligthMesh {
    fn new(renderer: &Renderer) -> Result<Self> {
        let cylinder = geometry::SurfaceMesh::cylinder::<30>(0.3, 0.05);
        let cylinder_vb = renderer.device.create_buffer_with_data(
            &cylinder.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let cylinder_ib = renderer.device.create_buffer_with_data(
            &cylinder.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let cone = geometry::SurfaceMesh::cone::<30>(0.125, 0.1);
        let cone_vb = renderer.device.create_buffer_with_data(
            &cone.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let cone_ib = renderer.device.create_buffer_with_data(
            &cone.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        Ok(Self {
            cylinder_vb,
            cylinder_ib,
            cone_vb,
            cone_ib,
        })
    }

    fn cleanup(&self, renderer: &Renderer) {
        self.cylinder_vb.destroy(&renderer.device);
        self.cylinder_ib.destroy(&renderer.device);

        self.cone_vb.destroy(&renderer.device);
        self.cone_ib.destroy(&renderer.device);
    }
}

pub struct LightData {
    start_position: Vec3,
    position: Vec3,
    look_at: Vec3,
    data_buffer: Buffer,
    flashlight: FlashligthMesh,
}

impl LightData {
    fn new(renderer: &Renderer, position: Vec3, look_at: Vec3, aspect_ratio: f32) -> Result<Self> {
        let data_buffer = renderer.device.create_buffer_with_data(
            &[LitMeshGlobalLightData {
                view: camera::look_to(
                    position,
                    (look_at - position).normalized(),
                    Vec3::new(0.0, 1.0, 0.0),
                )
                .into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 1.0).into(),
                position: [position.x(), position.y(), position.z(), 1.0],
                look_at: [look_at.x(), look_at.y(), look_at.z(), 1.0],
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;

        let flashlight = FlashligthMesh::new(renderer)?;

        Ok(Self {
            start_position: position,
            position,
            look_at,
            data_buffer,
            flashlight,
        })
    }

    pub fn update(&mut self, elapsed: f32, aspect_ratio: f32) -> Result<()> {
        let angle = (elapsed / 5.0) * (2.0 * std::f32::consts::PI);

        let p = Point2D::new(self.start_position.x(), self.start_position.z());
        let rotated_p = rotate_point(p, Point2D::zero(), angle);
        self.position = Vec3::new(rotated_p.x(), self.start_position.y(), rotated_p.y());

        self.data_buffer.mem_copy(
            0,
            &[LitMeshGlobalLightData {
                view: camera::look_to(
                    self.position,
                    (self.look_at - self.position).normalized(),
                    Vec3::new(0.0, 1.0, 0.0),
                )
                .into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 1.0).into(),
                position: [self.position.x(), self.position.y(), self.position.z(), 1.0],
                look_at: [self.look_at.x(), self.look_at.y(), self.look_at.z(), 1.0],
            }],
        )?;

        Ok(())
    }

    pub fn cleanup(&self, renderer: &Renderer) {
        self.flashlight.cleanup(renderer);
        self.data_buffer.destroy(&renderer.device);
    }
}

struct CameraData {
    bind_group: BindGroup,
    transforms_buffer: Buffer,
}

impl CameraData {
    pub fn new(
        renderer: &Renderer,
        bind_group_data: &BindGroupData,
        pos: Vec3,
        front: Vec3,
        aspect_ratio: f32,
        light_data: Option<&LightData>,
    ) -> Result<Self> {
        let bind_group = BindGroup::new(&renderer.device, bind_group_data)?;
        let transforms_buffer = renderer.device.create_buffer_with_data(
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(pos, front, Vec3::new(0.0, 1.0, 0.0)).into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 1.0).into(),
            }],
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::Uniform(transforms_buffer.bind_info()),
        }])?;

        if let Some(light_data) = light_data {
            renderer.device.write_bind_group(&[BindGroupBindInfo {
                group: bind_group,
                dst_binding: 1,
                data: BindGroupWriteData::Uniform(light_data.data_buffer.bind_info()),
            }])?;
        }

        Ok(Self {
            bind_group,
            transforms_buffer,
        })
    }

    pub fn cleanup(&self, renderer: &Renderer) {
        self.transforms_buffer.destroy(&renderer.device);
    }
}

struct Pipelines {
    lit_mesh: GraphicsPipeline,
    light_caster: GraphicsPipeline,
    shadow_map_depth: GraphicsPipeline,
    shadow_map_quad: GraphicsPipeline,
}

impl Pipelines {
    pub fn cleanup(&self, renderer: &Renderer) {
        self.lit_mesh.destroy(&renderer.device);
        self.light_caster.destroy(&renderer.device);
        self.shadow_map_depth.destroy(&renderer.device);
        self.shadow_map_quad.destroy(&renderer.device);
    }
}
pub struct SimpleLightSample {
    pipelines: Pipelines,
    sampler: Sampler,
    shadow_map_sampler: Sampler,
    depth_image_handle: ResourceId<Image>,
    shadow_map_image_handle: ResourceId<Image>,
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

impl App for SimpleLightSample {
    fn new(renderer: &mut Renderer, _width: u32, _height: u32) -> Result<Self> {
        //
        // Create Shaders and Pipelines
        //
        let light_vs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/light.vert.spv"),
            Default::default(),
        )?;
        let light_fs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/light.frag.spv"),
            Default::default(),
        )?;

        let lit_mesh_vs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/lit_mesh.vert.spv"),
            Default::default(),
        )?;
        let lit_mesh_fs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/lit_mesh.frag.spv"),
            Default::default(),
        )?;

        let shadow_map_vs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map.vert.spv"),
            Default::default(),
        )?;

        let shadow_map_quad_vs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map_quad.vert.spv"),
            Default::default(),
        )?;
        let shadow_map_quad_fs = renderer.device.create_shader(
            include_bytes!("../shaders/spv/shadow_map_quad.frag.spv"),
            Default::default(),
        )?;

        let lit_mesh_pipeline = renderer.device.create_graphics_pipeline(
            &lit_mesh_vs,
            Some(&lit_mesh_fs),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;

        let light_caster_pipeline = renderer.device.create_graphics_pipeline(
            &light_vs,
            Some(&light_fs),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;

        let shadow_map_depth_pipeline = renderer.device.create_graphics_pipeline(
            &shadow_map_vs,
            None,
            GraphicsPipelineDescription {
                color_format: None,
                depth_format: Some(Format::D32_SFLOAT),
                vertex_desc: Some(VertexDescription {
                    binding_desc: vec![VertexBindingDesc {
                        binding: 0,
                        stride: std::mem::size_of::<LitMeshVertex>() as u32,
                        input_rate: VertexInputRate::VERTEX,
                    }],
                    attribute_desc: vec![VertexAttributeDescription {
                        location: 0,
                        binding: 0,
                        format: Format::R32G32B32_SFLOAT.into(),
                        offset: 0,
                    }],
                }),
                ..Default::default()
            },
        )?;

        let shadow_map_quad_pipeline = renderer.device.create_graphics_pipeline(
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
        let surface_rect = renderer.device.surface_rect();
        let aspect_ratio = renderer.device.surface_aspect_ratio();

        let light_pos = Vec3::new(3.0, 2.0, 0.0);
        let light_look_at = Vec3::zero();
        let light_front = (light_look_at - light_pos).normalized();
        let light_camera = CameraData::new(
            &renderer,
            pipelines.shadow_map_depth.bind_group_data(0).unwrap(),
            light_pos,
            light_front,
            aspect_ratio,
            None,
        )?;
        let light_data = LightData::new(&renderer, light_pos, light_look_at, aspect_ratio)?;

        let eye_pos = Vec3::new(4.0, 4.0, 0.0);
        let eye_front = (Vec3::zero() - eye_pos).normalized();
        let eye_camera = CameraData::new(
            &renderer,
            pipelines.lit_mesh.bind_group_data(0).unwrap(),
            eye_pos,
            eye_front,
            aspect_ratio,
            Some(&light_data),
        )?;

        //
        // Create Bind Groups
        //
        let texture_bind_group = BindGroup::new(
            &renderer.device,
            pipelines.shadow_map_quad.bind_group_data(0).unwrap(),
        )?;

        //
        // Create Images
        //
        let sampler = renderer.device.create_sampler(Default::default())?;
        let shadow_map_sampler = renderer.device.create_sampler(SamplerDescription {
            address_mode: AddressMode::ClampToEdge,
            mipmap_mode: MipmapMode::Nearest,
            border_color: BorderColor::White,
            ..Default::default()
        })?;

        let depth_image = renderer.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let shadow_map_image = renderer.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::DepthSampled,
                ..Default::default()
            },
        )?;
        renderer.command_queue.transition_image(
            &renderer.device,
            &shadow_map_image,
            ImageUsage::Depth,
            Layout::Undefined,
            Layout::DepthStencilReadOnly,
        )?;

        //
        // Create Meshes
        //

        let quad_data =
            TexturedQuadData::new(&renderer, texture_bind_group, &shadow_map_image, &sampler)?;

        let cube_mesh_data = MeshData::new(
            &renderer,
            &pipelines.lit_mesh,
            &shadow_map_image,
            &shadow_map_sampler,
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
            &renderer,
            &pipelines.lit_mesh,
            &shadow_map_image,
            &shadow_map_sampler,
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
        lit_mesh_vs.destroy(&renderer.device);
        lit_mesh_fs.destroy(&renderer.device);
        light_vs.destroy(&renderer.device);
        light_fs.destroy(&renderer.device);
        shadow_map_vs.destroy(&renderer.device);
        shadow_map_quad_vs.destroy(&renderer.device);
        shadow_map_quad_fs.destroy(&renderer.device);

        let depth_image_handle = renderer.resource_manager.insert_image(depth_image);
        let shadow_map_image_handle = renderer.resource_manager.insert_image(shadow_map_image);

        Ok(Self {
            pipelines,
            sampler,
            shadow_map_sampler,
            depth_image_handle,
            shadow_map_image_handle,
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

    fn update(&mut self, renderer: &mut Renderer) -> Result<()> {
        let elapsed = renderer.init_time().elapsed().as_secs_f32();
        let scale = (elapsed / 2.5) * (2.0 * std::f32::consts::PI);

        self.cube_mesh_data
            .model_transform_buffer
            .mem_copy(0, &[Mat4::rotate(scale, Vec3::new(0.0, 1.0, 0.0))])?;

        let aspect_ratio = renderer.device.surface_aspect_ratio();
        self.light_data.update(elapsed, aspect_ratio)?;
        self.light_camera.transforms_buffer.mem_copy(
            0,
            &[LitMeshCameraUniformBufferObject {
                view: camera::look_to(
                    self.light_data.position,
                    (self.light_data.look_at - self.light_data.position).normalized(),
                    Vec3::new(0.0, 1.0, 0.0),
                )
                .into(),
                proj: camera::new_infinite_perspective_proj(aspect_ratio, 30.0, 1.0).into(),
            }],
        )?;

        Ok(())
    }

    fn draw<'a>(
        &'a mut self,
        allocator: &'a Bump,
        graph: &mut RenderGraph<'a>,
    ) -> anyhow::Result<()> {
        graph.add_pass(
            &allocator,
            RenderPass::new(&allocator)
                .set_depth_attachment(
                    AttachmentType::Reference(self.shadow_map_image_handle),
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::Store,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )
                .add_output(RenderPassResource::Image(self.shadow_map_image_handle))
                .with_flipped_viewport(false)
                .set_callback(&allocator, |renderer, cmd_list| {
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.shadow_map_depth,
                        0,
                        &[self.light_camera.bind_group],
                    );
                    cmd_list
                        .bind_graphics_pipeline(&renderer.device, &self.pipelines.shadow_map_depth);

                    // Draw Cube
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.shadow_map_depth,
                        1,
                        &[self.cube_mesh_data.model_bind_group],
                    );
                    cmd_list.bind_index_buffer(&renderer.device, &self.cube_mesh_data.index_buffer);
                    cmd_list
                        .bind_vertex_buffer(&renderer.device, &self.cube_mesh_data.vertex_buffer);
                    cmd_list.draw_offset(
                        &renderer.device,
                        self.cube_mesh_data.index_buffer.num_elements().unwrap(),
                        0,
                        0,
                    );

                    // Draw Plane
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.shadow_map_depth,
                        1,
                        &[self.plane_mesh_data.model_bind_group],
                    );
                    cmd_list
                        .bind_index_buffer(&renderer.device, &self.plane_mesh_data.index_buffer);
                    cmd_list
                        .bind_vertex_buffer(&renderer.device, &self.plane_mesh_data.vertex_buffer);
                    cmd_list.draw_offset(
                        &renderer.device,
                        self.plane_mesh_data.index_buffer.num_elements().unwrap(),
                        0,
                        0,
                    );

                    Ok(())
                }),
        );

        graph.add_pass(
            &allocator,
            RenderPass::new(&allocator)
                .add_color_attachment(
                    AttachmentType::SwapchainImage,
                    RenderAttachmentDesc {
                        clear_value: ClearValue::Color {
                            color: [0.4, 0.4, 0.4, 1.0],
                        },
                        ..Default::default()
                    },
                )
                .set_depth_attachment(
                    AttachmentType::Reference(self.depth_image_handle),
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::DontCare,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )
                .add_input(RenderPassResource::Image(self.shadow_map_image_handle))
                .add_output(RenderPassResource::SwapchainImage)
                .with_flipped_viewport(false)
                .set_callback(allocator, |renderer, cmd_list| {
                    // Bind Mesh Data
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.lit_mesh,
                        0,
                        &[self.eye_camera.bind_group],
                    );
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.pipelines.lit_mesh);

                    let scale = (renderer.init_time().elapsed().as_secs_f32() / 5.0)
                        * (2.0 * std::f32::consts::PI);
                    let light_color = [
                        (scale.sin() + 1.0) / 2.0,
                        (scale.cos() + 1.0) / 2.0,
                        ((scale * 1.5).cos() + 1.0) / 2.0,
                    ];

                    // Draw Cube
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.lit_mesh,
                        1,
                        &[
                            self.cube_mesh_data.model_bind_group,
                            self.cube_mesh_data.shadow_texture_bind_group,
                        ],
                    );
                    cmd_list.bind_index_buffer(&renderer.device, &self.cube_mesh_data.index_buffer);
                    cmd_list
                        .bind_vertex_buffer(&renderer.device, &self.cube_mesh_data.vertex_buffer);
                    cmd_list.set_vertex_bytes(
                        &renderer.device,
                        &self.pipelines.lit_mesh,
                        &[LitMeshConstants {
                            color: [161.0 / 255.0, 29.0 / 255.0, 194.0 / 255.0, 0.0],
                            view_from: [self.eye_pos.x(), self.eye_pos.y(), self.eye_pos.z(), 0.0],
                            light_color,
                        }],
                        0,
                    )?;
                    cmd_list.draw_offset(
                        &renderer.device,
                        self.cube_mesh_data.index_buffer.num_elements().unwrap(),
                        0,
                        0,
                    );

                    // Draw Plane
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.lit_mesh,
                        1,
                        &[
                            self.plane_mesh_data.model_bind_group,
                            self.plane_mesh_data.shadow_texture_bind_group,
                        ],
                    );

                    cmd_list
                        .bind_index_buffer(&renderer.device, &self.plane_mesh_data.index_buffer);
                    cmd_list
                        .bind_vertex_buffer(&renderer.device, &self.plane_mesh_data.vertex_buffer);
                    cmd_list.set_vertex_bytes(
                        &renderer.device,
                        &self.pipelines.lit_mesh,
                        &[LitMeshConstants {
                            color: [201.0 / 255.0, 114.0 / 255.0, 38.0 / 255.0, 0.0],
                            view_from: [self.eye_pos.x(), self.eye_pos.y(), self.eye_pos.z(), 0.0],
                            light_color,
                        }],
                        0,
                    )?;
                    cmd_list.draw_offset(
                        &renderer.device,
                        self.plane_mesh_data.index_buffer.num_elements().unwrap(),
                        0,
                        0,
                    );

                    // Draw Light
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipelines.light_caster,
                        0,
                        &[self.eye_camera.bind_group],
                    );
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.pipelines.light_caster);
                    cmd_list.set_vertex_bytes(
                        &renderer.device,
                        &self.pipelines.light_caster,
                        &light_color,
                        0,
                    )?;

                    cmd_list.bind_index_buffer(
                        &renderer.device,
                        &self.light_data.flashlight.cylinder_ib,
                    );
                    cmd_list.bind_vertex_buffer(
                        &renderer.device,
                        &self.light_data.flashlight.cylinder_vb,
                    );
                    cmd_list.draw_offset(
                        &renderer.device,
                        self.light_data
                            .flashlight
                            .cylinder_ib
                            .num_elements()
                            .unwrap(),
                        0,
                        0,
                    );

                    cmd_list
                        .bind_index_buffer(&renderer.device, &self.light_data.flashlight.cone_ib);
                    cmd_list
                        .bind_vertex_buffer(&renderer.device, &self.light_data.flashlight.cone_vb);
                    cmd_list.draw_offset(
                        &renderer.device,
                        self.light_data.flashlight.cone_ib.num_elements().unwrap(),
                        0,
                        0,
                    );

                    Ok(())
                }),
        );

        if self.show_shadow_map_image {
            graph.add_pass(
                &allocator,
                RenderPass::new(&allocator)
                    .add_color_attachment(
                        AttachmentType::SwapchainImage,
                        RenderAttachmentDesc {
                            load_op: AttachmentLoadOp::Load,
                            ..Default::default()
                        },
                    )
                    .add_input(RenderPassResource::Image(self.shadow_map_image_handle))
                    .add_input(RenderPassResource::SwapchainImage)
                    .with_flipped_viewport(false)
                    .set_callback(allocator, |renderer, cmd_list| {
                        cmd_list.bind_graphics_pipeline(
                            &renderer.device,
                            &self.pipelines.shadow_map_quad,
                        );
                        cmd_list.bind_descriptor_sets(
                            &renderer.device,
                            &self.pipelines.shadow_map_quad,
                            0,
                            &[self.texture_bind_group],
                        );
                        cmd_list.bind_index_buffer(&renderer.device, &self.quad_data.index_buffer);
                        cmd_list
                            .bind_vertex_buffer(&renderer.device, &self.quad_data.vertex_buffer);
                        cmd_list.draw_offset(
                            &renderer.device,
                            self.quad_data.index_buffer.num_elements().unwrap(),
                            0,
                            0,
                        );

                        Ok(())
                    }),
            );
        }
        Ok(())
    }

    fn resize(&mut self, renderer: &mut Renderer, width: u32, height: u32) -> Result<()> {
        renderer
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap()
            .resize(&renderer.device, Size2D::new(width, height))?;
        renderer
            .resource_manager
            .images
            .get_mut(self.shadow_map_image_handle)
            .unwrap()
            .resize(&renderer.device, Size2D::new(width, height))?;

        let shadow_map_image = renderer
            .resource_manager
            .images
            .get(self.shadow_map_image_handle)
            .unwrap();
        renderer.command_queue.transition_image(
            &renderer.device,
            shadow_map_image,
            ImageUsage::Depth,
            Layout::Undefined,
            Layout::DepthStencilReadOnly,
        )?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: self.texture_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(shadow_map_image.bind_info(
                &self.sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;

        self.cube_mesh_data
            .resize(&renderer, shadow_map_image, &self.shadow_map_sampler)?;
        self.plane_mesh_data
            .resize(&renderer, shadow_map_image, &self.shadow_map_sampler)?;
        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.cube_mesh_data.cleanup(&renderer);
        self.plane_mesh_data.cleanup(&renderer);
        self.light_data.cleanup(&renderer);
        self.quad_data.cleanup(&renderer);
        self.eye_camera.cleanup(&renderer);
        self.light_camera.cleanup(&renderer);
        self.sampler.destroy(&renderer.device);
        self.shadow_map_sampler.destroy(&renderer.device);
        self.pipelines.cleanup(&renderer);
        Ok(())
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
    let mut cinder = Cinder::<SimpleLightSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
