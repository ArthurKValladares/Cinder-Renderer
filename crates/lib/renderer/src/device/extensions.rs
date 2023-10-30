use super::instance::Instance;
use ash::extensions::khr::DynamicRendering;

pub struct DeviceExtensions {
    dynamic_rendering: DynamicRendering,
}

impl DeviceExtensions {
    pub fn new(instance: &Instance, device: &ash::Device) -> Self {
        let dynamic_rendering = DynamicRendering::new(instance.raw(), device);

        Self { dynamic_rendering }
    }

    pub fn dynamic_rendering(&self) -> &DynamicRendering {
        &self.dynamic_rendering
    }
}
