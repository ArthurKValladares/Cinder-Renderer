use super::{get_pipeline_layout, BindGroupData, PipelineCommon};
use crate::device::Device;

use crate::resources::{
    image::{reflect_format_to_vk, Format},
    shader::Shader,
};
use anyhow::Result;
use ash::vk;
use resource_manager::ResourceId;
use std::ffi::CStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphicsPipelineError {
    #[error("shader for handle not in resource pool: {0:?}")]
    ShaderNotInResourcePool(ResourceId<Shader>),
}

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
pub struct DepthBiasInfo {
    pub constant_factor: f32,
    pub slope_factor: f32,
}

#[derive(Debug, Copy, Clone)]
pub enum CullMode {
    Back,
    Front,
    FrontAndBack,
    None,
}

impl Default for CullMode {
    fn default() -> Self {
        Self::None
    }
}

impl From<CullMode> for vk::CullModeFlags {
    fn from(value: CullMode) -> Self {
        match value {
            CullMode::Back => vk::CullModeFlags::BACK,
            CullMode::Front => vk::CullModeFlags::FRONT,
            CullMode::FrontAndBack => vk::CullModeFlags::FRONT_AND_BACK,
            CullMode::None => vk::CullModeFlags::NONE,
        }
    }
}

pub type VertexInputRate = vk::VertexInputRate;
pub type VertexBindingDesc = vk::VertexInputBindingDescription;
pub type VertexAttributeDescription = vk::VertexInputAttributeDescription;

#[derive(Debug, Clone)]
pub struct VertexDescription {
    pub binding_desc: Vec<VertexBindingDesc>,
    pub attribute_desc: Vec<VertexAttributeDescription>,
}

#[derive(Debug, Clone)]
pub struct GraphicsPipelineDescription {
    pub name: Option<String>,
    pub blending: ColorBlendState,
    pub color_format: Option<Format>,
    pub depth_format: Option<Format>,
    pub cull_mode: CullMode,
    pub depth_bias: Option<DepthBiasInfo>,
    pub vertex_desc: Option<VertexDescription>,
}

impl Default for GraphicsPipelineDescription {
    fn default() -> Self {
        Self {
            name: None,
            blending: Default::default(),
            color_format: Some(Format::B8G8R8A8_Unorm),
            depth_format: None,
            cull_mode: Default::default(),
            depth_bias: None,
            vertex_desc: None,
        }
    }
}

pub struct GraphicsPipeline {
    pub common: PipelineCommon,
    pub desc: GraphicsPipelineDescription,
}

impl GraphicsPipeline {
    fn create_raw_pipeline(
        device: &Device,
        vertex_shader: &Shader,
        fragment_shader: Option<&Shader>,
        desc: &GraphicsPipelineDescription,
        pipeline_layout: vk::PipelineLayout,
    ) -> Result<vk::Pipeline> {
        let atttributes = vertex_shader.reflect_data.get_vertex_attributes();
        let binding = 0; // TODO: Support non-zero bindings, need to be done shader-side, probably in the name atm
        if let Some(vertex_desc) = &desc.vertex_desc {
            inner_create_raw_pipeline(
                device,
                vertex_shader,
                fragment_shader,
                desc,
                pipeline_layout,
                &vertex_desc.binding_desc,
                &vertex_desc.attribute_desc,
            )
        } else {
            let vertex_input_binding_descriptions = vec![vk::VertexInputBindingDescription {
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

            inner_create_raw_pipeline(
                device,
                vertex_shader,
                fragment_shader,
                desc,
                pipeline_layout,
                &vertex_input_binding_descriptions,
                &vertex_input_attribute_descriptions,
            )
        }
    }

    pub fn bind_group_data(&self, idx: usize) -> Option<&BindGroupData> {
        self.common.bind_group_data(idx)
    }

    pub(crate) fn create(
        device: &Device,
        vertex_shader: &Shader,
        fragment_shader: Option<&Shader>,
        desc: GraphicsPipelineDescription,
    ) -> Result<Self> {
        //
        // Pipeline stuff, pretty temp
        //
        let default_shader = Shader::default();
        let shaders = [
            vertex_shader,
            if let Some(fragment_shader) = fragment_shader {
                fragment_shader
            } else {
                &default_shader
            },
        ];
        let (pipeline_layout, common_data) = get_pipeline_layout(
            device,
            if fragment_shader.is_some() {
                &shaders
            } else {
                &shaders[0..1]
            },
            &desc.name,
        )?;

        let pipeline = Self::create_raw_pipeline(
            device,
            vertex_shader,
            fragment_shader,
            &desc,
            pipeline_layout,
        )?;

        let common =
            PipelineCommon::new(device, pipeline_layout, pipeline, common_data, &desc.name);

        Ok(GraphicsPipeline { common, desc })
    }

    pub fn recreate(
        &mut self,
        vertex_shader: &Shader,
        fragment_shader: Option<&Shader>,
        device: &Device,
    ) -> Result<vk::Pipeline> {
        let new_pipeline = Self::create_raw_pipeline(
            device,
            vertex_shader,
            fragment_shader,
            &self.desc,
            self.common.pipeline_layout,
        )?;
        let old = self.common.pipeline;
        self.common.pipeline = new_pipeline;
        Ok(old)
    }

    pub fn destroy(&self, device: &Device) {
        self.common.destroy(device);
    }
}

fn inner_create_raw_pipeline(
    device: &Device,
    vertex_shader: &Shader,
    fragment_shader: Option<&Shader>,
    desc: &GraphicsPipelineDescription,
    pipeline_layout: vk::PipelineLayout,
    vertex_input_binding_descriptions: &[vk::VertexInputBindingDescription],
    vertex_input_attribute_descriptions: &[vk::VertexInputAttributeDescription],
) -> Result<vk::Pipeline> {
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
        .cull_mode(desc.cull_mode.into())
        .front_face(vk::FrontFace::CLOCKWISE)
        .line_width(1.0);

    let rasterization_info = if let Some(info) = desc.depth_bias {
        rasterization_info
            .depth_bias_enable(true)
            .depth_bias_constant_factor(info.constant_factor)
            .depth_bias_slope_factor(info.slope_factor)
    } else {
        rasterization_info.depth_bias_enable(false)
    };
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
        if let Some(fragment_shader) = fragment_shader {
            vk::PipelineShaderStageCreateInfo {
                module: fragment_shader.module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            }
        } else {
            Default::default()
        },
    ];

    let color_attachment_formats = if let Some(color_format) = desc.color_format {
        [color_format.into()]
    } else {
        [Default::default()]
    };
    let mut pipeline_rendering_ci = {
        let mut builder = vk::PipelineRenderingCreateInfo::builder();
        if desc.color_format.is_some() {
            builder = builder.color_attachment_formats(&color_attachment_formats);
        }
        if let Some(depth_format) = desc.depth_format {
            builder = builder.depth_attachment_format(depth_format.into());
        }
        builder.build()
    };

    let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::builder()
        .push_next(&mut pipeline_rendering_ci)
        .stages(if fragment_shader.is_some() {
            &shader_stage_create_infos
        } else {
            &shader_stage_create_infos[..1]
        })
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

    Ok(pipeline)
}
