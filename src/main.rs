use cinder::{
    BackbufferRatio, Cinder, ColorClear, InitData, PassId, PlatformData, Resolution, TextureFormat,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    const WINDOW_HEIGHT: u32 = 1000;
    const WINDOW_WIDTH: u32 = 1000;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let init_data = InitData {
        debug_enabled: true,
        profiling_enabled: true,
        platform_data: PlatformData::Windows(()),
        backbuffer_resolution: Resolution {
            format: TextureFormat::Rgba8Srgb,
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        },
    };
    let mut cinder = Cinder::init(init_data).expect("Could not create cinder instance");

    let pass_id = PassId(0);
    cinder.set_pass_color_clear(pass_id, ColorClear::Value([255, 0, 0, 0]));
    cinder.set_pass_rect_relative_backbufer(pass_id, 0, 0, BackbufferRatio::Equal);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let frame_index = cinder.frame();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
