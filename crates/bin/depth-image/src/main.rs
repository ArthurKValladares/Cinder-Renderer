use anyhow::Result;
use bumpalo::Bump;
use cinder::{
    App, AttachmentLoadOp, AttachmentStoreOp, BindGroup, BindGroupBindInfo, BindGroupWriteData,
    Buffer, BufferDescription, BufferUsage, Cinder, ClearValue, Format, GraphicsPipeline,
    GraphicsPipelineDescription, Image, ImageDescription, ImageUsage, Layout, RenderAttachmentDesc,
    Renderer, ResourceId, Sampler,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use render_graph::{AttachmentType, RenderGraph, RenderPass, RenderPassResource};

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

pub struct DepthImageSample {
    depth_image_handle: ResourceId<Image>,
    mesh_pipeline: GraphicsPipeline,
    mesh_bind_group: BindGroup,
    texture_pipeline: GraphicsPipeline,
    texture_bind_group: BindGroup,
    cube_vertex_buffer: Buffer,
    cube_index_buffer: Buffer,
    ubo_buffer: Buffer,
    quad_vertex_buffer: Buffer,
    quad_index_buffer: Buffer,
    sampler: Sampler,
}

impl App for DepthImageSample {
    fn new(renderer: &mut Renderer, _width: u32, _height: u32) -> Result<Self> {
        //
        // Create App Resources
        //
        let surface_rect = renderer.device.surface_rect();
        let depth_image = renderer.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::DepthSampled,
                ..Default::default()
            },
        )?;
        renderer.command_queue.transition_image(
            &renderer.device,
            &depth_image,
            ImageUsage::Depth,
            Layout::Undefined,
            Layout::DepthStencilReadOnly,
        )?;
        let mesh_vertex_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.vert.spv"),
            Default::default(),
        )?;
        let mesh_fragment_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/depth_mesh.frag.spv"),
            Default::default(),
        )?;
        let mesh_pipeline = renderer.device.create_graphics_pipeline(
            &mesh_vertex_shader,
            Some(&mesh_fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;
        let mesh_bind_group =
            BindGroup::new(&renderer.device, mesh_pipeline.bind_group_data(0).unwrap())?;

        let texture_vertex_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.vert.spv"),
            Default::default(),
        )?;
        let texture_fragment_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/depth_texture.frag.spv"),
            Default::default(),
        )?;
        let texture_pipeline = renderer.device.create_graphics_pipeline(
            &texture_vertex_shader,
            Some(&texture_fragment_shader),
            Default::default(),
        )?;
        let texture_bind_group = BindGroup::new(
            &renderer.device,
            texture_pipeline.bind_group_data(0).unwrap(),
        )?;

        let cube_vertex_buffer = renderer.device.create_buffer_with_data(
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
        let cube_index_buffer = renderer.device.create_buffer_with_data(
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

        let ubo_buffer = renderer.device.create_buffer(
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
            renderer.device.write_bind_group(&[BindGroupBindInfo {
                dst_binding: 0,
                group: mesh_bind_group,
                data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
            }])?;
        }
        let quad_vertex_buffer = renderer.device.create_buffer_with_data(
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
        let quad_index_buffer = renderer.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let sampler = renderer.device.create_sampler(Default::default())?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: texture_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(depth_image.bind_info(
                &sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;

        //
        // Cleanup
        //
        texture_vertex_shader.destroy(&renderer.device);
        texture_fragment_shader.destroy(&renderer.device);
        mesh_vertex_shader.destroy(&renderer.device);
        mesh_fragment_shader.destroy(&renderer.device);

        let depth_image_handle = renderer.resource_manager.insert_image(depth_image);

        Ok(Self {
            depth_image_handle,
            mesh_pipeline,
            mesh_bind_group,
            texture_pipeline,
            texture_bind_group,
            cube_vertex_buffer,
            cube_index_buffer,
            ubo_buffer,
            quad_vertex_buffer,
            quad_index_buffer,
            sampler,
        })
    }

    fn update(&mut self, renderer: &mut Renderer) -> Result<()> {
        let scale =
            (renderer.init_time().elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        self.ubo_buffer.mem_copy(
            util::offset_of!(DepthMeshUniformBufferObject, model) as u64,
            &[Mat4::rotate(scale, Vec3::new(1.0, 1.0, 0.0))],
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
                        store_op: AttachmentStoreOp::Store,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )
                .add_output(RenderPassResource::Image(self.depth_image_handle))
                .set_callback(allocator, |renderer, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.mesh_pipeline);
                    cmd_list.bind_index_buffer(&renderer.device, &self.cube_index_buffer);
                    cmd_list.bind_vertex_buffer(&renderer.device, &self.cube_vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.mesh_pipeline,
                        0,
                        &[self.mesh_bind_group],
                    );
                    cmd_list.draw_offset(&renderer.device, 36, 0, 0);

                    Ok(())
                }),
        );

        graph.add_pass(
            allocator,
            RenderPass::new(allocator)
                .add_color_attachment(
                    AttachmentType::SwapchainImage,
                    RenderAttachmentDesc {
                        load_op: AttachmentLoadOp::Load,
                        ..Default::default()
                    },
                )
                .add_input(RenderPassResource::Image(self.depth_image_handle))
                .set_callback(allocator, |renderer, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.texture_pipeline);
                    cmd_list.bind_index_buffer(&renderer.device, &self.quad_index_buffer);
                    cmd_list.bind_vertex_buffer(&renderer.device, &self.quad_vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.texture_pipeline,
                        0,
                        &[self.texture_bind_group],
                    );
                    cmd_list.draw_offset(&renderer.device, 6, 0, 0);

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
        // TODO: Some of this stuff should be more automated?
        renderer.command_queue.transition_image(
            &renderer.device,
            depth_image,
            ImageUsage::Depth,
            Layout::Undefined,
            Layout::DepthStencilReadOnly,
        )?;
        renderer.device.write_bind_group(&[BindGroupBindInfo {
            group: self.texture_bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(depth_image.bind_info(
                &self.sampler,
                Layout::DepthStencilReadOnly,
                None,
            )),
        }])?;

        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.mesh_pipeline.destroy(&renderer.device);
        self.texture_pipeline.destroy(&renderer.device);
        self.cube_vertex_buffer.destroy(&renderer.device);
        self.cube_index_buffer.destroy(&renderer.device);
        self.ubo_buffer.destroy(&renderer.device);
        self.quad_vertex_buffer.destroy(&renderer.device);
        self.quad_index_buffer.destroy(&renderer.device);
        self.sampler.destroy(&renderer.device);
        Ok(())
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "depth-image",
            ..Default::default()
        },
    )
    .unwrap();
    let mut cinder = Cinder::<DepthImageSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
