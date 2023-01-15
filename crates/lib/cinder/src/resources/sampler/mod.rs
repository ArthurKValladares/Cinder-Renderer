use ash::vk;

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
