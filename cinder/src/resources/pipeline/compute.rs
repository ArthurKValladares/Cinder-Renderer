use super::{get_pipeline_layout, PipelineCache, PipelineCommon, PipelineCommonData};
use crate::{device::Device, resources::shader::Shader};
use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

pub struct ComputePipelineDescription {
    pub shader: Shader,
}

pub struct ComputePipeline {
    pub common: PipelineCommon,
    common_data: PipelineCommonData,
}

impl ComputePipeline {
    pub fn new(
        device: &Device,
        pipeline_cache: Option<PipelineCache>,
        desc: ComputePipelineDescription,
    ) -> Result<Self> {
        let (pipeline_layout, common_data) = get_pipeline_layout(device, &[&desc.shader])?;

        let shader_entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let stage = vk::PipelineShaderStageCreateInfo {
            module: desc.shader.module,
            p_name: shader_entry_name.as_ptr(),
            stage: vk::ShaderStageFlags::COMPUTE,
            // TODO: Specialization
            ..Default::default()
        };

        let ci = vk::ComputePipelineCreateInfo::builder()
            .stage(stage)
            .layout(pipeline_layout)
            .build();

        let compute_pipelines = unsafe {
            device.raw().create_compute_pipelines(
                pipeline_cache.map_or_else(|| vk::PipelineCache::null(), |cache| cache.0),
                std::slice::from_ref(&ci),
                None,
            )
        }
        .map_err(|(_, err)| err)?;
        let pipeline = compute_pipelines[0];
        for pipeline in compute_pipelines.iter().skip(1) {
            unsafe {
                device.raw().destroy_pipeline(*pipeline, None);
            }
        }

        let common = PipelineCommon {
            pipeline_layout,
            pipeline,
        };

        Ok(Self {
            common,
            common_data,
        })
    }
}
