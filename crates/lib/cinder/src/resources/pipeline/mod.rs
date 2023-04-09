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

// TODO: Can refactor a bunch of pipeline stuff

#[derive(Debug)]
pub struct PipelineCommonData {
    // TODO: Think of a better key
    push_constants: HashMap<(ShaderStage, u32), PushConstant>,
    // TODO: Also need a better way to get these
    bind_group_layouts: Vec<BindGroupLayout>,
    counts: Vec<u32>,
}

impl PipelineCommonData {
    pub fn bind_group_layouts(&self) -> &[BindGroupLayout] {
        &self.bind_group_layouts
    }

    pub fn descriptor_counts(&self) -> &[u32] {
        &self.counts
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        for mut layout in self.bind_group_layouts.drain(..) {
            layout.destroy(device)
        }
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

    fn set_name(&self, device: &Device, name: &str) {
        device.set_name(
            vk::ObjectType::PIPELINE,
            self.pipeline,
            &format!("{name} [pipeline]"),
        );
        device.set_name(
            vk::ObjectType::PIPELINE_LAYOUT,
            self.pipeline_layout,
            &format!("{name} [pipeline layout]"),
        );
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        self.common_data.destroy(device);
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
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

    let (bind_group_layouts, counts) = {
        let mut data_map: BTreeMap<u32, Vec<BindGroupLayoutData>> = Default::default();
        for shader in shaders {
            for (set, data) in shader.bind_group_layouts(device.descriptor_indexing_properties())? {
                let entry = data_map.entry(set).or_insert_with(Vec::new);
                entry.extend(data);
            }
        }

        let mut bind_group_layouts = Vec::with_capacity(data_map.len());
        let mut counts = Vec::with_capacity(data_map.len());
        for (i, layout_data) in data_map.values().enumerate() {
            let count = layout_data.last().unwrap().count;
            let layout = BindGroupLayout::new(device, layout_data)?;
            if let Some(name) = name {
                layout.set_name(device, &format!("{name} [descriptor set layout {i}]"));
            }
            bind_group_layouts.push(layout);
            counts.push(count);
        }
        (bind_group_layouts, counts)
    };
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
        counts,
    };

    Ok((pipeline_layout, pipeline_common_data))
}
