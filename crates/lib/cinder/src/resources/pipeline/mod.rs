pub mod compute;
pub mod graphics;
pub mod push_constant;

use super::bind_group::{BindGroupData, BindGroupMap, BindGroupSet};
use crate::{
    device::Device,
    resources::{
        bind_group::{BindGroupBindingData, BindGroupLayout},
        pipeline::push_constant::PushConstant,
        shader::{Shader, ShaderStage},
    },
};
use anyhow::Result;
use ash::vk;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("invalid push constant")]
    InvalidPushConstant,
    #[error("invalid pipeline handle")]
    InvalidPipelineHandle,
    #[error("no bound pipeline")]
    NoBoundPipeline,
}

#[derive(Debug, Default)]
pub struct PipelineCommonData {
    // TODO: Think of a better key
    push_constants: HashMap<(ShaderStage, u32), PushConstant>,
    bind_group_map: BindGroupMap,
}

impl PipelineCommonData {
    pub fn destroy(&self, device: &ash::Device) {
        self.bind_group_map.destroy(device);
    }

    pub fn bind_group_data(&self, idx: usize) -> Option<&BindGroupData> {
        self.bind_group_map.map.get(&idx)
    }
}

pub struct PipelineCommon {
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    common_data: PipelineCommonData,
}

impl PipelineCommon {
    pub fn new(
        device: &Device,
        pipeline_layout: vk::PipelineLayout,
        pipeline: vk::Pipeline,
        common_data: PipelineCommonData,
        name: Option<&str>,
    ) -> Self {
        let ret = Self {
            pipeline_layout,
            pipeline,
            common_data,
        };
        if let Some(name) = name {
            device.set_name(
                vk::ObjectType::PIPELINE,
                pipeline,
                &format!("{name} [Pipeline]"),
            );
            device.set_name(
                vk::ObjectType::PIPELINE_LAYOUT,
                pipeline_layout,
                &format!("{name} [Pipeline Layout]"),
            );
        }
        ret
    }

    pub fn bind_group_data(&self, idx: usize) -> Option<&BindGroupData> {
        self.common_data.bind_group_data(idx)
    }

    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }

    pub fn get_push_constant(&self, shader_stage: ShaderStage, idx: u32) -> Option<&PushConstant> {
        self.common_data.push_constants.get(&(shader_stage, idx))
    }

    pub fn destroy(&self, device: &Device) {
        self.common_data.destroy(device.raw());
        unsafe {
            device.raw().destroy_pipeline(self.pipeline, None);
            device
                .raw()
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

pub fn get_pipeline_layout(
    device: &Device,
    shaders: &[&Shader],
    name: Option<&'static str>,
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

    let bind_group_map = {
        let mut data_map: BTreeMap<BindGroupSet, Vec<BindGroupBindingData>> = Default::default();
        for shader in shaders {
            for (set, data) in
                shader.bind_group_descriptions(device.descriptor_indexing_properties())?
            {
                let entry = data_map.entry(set).or_insert_with(Vec::new);
                entry.extend(data);
            }
        }

        let mut bind_group_map = BindGroupMap::default();
        for (i, layout_data) in data_map.values().enumerate() {
            let count = layout_data.last().unwrap().count;
            let layout = BindGroupLayout::new(device, layout_data)?;
            if let Some(name) = name {
                layout.set_name(device, &format!("{name} [Descriptor Set Layout {i}]"));
            }
            bind_group_map
                .map
                .insert(i, BindGroupData { count, layout });
        }
        bind_group_map
    };

    let set_layouts = bind_group_map
        .map
        .values()
        .map(|data| data.layout.0)
        .collect::<Vec<_>>();
    let push_constant_ranges = push_constants
        .values()
        .map(|pc| pc.to_raw())
        .collect::<Vec<_>>();
    let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constant_ranges)
        .build();

    let pipeline_layout = unsafe {
        device
            .raw()
            .create_pipeline_layout(&layout_create_info, None)
    }?;

    let pipeline_common_data = PipelineCommonData {
        push_constants,
        bind_group_map,
    };

    Ok((pipeline_layout, pipeline_common_data))
}
