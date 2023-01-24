use ash::vk;

#[derive(Default)]
pub struct SamplerDescription {
    pub name: Option<&'static str>,
}

pub struct Sampler {
    pub raw: vk::Sampler,
}

impl Sampler {
    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.raw, None);
        }
    }
}
