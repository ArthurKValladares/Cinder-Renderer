pub mod push_constant;

use self::push_constant::PushConstant;

use super::{image::Format, render_pass::RenderPass, shader::Shader};
use crate::surface::SurfaceData;
use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

#[derive(Debug)]
pub struct VertexAttributeDesc {
    pub format: Format,
    pub offset: u32,
}

#[derive(Debug)]
pub struct VertexInputStateDesc {
    pub binding: u32,
    pub stride: u32,
    pub attributes: Vec<VertexAttributeDesc>, // TODO: ArrayVec
}

pub struct GraphicsPipelineDescription<'a> {
    pub vertex_shader: Shader,
    pub fragment_shader: Shader,
    pub vertex_state: VertexInputStateDesc,
    pub render_pass: &'a RenderPass,
    pub desc_set_layouts: Vec<vk::DescriptorSetLayout>,
    pub push_constants: Vec<&'a PushConstant>,
    pub depth_testing_enabled: bool,
    pub backface_culling: bool,
}

pub struct PipelineCommon {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

pub struct GraphicsPipeline {
    pub common: PipelineCommon,
}

impl GraphicsPipeline {
    pub(crate) fn create(
        device: &ash::Device,
        surface_data: &SurfaceData,
        desc: GraphicsPipelineDescription,
    ) -> Result<Self> {
        //
        // Pipeline stuff, pretty temp
        //
        let push_constant_ranges = desc
            .push_constants
            .iter()
            .map(|pc| pc.to_raw())
            .collect::<Vec<_>>();
        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&desc.desc_set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_create_info, None) }?;

        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: desc.vertex_state.binding,
            stride: desc.vertex_state.stride,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = desc
            .vertex_state
            .attributes
            .iter()
            .enumerate()
            .map(|(location, att)| vk::VertexInputAttributeDescription {
                location: location as u32,
                binding: desc.vertex_state.binding,
                format: att.format.into(),
                offset: att.offset,
            })
            .collect::<Vec<_>>();
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);

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
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);
        let stencil_state = vk::StencilOpState::builder()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .depth_fail_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::ALWAYS)
            .build();

        let depth_state_info = if desc.depth_testing_enabled {
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
                .front(stencil_state)
                .back(stencil_state)
                .max_depth_bounds(1.0)
        } else {
            vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::ALWAYS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .front(stencil_state)
                .back(stencil_state)
        };
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState::builder()
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
            .build()];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachment_states);
        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let shader_entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: desc.vertex_shader.module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: desc.fragment_shader.module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::builder()
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
            .render_pass(desc.render_pass.render_pass)
            .build();

        // TODO: investigate the error return type here
        let graphics_pipelines = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[graphic_pipeline_infos],
                None,
            )
        }
        .map_err(|(_, err)| err)?;

        let pipeline = graphics_pipelines[0];

        Ok(GraphicsPipeline {
            common: PipelineCommon {
                pipeline_layout,
                pipeline,
            },
        })
    }
}
