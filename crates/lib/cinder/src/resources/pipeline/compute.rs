use super::{get_pipeline_layout, PipelineCommon};
use crate::{device::Device, resources::shader::Shader};
use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

pub fn get_group_count(thread_count: u32, local_size: u32) -> u32 {
    (thread_count + local_size - 1) / local_size
}

pub struct ComputePipelineDescription {
    pub shader: Shader,
}

pub struct ComputePipeline {
    pub common: PipelineCommon,
}

impl ComputePipeline {
    pub fn create(device: &Device, desc: ComputePipelineDescription) -> Result<Self> {
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
                device.pipeline_cache,
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
            common_data,
        };

        Ok(Self { common })
    }
}
