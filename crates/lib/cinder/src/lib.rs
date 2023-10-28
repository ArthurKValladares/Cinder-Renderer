use bumpalo::Bump;
use egui_integration::EguiIntegration;
use render_graph::RenderGraph;
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::SdlContext;

pub use renderer::{
    resources::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        pipeline::graphics::GraphicsPipeline,
    },
    Renderer,
};

pub trait App: Sized {
    // TODO: Explicit error type
    fn new(renderer: &mut Renderer, width: u32, height: u32) -> anyhow::Result<Self>;
    fn draw<'a>(&'a mut self, allocator: &'a Bump, graph: &mut RenderGraph<'a>) -> anyhow::Result<()>;
    
    fn resize(&mut self, _width: u32, _height: u32) -> anyhow::Result<()> {
        Ok(())
    }
    fn cleanup(&mut self, _renderer: &mut Renderer) -> anyhow::Result<()>{
        Ok(())
    }
}

pub struct Cinder<A: App> {
    renderer: Renderer,
    allocator: Bump,
    egui: EguiIntegration,
    app: A,
}

impl<A> Cinder<A>
where
    A: App,
{
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let (width, height) = window.drawable_size();
        // TODO: Pull ResourceManager out of renderer
        let mut renderer = Renderer::new(window, width, height)?;
        let allocator = Bump::new();
        let egui = EguiIntegration::new(
            &mut renderer.resource_manager,
            &renderer.device,
            &renderer.swapchain,
        )?;

        let app = A::new(&mut renderer, width, height)?;

        Ok(Self {
            renderer,
            allocator,
            egui,
            app,
        })
    }

    // TODO: Update function

    fn draw(&mut self) -> anyhow::Result<bool> {
        let mut graph = RenderGraph::new(&self.allocator);
        self.app.draw(&self.allocator, &mut graph)?;
        let present_context = graph.run(&self.allocator, &mut self.renderer)?;
        self.egui.run(
            &mut self.renderer.resource_manager,
            &self.renderer.device,
            &present_context.cmd_list,
            present_context.present_rect,
            present_context.swapchain_image,
            |ctx| {
                // TODO: App-defined function
            },
        )?;

        present_context.present(&mut self.renderer)
    }

    fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()> {
        self.app.resize(width, height)?;
        self.renderer.resize(width, height)?;
        self.egui.resize(width, height);
        Ok(())
    }

    pub fn run_game_loop(&mut self, sdl: &mut SdlContext) {
        'running: loop {
            self.allocator.reset();
            self.renderer.start_frame().unwrap();

            for event in sdl.event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => {
                        break 'running;
                    }
                    Event::Window {
                        win_event: sdl2::event::WindowEvent::SizeChanged(width, height),
                        ..
                    } => {
                        self.resize(width as u32, height as u32).unwrap();
                    }
                    _ => {}
                }
            }

            self.draw().unwrap();

            self.renderer.end_frame();
        }
    }
}

impl<A> Drop for Cinder<A> where A: App {
    fn drop(&mut self) {
        self.renderer.device.wait_idle().ok();
        self.app.cleanup(&mut self.renderer).ok();
    }
}