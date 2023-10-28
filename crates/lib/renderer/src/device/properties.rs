use ash::vk;

pub struct DeviceProperties {
    p_device_properties: vk::PhysicalDeviceProperties,
    p_device_properties2: vk::PhysicalDeviceProperties2,
    p_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    p_device_descriptor_indexing_properties: vk::PhysicalDeviceDescriptorIndexingProperties,
}

impl DeviceProperties {
    pub fn new(
        instance: &ash::Instance,
        p_device: vk::PhysicalDevice,
        p_device_properties: vk::PhysicalDeviceProperties,
    ) -> Self {
        let mut p_device_descriptor_indexing_properties =
            ash::vk::PhysicalDeviceDescriptorIndexingProperties::builder().build();
        let mut p_device_properties2 = ash::vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut p_device_descriptor_indexing_properties)
            .build();
        unsafe { instance.get_physical_device_properties2(p_device, &mut p_device_properties2) };
        let p_device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(p_device) };

        Self {
            p_device_properties,
            p_device_properties2,
            p_device_memory_properties,
            p_device_descriptor_indexing_properties,
        }
    }

    pub fn properties(&self) -> vk::PhysicalDeviceProperties {
        self.p_device_properties
    }

    pub fn properties2(&self) -> vk::PhysicalDeviceProperties2 {
        self.p_device_properties2
    }

    pub fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.p_device_memory_properties
    }

    pub fn descriptor_indexing_properties(&self) -> vk::PhysicalDeviceDescriptorIndexingProperties {
        self.p_device_descriptor_indexing_properties
    }
}
