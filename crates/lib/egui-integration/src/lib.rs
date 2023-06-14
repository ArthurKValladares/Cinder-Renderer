pub mod helpers;
mod sdl;

use anyhow::Result;
use cinder::{
    command_queue::{
        AttachmentLoadOp, CommandList, CommandQueue, RenderAttachment, RenderAttachmentDesc,
    },
    device::Device,
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, Layout},
        pipeline::graphics::{
            ColorBlendState, GraphicsPipeline, GraphicsPipelineDescription,
            VertexAttributeDescription, VertexBindingDesc, VertexDescription, VertexInputRate,
        },
        sampler::{AddressMode, Sampler, SamplerDescription},
        ResourceManager,
    },
    swapchain::{Swapchain, SwapchainImage},
    util::MemoryMappablePointer,
    ResourceId,
};
use core::panic;
pub use egui;
use egui::{
    epaint::{ImageDelta, Primitive},
    ClippedPrimitive, ImageData, Mesh, TextureId, TexturesDelta,
};
use math::{point::Point2D, rect::Rect2D, size::Size2D};
use sdl::{EguiSdl, EventResponse};
use sdl2::{event::Event, video::Window};
use std::collections::HashMap;

pub(crate) const DEFAULT_PPP: f32 = 3.0;

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 4;
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 2;

pub struct EguiIntegration {
    egui_context: egui::Context,
    egui_sdl: EguiSdl,
    pipeline: ResourceId<GraphicsPipeline>,
    bind_group: BindGroup,
    sampler: ResourceId<Sampler>,
    image_map: HashMap<TextureId, ResourceId<Image>>,
    vertex_buffers: Vec<ResourceId<Buffer>>,
    index_buffers: Vec<ResourceId<Buffer>>,
}

impl EguiIntegration {
    pub fn new(
        resource_manager: &mut ResourceManager,
        device: &Device,
        swapchain: &Swapchain,
    ) -> Result<Self> {
        let egui_context = egui::Context::default();
        let mut egui_sdl = EguiSdl::new();
        egui_context.set_visuals(egui::Visuals::light());
        egui_context.set_pixels_per_point(DEFAULT_PPP);
        egui_sdl.set_pixels_per_point(DEFAULT_PPP);

        let vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/egui.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/egui.frag.spv"),
            Default::default(),
        )?;
        let pipeline = device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                blending: ColorBlendState::pma(),
                color_format: Some(device.surface_data().format()),
                vertex_desc: Some(VertexDescription {
                    binding_desc: vec![VertexBindingDesc {
                        binding: 0,
                        stride: std::mem::size_of::<egui::epaint::Vertex>() as u32,
                        input_rate: VertexInputRate::VERTEX,
                    }],
                    attribute_desc: vec![
                        VertexAttributeDescription {
                            location: 0,
                            binding: 0,
                            format: Format::R32G32_SFloat.into(),
                            offset: 0,
                        },
                        VertexAttributeDescription {
                            location: 1,
                            binding: 0,
                            format: Format::R32G32_SFloat.into(),
                            offset: util::offset_of!(egui::epaint::Vertex, uv) as u32,
                        },
                        VertexAttributeDescription {
                            location: 2,
                            binding: 0,
                            format: Format::R8G8B8A8_Srgb.into(),
                            offset: util::offset_of!(egui::epaint::Vertex, color) as u32,
                        },
                    ],
                }),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(device, pipeline.bind_group_data(0).unwrap())?;
        let pipeline = resource_manager.insert_graphics_pipeline(pipeline);
        vertex_shader.destroy(device);
        fragment_shader.destroy(device);

        let sampler =
            resource_manager.insert_sampler(device.create_sampler(SamplerDescription {
                address_mode: AddressMode::ClampToEdge,
                ..Default::default()
            })?);

        let (vertex_buffers, index_buffers) = {
            let len = swapchain.num_images();
            let mut vertex_buffers = Vec::with_capacity(len);
            let mut index_buffers = Vec::with_capacity(len);
            for _ in 0..len {
                let vertex_buffer = resource_manager.insert_buffer(device.create_buffer(
                    VERTEX_BUFFER_SIZE,
                    BufferDescription {
                        usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                )?);
                vertex_buffers.push(vertex_buffer);

                let index_buffer = resource_manager.insert_buffer(device.create_buffer(
                    INDEX_BUFFER_SIZE,
                    BufferDescription {
                        usage: BufferUsage::INDEX,
                        ..Default::default()
                    },
                )?);
                index_buffers.push(index_buffer);
            }
            (vertex_buffers, index_buffers)
        };

        Ok(Self {
            egui_context,
            egui_sdl,
            sampler,
            pipeline,
            bind_group,
            image_map: Default::default(),
            vertex_buffers,
            index_buffers,
        })
    }

    pub fn context(&self) -> &egui::Context {
        &self.egui_context
    }

    pub fn on_event(&mut self, event: &Event) -> EventResponse {
        self.egui_sdl.on_event(&self.egui_context, event)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        resource_manager: &mut ResourceManager,
        device: &Device,
        window: &Window,
        command_list: &CommandList,
        render_area: Rect2D<i32, u32>,
        swapchain_image: SwapchainImage,
        f: impl FnOnce(&egui::Context),
    ) -> Result<()> {
        let raw_input = self.egui_sdl.take_egui_input(window);

        // TODO: Hook up repaint_after
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            repaint_after: _,
        } = self.egui_context.run(raw_input, f);

        let clipped_primitives = self.egui_context.tessellate(shapes);

        self.egui_sdl
            .handle_platform_output(window, &self.egui_context, platform_output);

        self.set_textures(resource_manager, device, command_list, &textures_delta)?;

        self.paint(
            resource_manager,
            device,
            command_list,
            render_area,
            swapchain_image,
            self.egui_context.pixels_per_point(),
            &clipped_primitives,
        )?;

        self.free_textures(textures_delta);

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        resource_manager: &ResourceManager,
        device: &Device,
        command_list: &CommandList,
        render_area: Rect2D<i32, u32>,
        swapchain_image: SwapchainImage,
        pixels_per_point: f32,
        clipped_primitives: &[ClippedPrimitive],
    ) -> Result<()> {
        let present_index = swapchain_image.index();
        let vertex_buffer = resource_manager
            .buffers
            .get(self.vertex_buffers[present_index as usize])
            .unwrap();
        let index_buffer = resource_manager
            .buffers
            .get(self.index_buffers[present_index as usize])
            .unwrap();
        let pipeline = resource_manager
            .graphics_pipelines
            .get(self.pipeline)
            .unwrap();

        let mut vertex_buffer_ptr = vertex_buffer.ptr().unwrap();
        let mut index_buffer_ptr = index_buffer.ptr().unwrap();

        let mut vertex_base = 0;
        let mut index_base = 0;

        let size = render_area.size();
        command_list.begin_rendering(
            device,
            render_area,
            &[RenderAttachment::color(
                swapchain_image,
                RenderAttachmentDesc {
                    load_op: AttachmentLoadOp::Load,
                    ..Default::default()
                },
            )],
            None,
        );
        command_list.bind_graphics_pipeline(device, pipeline);
        command_list.bind_vertex_buffer(device, vertex_buffer);
        command_list.bind_index_buffer(device, index_buffer);
        command_list.bind_viewport(
            device,
            Rect2D::from_width_height(size.width(), size.height()),
            false,
        );
        command_list.set_vertex_bytes(
            device,
            pipeline,
            &[
                size.width() as f32 / pixels_per_point,
                size.height() as f32 / pixels_per_point,
            ],
            0,
        )?;
        for egui::ClippedPrimitive {
            clip_rect,
            primitive,
        } in clipped_primitives
        {
            {
                let min = {
                    let min = clip_rect.min;

                    egui::Pos2 {
                        x: f32::clamp(min.x * pixels_per_point, 0.0, size.width() as f32),
                        y: f32::clamp(min.y * pixels_per_point, 0.0, size.height() as f32),
                    }
                };
                let max = {
                    let max = clip_rect.max;
                    egui::Pos2 {
                        x: f32::clamp(max.x * pixels_per_point, min.x, size.width() as f32),
                        y: f32::clamp(max.y * pixels_per_point, min.y, size.height() as f32),
                    }
                };
                command_list.bind_scissor(
                    device,
                    Rect2D::from_offset_and_size(
                        Point2D::new(min.x.round() as i32, min.y.round() as i32),
                        Size2D::new(
                            (max.x.round() - min.x) as u32,
                            (max.y.round() - min.y) as u32,
                        ),
                    ),
                );
            }

            match primitive {
                Primitive::Mesh(mesh) => {
                    self.paint_mesh(
                        resource_manager,
                        device,
                        command_list,
                        present_index,
                        mesh,
                        &mut vertex_buffer_ptr,
                        &mut vertex_base,
                        &mut index_buffer_ptr,
                        &mut index_base,
                    )?;
                }
                Primitive::Callback(_) => {
                    println!("Egui callback primitives not supported");
                }
            }
        }
        command_list.end_rendering(device);

        Ok(())
    }

    fn paint_mesh(
        &mut self,
        resource_manager: &ResourceManager,
        device: &Device,
        command_list: &CommandList,
        present_index: u32,
        mesh: &Mesh,
        vertex_buffer_ptr: &mut MemoryMappablePointer,
        vertex_base: &mut i32,
        index_buffer_ptr: &mut MemoryMappablePointer,
        index_base: &mut u32,
    ) -> Result<()> {
        let vertices = &mesh.vertices;
        let vertex_copy_size = std::mem::size_of_val(&vertices[0]) * vertices.len();

        let indices = &mesh.indices;
        let index_copy_size = std::mem::size_of_val(&indices[0]) * indices.len();

        vertex_buffer_ptr.copy_from(vertices, vertex_copy_size);
        index_buffer_ptr.copy_from(indices, index_copy_size);

        let vertex_buffer_ptr_next = vertex_buffer_ptr.add(vertex_copy_size);
        let index_buffer_ptr_next = index_buffer_ptr.add(index_copy_size);

        let vertex_buffer = resource_manager
            .buffers
            .get(self.vertex_buffers[present_index as usize])
            .unwrap();
        let index_buffer = resource_manager
            .buffers
            .get(self.index_buffers[present_index as usize])
            .unwrap();
        if vertex_buffer_ptr_next >= vertex_buffer.end_ptr().unwrap()
            || index_buffer_ptr_next >= index_buffer.end_ptr().unwrap()
        {
            panic!("egui out of memory");
        }

        vertex_buffer_ptr.copy_from(vertices, vertex_copy_size);
        index_buffer_ptr.copy_from(indices, index_copy_size);

        *vertex_buffer_ptr = vertex_buffer_ptr_next;
        *index_buffer_ptr = index_buffer_ptr_next;

        let pipeline = resource_manager
            .graphics_pipelines
            .get(self.pipeline)
            .unwrap();
        command_list.bind_descriptor_sets(device, pipeline, 0, &[self.bind_group]);

        let index = match mesh.texture_id {
            TextureId::Managed(index) => index as usize,
            TextureId::User(_) => unimplemented!(),
        };

        command_list
            .set_fragment_bytes(device, pipeline, &index, 0)
            .unwrap();

        command_list.draw_offset(device, indices.len() as u32, *index_base, *vertex_base);

        *vertex_base += vertices.len() as i32;
        *index_base += indices.len() as u32;

        Ok(())
    }

    fn set_image_helper(
        &mut self,
        resource_manager: &mut ResourceManager,
        device: &Device,
        command_list: &CommandList,
        id: &TextureId,
        size: Size2D<u32>,
        data: &[egui::Color32],
    ) -> Result<()> {
        let (image, buffer) = device.create_image_with_data(
            size,
            util::typed_to_bytes(data),
            command_list,
            Default::default(),
        )?;
        resource_manager.delete_buffer_raw(buffer, device.current_frame_in_flight());

        let index = match id {
            TextureId::Managed(index) => *index,
            TextureId::User(_) => unimplemented!(),
        };

        let _pipeline = resource_manager
            .graphics_pipelines
            .get(self.pipeline)
            .unwrap();
        let sampler = resource_manager.samplers.get(self.sampler).unwrap();
        device.write_bind_group(&[BindGroupBindInfo {
            group: self.bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(image.bind_info(
                sampler,
                Layout::ShaderReadOnly,
                Some(index as u32),
            )),
        }])?;

        let image_handle = resource_manager.insert_image(image);
        self.image_map.insert(*id, image_handle);

        Ok(())
    }

    fn set_image(
        &mut self,
        resource_manager: &mut ResourceManager,
        device: &Device,
        command_list: &CommandList,
        id: &TextureId,
        delta: &ImageDelta,
    ) -> Result<()> {
        match &delta.image {
            ImageData::Color(color_data) => self.set_image_helper(
                resource_manager,
                device,
                command_list,
                id,
                Size2D::new(color_data.size[0] as u32, color_data.size[1] as u32),
                &color_data.pixels,
            ),
            ImageData::Font(font_data) => {
                let data = font_data.srgba_pixels(Some(1.0)).collect::<Vec<_>>();

                self.set_image_helper(
                    resource_manager,
                    device,
                    command_list,
                    id,
                    Size2D::new(font_data.width() as u32, font_data.height() as u32),
                    &data,
                )
            }
        }
    }

    fn set_textures(
        &mut self,
        resource_manager: &mut ResourceManager,
        device: &Device,
        command_list: &CommandList,
        textures_delta: &TexturesDelta,
    ) -> Result<()> {
        for (id, delta) in &textures_delta.set {
            self.set_image(resource_manager, device, command_list, id, delta)?;
        }
        Ok(())
    }

    fn free_textures(&mut self, _textures_delta: TexturesDelta) {}

    pub fn set_pixels_per_point(&mut self, ppp: f32) {
        self.egui_context.set_pixels_per_point(ppp);
        self.egui_sdl.set_pixels_per_point(ppp);
    }
}
