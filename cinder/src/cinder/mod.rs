use crate::{
    context::{
        render_context::{RenderContext, RenderContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
    },
    device::Device,
    instance::Instance,
    profiling::Profiling,
    resoruces::{
        buffer::{Buffer, BufferDescription},
        image::{self, Image, ImageDescription, ImageViewDescription},
        pipeline::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        shader::{Shader, ShaderDescription},
    },
    surface::{Surface, SurfaceData},
    swapchain::Swapchain,
    InitData,
};
use anyhow::Result;
use ash::vk;
use math::{rect::Rect2D, size::Size2D};
use tracing::{span, Level};

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/default_shader_structs.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../gen/egui_shader_structs.rs"
));

pub const RESERVED_DESCRIPTOR_COUNT: u32 = 32;
