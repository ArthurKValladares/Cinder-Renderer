use super::{get_pipeline_layout, PipelineCommon};
use crate::device::Device;
use crate::resources::bind_group::BindGroup;
use crate::resources::{
    image::{reflect_format_to_vk, Format},
    shader::Shader,
};
use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColorBlendState {
    state: vk::PipelineColorBlendAttachmentState,
}

impl Default for ColorBlendState {
    fn default() -> Self {
        Self::add()
    }
}

impl ColorBlendState {
    pub fn add() -> Self {
        Self {
            state: vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_COLOR)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ZERO)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .build(),
        }
    }

    pub fn pma() -> Self {
        Self {
            state: vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .build(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GraphicsPipelineDescription {
    pub name: Option<&'static str>, // TODO: Probably should have a lifetime
    pub blending: ColorBlendState,
    pub surface_format: Format,
    pub depth_format: Option<Format>,
    pub backface_culling: bool,
}

impl Default for GraphicsPipelineDescription {
    fn default() -> Self {
        Self {
            name: None,
            blending: Default::default(),
            surface_format: Format::B8_G8_R8_A8_Unorm,
            depth_format: Default::default(),
            backface_culling: Default::default(),
        }
    }
}

pub struct GraphicsPipeline {
    pub common: PipelineCommon,
    pub bind_group: Option<BindGroup>,
    pub desc: GraphicsPipelineDescription,
}

impl GraphicsPipeline {
    pub(crate) fn create(
        device: &Device,
        vertex_shader: &Shader,
        fragment_shader: &Shader,
        desc: GraphicsPipelineDescription,
    ) -> Result<Self> {
        //
        // Pipeline stuff, pretty temp
        //
        let (pipeline_layout, common_data) =
            get_pipeline_layout(device, &[vertex_shader, fragment_shader], desc.name)?;

        let atttributes = vertex_shader.reflect_data.get_vertex_attributes();
        let binding = 0; // TODO: Support non-zero bindings
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding,
            stride: atttributes.stride / 8,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = atttributes
            .atts
            .iter()
            .enumerate()
            .map(|(location, att)| vk::VertexInputAttributeDescription {
                location: location as u32,
                binding,
                format: reflect_format_to_vk(att.format, att.low_precision),
                offset: att.offset / 8,
            })
            .collect::<Vec<_>>();
        let vertex_input_state_info = if !vertex_input_attribute_descriptions.is_empty() {
            vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
                .vertex_binding_descriptions(&vertex_input_binding_descriptions)
                .build()
        } else {
            vk::PipelineVertexInputStateCreateInfo::builder().build()
        };

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);
        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(if desc.backface_culling {
                vk::CullModeFlags::BACK
            } else {
                vk::CullModeFlags::NONE
            })
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        let depth_state_info = if desc.depth_format.is_some() {
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::GREATER)
                .build()
        } else {
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::ALWAYS)
                .build()
        };

        let color_blend_attachment_states = [desc.blending.state];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);
        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let shader_entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader.module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: fragment_shader.module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];

        // TODO: Will make this better
        let surface_format = desc.surface_format.into();
        let pipeline_rendering_ci = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(std::slice::from_ref(&surface_format));
        let mut pipeline_rendering_ci = if let Some(depth_format) = desc.depth_format {
            pipeline_rendering_ci
                .depth_attachment_format(depth_format.into())
                .build()
        } else {
            pipeline_rendering_ci.build()
        };
        let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::builder()
            .push_next(&mut pipeline_rendering_ci)
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .build();

        let graphics_pipelines = unsafe {
            device.raw().create_graphics_pipelines(
                device.pipeline_cache,
                &[graphic_pipeline_infos],
                None,
            )
        }
        .map_err(|(_, err)| err)?;
        let pipeline = graphics_pipelines[0];
        for pipeline in graphics_pipelines.iter().skip(1) {
            unsafe {
                device.raw().destroy_pipeline(*pipeline, None);
            }
        }

        if let Some(name) = desc.name {
            device.set_name(vk::ObjectType::PIPELINE, pipeline, name)
        }

        let bind_group = if common_data.bind_group_layouts().is_empty() {
            None
        } else {
            let bind_group = BindGroup::new(
                device,
                common_data.bind_group_layouts(),
                common_data.variable_count,
            )?;

            if let Some(name) = desc.name {
                bind_group.set_name(device, name);
            }

            Some(bind_group)
        };

        let common = PipelineCommon {
            pipeline_layout,
            pipeline,
            common_data,
        };
        if let Some(name) = desc.name {
            common.set_name(device, name);
        }
        Ok(GraphicsPipeline {
            common,
            bind_group,
            desc,
        })
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        self.common.destroy(device);
    }
}
