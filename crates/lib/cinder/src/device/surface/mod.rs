use crate::{device::instance::Instance, resources::image::Format};
use anyhow::Result;
use ash::vk;
use math::size::Size2D;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub struct Surface {
    pub surface_loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn new(window: &winit::window::Window, instance: &Instance) -> Result<Self> {
        let surface_loader = ash::extensions::khr::Surface::new(instance.entry(), instance.raw());
        let surface = unsafe {
            ash_window::create_surface(
                instance.entry(),
                instance.raw(),
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
        }?;

        Ok(Self {
            surface_loader,
            surface,
        })
    }

    pub fn get_data(
        &self,
        p_device: vk::PhysicalDevice,
        backbuffer_resolution: impl Into<Size2D<u32>>,
        vsync: bool,
    ) -> Result<SurfaceData> {
        let surface_formats = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(p_device, self.surface)
        }?;

        let surface_format = surface_formats
            .iter()
            .map(|sfmt| match sfmt.format {
                vk::Format::UNDEFINED => vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8_UNORM,
                    color_space: sfmt.color_space,
                },
                _ => *sfmt,
            })
            .next()
            .expect("Unable to find suitable surface format.");
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(p_device, self.surface)
        }?;

        let desired_image_count = {
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            desired_image_count
        };

        let backbuffer_resolution = backbuffer_resolution.into();
        let surface_resolution = match surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D {
                width: backbuffer_resolution.width(),
                height: backbuffer_resolution.height(),
            },
            _ => surface_capabilities.current_extent,
        };

        let present_modes = unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(p_device, self.surface)
        }?;

        let present_mode_preference = if !vsync {
            vec![vk::PresentModeKHR::FIFO_RELAXED, vk::PresentModeKHR::FIFO]
        } else {
            vec![vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE]
        };
        let present_mode = present_mode_preference
            .into_iter()
            .find(|mode| present_modes.contains(mode))
            .unwrap_or(vk::PresentModeKHR::FIFO);

        Ok(SurfaceData {
            surface_format,
            surface_capabilities,
            surface_resolution,
            present_mode,
            desired_image_count,
        })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.surface_loader.destroy_surface(self.surface, None) }
    }
}

pub struct SurfaceData {
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_resolution: vk::Extent2D,
    pub present_mode: vk::PresentModeKHR,
    pub desired_image_count: u32,
}

impl SurfaceData {
    pub fn size(&self) -> Size2D<u32> {
        Size2D::new(
            self.surface_resolution.width,
            self.surface_resolution.height,
        )
    }

    pub fn format(&self) -> Format {
        self.surface_format.format.into()
    }
}
