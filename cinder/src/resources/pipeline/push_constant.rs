use crate::resources::shader::ShaderStage;
use ash::vk;

#[derive(Debug, Clone, Copy)]
pub struct PushConstant {
    pub stage: ShaderStage,
    pub offset: u32,
    pub size: u32,
}

impl PushConstant {
    pub fn to_raw(&self) -> vk::PushConstantRange {
        vk::PushConstantRange::builder()
            .stage_flags(self.stage.into())
            .offset(self.offset)
            .size(self.size)
            .build()
    }
}
