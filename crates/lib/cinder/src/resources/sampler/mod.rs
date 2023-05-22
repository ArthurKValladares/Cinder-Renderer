use crate::device::Device;
use ash::vk;

#[derive(Debug, Clone, Copy)]
pub enum Filter {
    Linear,
    Nearest,
}

impl Default for Filter {
    fn default() -> Self {
        Self::Linear
    }
}

impl From<Filter> for vk::Filter {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Linear => vk::Filter::LINEAR,
            Filter::Nearest => vk::Filter::NEAREST,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

impl Default for AddressMode {
    fn default() -> Self {
        Self::Repeat
    }
}

impl From<AddressMode> for vk::SamplerAddressMode {
    fn from(value: AddressMode) -> Self {
        match value {
            AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            AddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

pub enum BorderColor {
    Black,
    White,
}

impl Default for BorderColor {
    fn default() -> Self {
        Self::White
    }
}

impl From<BorderColor> for vk::BorderColor {
    fn from(value: BorderColor) -> Self {
        match value {
            BorderColor::Black => vk::BorderColor::INT_OPAQUE_BLACK,
            BorderColor::White => vk::BorderColor::INT_OPAQUE_WHITE,
        }
    }
}

pub enum MipmapMode {
    Linear,
    Nearest,
}

impl Default for MipmapMode {
    fn default() -> Self {
        Self::Nearest
    }
}

impl From<MipmapMode> for vk::SamplerMipmapMode {
    fn from(value: MipmapMode) -> Self {
        match value {
            MipmapMode::Linear => vk::SamplerMipmapMode::LINEAR,
            MipmapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
        }
    }
}

#[derive(Default)]
pub struct SamplerDescription {
    pub name: Option<&'static str>,
    pub filter: Filter,
    pub address_mode: AddressMode,
    pub border_color: BorderColor,
    pub mipmap_mode: MipmapMode,
}

pub struct Sampler {
    pub raw: vk::Sampler,
}

impl Sampler {
    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.raw().destroy_sampler(self.raw, None);
        }
    }
}
