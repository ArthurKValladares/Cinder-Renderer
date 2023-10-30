use anyhow::Result;
use camera::{
    input::{KeyboardState, MouseState},
    Camera, CameraDescription,
};
use cinder::{
    App, AttachmentStoreOp, AttachmentType, BindGroup, BindGroupBindInfo, BindGroupWriteData,
    Buffer, BufferDescription, BufferUsage, Bump, Cinder, ClearValue, Format, GraphicsPipeline,
    GraphicsPipelineDescription, Image, ImageDescription, ImageUsage, InitContext, Layout,
    RenderAttachmentDesc, RenderGraph, RenderPass, Renderer, ResourceId,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use rayon::iter::*;
use scene::{ObjMesh, Scene, Vertex};
use sdl2::event::Event;
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
            mesh.positions[i * 3 + 1],
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
    image_index: Option<u32>,
}

pub struct BindlessSample {
    camera: Camera,
    keyboard_state: KeyboardState,
    mouse_state: MouseState,
    mesh_draws: Vec<MeshDraw>,
    depth_image_handle: ResourceId<Image>,
    pipeline: GraphicsPipeline,
    bind_group: BindGroup,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl App for BindlessSample {
    fn new(context: InitContext<'_>) -> Result<Self> {
        //
        // Create App Resources
        //
        let surface_rect = context.renderer.device.surface_rect();
        let depth_image = context.renderer.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let vertex_shader = context.renderer.device.create_shader(
            include_bytes!("../shaders/spv/bindless.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = context.renderer.device.create_shader(
            include_bytes!("../shaders/spv/bindless.frag.spv"),
            Default::default(),
        )?;
        let pipeline = context.renderer.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(
            &context.renderer.device,
            pipeline.bind_group_data(0).unwrap(),
        )?;

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
        println!("Scene creation: {:?}ms", init_time.elapsed().as_millis());

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
                    image_index: mesh.material_index,
                });
            }
            (vertices, indices, mesh_draws)
        };
        let camera = Camera::new(
            Vec3::new(0.0, 50.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            CameraDescription {
                movement_per_sec: 200.0,
                ..Default::default()
            },
        );
        let ubo_buffer = context.renderer.device.create_buffer(
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
                camera.view(),
                camera.projection(surface_rect.width() as f32, surface_rect.height() as f32),
            ],
        )?;
        let index_buffer = context.renderer.device.create_buffer_with_data(
            &indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;
        let vertex_buffer = context.renderer.device.create_buffer_with_data(
            &vertices,
            BufferDescription {
                usage: BufferUsage::STORAGE | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
        )?;
        context.renderer.device.write_bind_group(&[
            BindGroupBindInfo {
                group: bind_group,
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
            },
            BindGroupBindInfo {
                group: bind_group,
                dst_binding: 1,
                data: BindGroupWriteData::Storage(vertex_buffer.bind_info()),
            },
        ])?;

        let image_data = scene
            .materials
            .par_iter()
            .enumerate()
            .filter(|(_, material)| material.diffuse.is_some())
            .map(|(idx, material)| (idx, material.diffuse.as_ref().unwrap()))
            .collect::<Vec<_>>();

        let sampler = context.renderer.device.create_sampler(Default::default())?;
        let images = image_data
            .into_iter()
            .map(|(idx, image_data)| {
                let texture = context
                    .renderer
                    .device
                    .create_image_with_data_immediate(
                        Size2D::new(image_data.width, image_data.height),
                        &image_data.bytes,
                        &context.renderer.command_queue,
                        Default::default(),
                    )
                    .unwrap();

                context
                    .renderer
                    .device
                    .write_bind_group(&[BindGroupBindInfo {
                        group: bind_group,
                        dst_binding: 2,
                        data: BindGroupWriteData::SampledImage(texture.bind_info(
                            &sampler,
                            Layout::ShaderReadOnly,
                            Some(idx as u32),
                        )),
                    }])
                    .unwrap();

                texture
            })
            .collect::<Vec<_>>();

        //
        // Add resources to ResourceManager
        //
        for image in images {
            context.renderer.resource_manager.insert_image(image);
        }
        context
            .renderer
            .resource_manager
            .insert_buffer(vertex_buffer);
        context.renderer.resource_manager.insert_sampler(sampler);

        //
        // Cleanup
        //
        vertex_shader.destroy(&context.renderer.device);
        fragment_shader.destroy(&context.renderer.device);

        let depth_image_handle = context.renderer.resource_manager.insert_image(depth_image);

        Ok(Self {
            camera,
            keyboard_state: Default::default(),
            mouse_state: Default::default(),
            mesh_draws,
            depth_image_handle,
            pipeline,
            bind_group,
            index_buffer,
            ubo_buffer,
        })
    }

    fn on_frame_start(&mut self) -> anyhow::Result<()> {
        self.mouse_state.reset_delta();
        Ok(())
    }

    fn on_event(&mut self, event: &Event) -> anyhow::Result<()> {
        self.keyboard_state.on_event(&event);
        self.mouse_state.on_event(&event);
        Ok(())
    }

    fn update(&mut self, renderer: &mut Renderer) -> Result<()> {
        let surface_rect = renderer.device.surface_rect();
        self.camera.update(
            &self.keyboard_state,
            &self.mouse_state,
            surface_rect.width(),
            surface_rect.height(),
            renderer.last_dt(),
        );
        self.ubo_buffer.mem_copy(
            util::offset_of!(BindlessUniformBufferObject, view) as u64,
            &[
                self.camera.view(),
                self.camera
                    .projection(surface_rect.width() as f32, surface_rect.height() as f32),
            ],
        )?;
        Ok(())
    }

    fn draw<'a>(
        &'a mut self,
        allocator: &'a Bump,
        graph: &mut RenderGraph<'a>,
    ) -> anyhow::Result<()> {
        graph.add_pass(
            allocator,
            RenderPass::new(allocator)
                .add_color_attachment(AttachmentType::SwapchainImage, Default::default())
                .set_depth_attachment(
                    AttachmentType::Reference(self.depth_image_handle),
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::DontCare,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )
                .set_callback(allocator, |renderer, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.pipeline);
                    cmd_list.bind_index_buffer(&renderer.device, &self.index_buffer);
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipeline,
                        0,
                        &[self.bind_group],
                    );
                    for mesh_draw in &self.mesh_draws {
                        if let Some(index) = mesh_draw.image_index {
                            cmd_list.set_fragment_bytes(
                                &renderer.device,
                                &self.pipeline,
                                &[index],
                                0,
                            )?;
                        }
                        cmd_list.draw_offset(
                            &renderer.device,
                            mesh_draw.num_indices,
                            mesh_draw.index_buffer_offset,
                            mesh_draw.vertex_buffer_offset,
                        );
                    }
                    Ok(())
                }),
        );
        Ok(())
    }

    fn resize(&mut self, renderer: &mut Renderer, width: u32, height: u32) -> Result<()> {
        let depth_image = renderer
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&renderer.device, Size2D::new(width, height))?;
        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.pipeline.destroy(&renderer.device);
        self.index_buffer.destroy(&renderer.device);
        self.ubo_buffer.destroy(&renderer.device);
        Ok(())
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "bindless",
            capture_mouse: true,
        },
    )
    .unwrap();
    let mut cinder = Cinder::<BindlessSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
