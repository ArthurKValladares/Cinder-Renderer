use super::memory::Memory;
use ash::vk;
use math::size::Size2D;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    R8G8B8A8Unorm,
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::R8G8B8A8Unorm => vk::Format::R8G8B8A8_UNORM,
        }
    }
}

pub struct TextureDescription {
    pub format: Format,
    pub size: Size2D<u32>,
}

pub struct Texture {
    pub raw: vk::Image,
    pub view: vk::ImageView,
    pub memory: Memory,
    pub desc: TextureDescription,
}

impl Texture {
    pub fn dims(&self) -> Size2D<u32> {
        todo!()
    }
    pub fn format(&self) -> Format {
        todo!()
    }
}
