pub enum PlatformData {
    Windows(()),
    MacOS(()),
}

#[derive(Debug, Clone, Copy)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8Srgb,
}

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

pub struct InitData {
    pub debug_enabled: bool,
    pub profiling_enabled: bool,
    pub platform_data: PlatformData,
    pub backbuffer_resolution: Resolution,
}

pub struct Init {
    resolution: Resolution,
    // TODO: Add a bunch more stuff
}

impl Init {
    pub fn from_data(data: InitData) -> Self {
        Self {
            resolution: data.backbuffer_resolution,
        }
    }
}
