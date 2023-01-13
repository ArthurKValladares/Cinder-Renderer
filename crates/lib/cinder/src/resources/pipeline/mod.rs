use crate::{
    device::Device,
    resources::{
        bind_group::{BindGroupLayout, BindGroupLayoutData},
        pipeline::push_constant::PushConstant,
        shader::{Shader, ShaderStage},
    },
};
use anyhow::Result;
use ash::vk;
use std::collections::{BTreeMap, HashMap};

pub mod compute;
pub mod graphics;
pub mod push_constant;

pub struct PipelineCommonData {
    // TODO: Think of a better key
    push_constants: HashMap<(ShaderStage, u32), PushConstant>,
    // TODO: Also need a better way to get these
    bind_group_layouts: Vec<BindGroupLayout>,
    variable_count: bool,
}

impl PipelineCommonData {
    pub fn bind_group_layouts(&self) -> &[BindGroupLayout] {
        &self.bind_group_layouts
    }
}

pub struct PipelineCommon {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub common_data: PipelineCommonData,
}

impl PipelineCommon {
    pub fn get_push_constant(&self, shader_stage: ShaderStage, idx: u32) -> Option<&PushConstant> {
        self.common_data.push_constants.get(&(shader_stage, idx))
    }

    pub fn bind_group_layouts(&self) -> &[BindGroupLayout] {
        self.common_data.bind_group_layouts()
    }
}

pub fn get_pipeline_layout(
    device: &Device,
    shaders: &[&Shader],
) -> Result<(vk::PipelineLayout, PipelineCommonData)> {
    let push_constants = {
        let mut map = HashMap::new();
        for shader in shaders {
            for (idx, pc) in shader.push_constants()?.into_iter().enumerate() {
                map.insert((shader.stage(), idx as u32), pc);
            }
        }
        map
    };
    // TODO: figure out variable_count situation
    let mut variable_count = false;
    let bind_group_layouts = {
        let mut data_map: BTreeMap<u32, Vec<BindGroupLayoutData>> = Default::default();
        for shader in shaders {
            for (set, data) in shader.bind_group_layouts()? {
                let entry = data_map.entry(set).or_insert_with(Vec::new);
                entry.extend(data);
            }
        }
        data_map
            .values()
            .map(|layout_data| {
                variable_count |= layout_data
                    .last()
                    .map_or(false, |data| data.count.is_none());
                BindGroupLayout::new(device, layout_data)
            })
            .collect::<Result<Vec<_>>>()
    }?;
    let set_layouts = unsafe {
        std::mem::transmute::<&[BindGroupLayout], &[vk::DescriptorSetLayout]>(&bind_group_layouts)
    };
    let push_constant_ranges = push_constants
        .values()
        .map(|pc| pc.to_raw())
        .collect::<Vec<_>>();
    let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(set_layouts)
        .push_constant_ranges(&push_constant_ranges)
        .build();

    let pipeline_layout = unsafe {
        device
            .raw()
            .create_pipeline_layout(&layout_create_info, None)
    }?;

    let pipeline_common_data = PipelineCommonData {
        push_constants,
        bind_group_layouts,
        variable_count,
    };

    Ok((pipeline_layout, pipeline_common_data))
}
