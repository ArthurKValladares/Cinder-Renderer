mod app;
mod renderer;
mod ui;

use app::App;
use cinder::{
    cinder::MeshVertex,
    resources::{buffer::BufferUsage, memory::MemoryType},
};
use ember::GpuStagingBuffer;
use util::*;
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::WindowBuilder};

pub const WINDOW_HEIGHT: u32 = 2000;
pub const WINDOW_WIDTH: u32 = 2000;

pub struct MeshDraw {
    index_buffer_offset: u32,
    num_indices: usize,
    vertex_buffer_offset: i32,
    image_index: usize,
}

// TODO: verify that all triple buffering stuff is working

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let app = App::new(&event_loop, &window).unwrap();

    // TODO: Revisit this staging buffer idea, it could be good with some tweaks
    let mut staging_buffer = GpuStagingBuffer::new(
        app.renderer.device(),
        BufferUsage::empty().transfer_src(),
        MemoryType::CpuVisible,
    )
    .expect("Could not create GPU staging buffer");

    let mut vertex_buffer_offset = 0;
    let mut index_buffer_offset = 0;
    app.upload_context
        .begin(app.renderer.device(), app.renderer.setup_fence())
        .expect("Could not begin upload context");
    let mesh_draws = app
        .scene
        .meshes
        .iter()
        .map(|mesh| {
            let buffer_region = staging_buffer
                .copy_data(&mesh.indices)
                .expect("could not write to staging buffer");
            let index_buffer_size = size_of_slice(&mesh.indices);
            app.upload_context.copy_buffer(
                app.renderer.device(),
                staging_buffer.buffer(),
                &app.index_buffer,
                buffer_region.offset,
                (index_buffer_offset * std::mem::size_of::<u32>()) as u64,
                index_buffer_size,
            );

            let buffer_region = staging_buffer
                .copy_data(&mesh.vertices)
                .expect("could not write to staging buffer");
            let vertex_buffer_size = size_of_slice(&mesh.vertices);
            app.upload_context.copy_buffer(
                app.renderer.device(),
                staging_buffer.buffer(),
                &app.vertex_buffer,
                buffer_region.offset,
                (vertex_buffer_offset * std::mem::size_of::<MeshVertex>()) as u64,
                vertex_buffer_size,
            );

            let num_indices = mesh.indices.len();
            let image_index = mesh.material_index.unwrap_or(0); //TODO: Actually handle this the right way, or use a white texture

            let ret = MeshDraw {
                index_buffer_offset: index_buffer_offset as u32,
                num_indices,
                vertex_buffer_offset: vertex_buffer_offset as i32,
                image_index,
            };
            index_buffer_offset += mesh.indices.len();
            vertex_buffer_offset += mesh.vertices.len();
            ret
        })
        .collect::<Vec<_>>();
    app.upload_context
        .end(
            app.renderer.device(),
            app.renderer.setup_fence(),
            app.renderer.present_queue(),
            &[],
            &[],
            &[],
        )
        .expect("could not end command context");

    app.run(window, event_loop, mesh_draws);
}
