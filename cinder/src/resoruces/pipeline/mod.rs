pub mod push_constant;

use self::push_constant::PushConstant;
use super::{
    bind_group::BindGroupLayout,
    image::reflect_format_to_vk,
    shader::{Shader, ShaderStage},
};
use anyhow::Result;
use ash::vk;
use std::{collections::HashMap, ffi::CStr};

#[repr(C)]
#[derive(Debug, Default)]
pub struct ColorBlendState {
    state: vk::PipelineColorBlendAttachmentState,
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

pub struct GraphicsPipelineDescription {
    pub vertex_shader: Shader,
    pub fragment_shader: Shader,
    pub blending: ColorBlendState,
    pub depth_testing_enabled: bool,
    pub backface_culling: bool,
    pub uses_depth: bool,
}

pub struct PipelineCommon {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

pub struct GraphicsPipeline {
    pub common: PipelineCommon,
    // TODO: Think of a better key
    push_constants: HashMap<(ShaderStage, u32), PushConstant>,
    // TODO: Also need a better way to get these
    bind_group_layouts: Vec<BindGroupLayout>,
}

impl GraphicsPipeline {
    pub(crate) fn create(
        device: &ash::Device,
        surface_format: vk::Format,
        pipeline_cache: vk::PipelineCache,
        desc: GraphicsPipelineDescription,
    ) -> Result<Self> {
        //
        // Pipeline stuff, pretty temp
        //
        let push_constants = {
            let mut map = HashMap::new();
            for (idx, pc) in desc.vertex_shader.push_constants()?.into_iter().enumerate() {
                map.insert((ShaderStage::Vertex, idx as u32), pc);
            }
            for (idx, pc) in desc
                .fragment_shader
                .push_constants()?
                .into_iter()
                .enumerate()
            {
                map.insert((ShaderStage::Fragment, idx as u32), pc);
            }
            map
        };
        let bind_group_layouts = {
            let mut data_map = desc.vertex_shader.bind_group_layouts()?;
            for (set, data) in desc.fragment_shader.bind_group_layouts()? {
                let entry = data_map.entry(set).or_insert_with(|| Vec::new());
                entry.extend(data);
            }

            data_map
                .values()
                .map(|layout_data| BindGroupLayout::new(device, &layout_data))
                .collect::<Result<Vec<_>>>()
        }?;
        let set_layouts = unsafe {
            std::mem::transmute::<&[BindGroupLayout], &[vk::DescriptorSetLayout]>(
                &bind_group_layouts,
            )
        };
        let push_constant_ranges = push_constants
            .values()
            .map(|pc| pc.to_raw())
            .collect::<Vec<_>>();
        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges)
            .build();

        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_create_info, None) }?;

        let atttributes = desc.vertex_shader.reflect_data.get_vertex_attributes();
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

        let depth_state_info = if desc.depth_testing_enabled {
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

        // TODO: Will make this better
        let pipeline_rendering_ci = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(std::slice::from_ref(&surface_format));
        let mut pipeline_rendering_ci = if desc.uses_depth {
            pipeline_rendering_ci
                .depth_attachment_format(vk::Format::D32_SFLOAT) // TODO: get from depth image
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
            device.create_graphics_pipelines(pipeline_cache, &[graphic_pipeline_infos], None)
        }
        .map_err(|(_, err)| err)?;
        let pipeline = graphics_pipelines[0];
        for pipeline in graphics_pipelines.iter().skip(1) {
            unsafe {
                device.destroy_pipeline(*pipeline, None);
            }
        }

        Ok(GraphicsPipeline {
            common: PipelineCommon {
                pipeline_layout,
                pipeline,
            },
            push_constants,
            bind_group_layouts,
        })
    }

    pub fn get_push_constant(&self, shader_stage: ShaderStage, idx: u32) -> Option<&PushConstant> {
        self.push_constants.get(&(shader_stage, idx))
    }

    pub fn bind_group_layouts(&self) -> &[BindGroupLayout] {
        &self.bind_group_layouts
    }
}
